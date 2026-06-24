use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};

const REPO: &str = "Stasshe/saya";

pub fn run() -> Result<()> {
    let current_exe = std::env::current_exe().context("detecting current executable path")?;
    let version = resolve_latest_version()?;
    let target = release_target()?;
    let archive = format!("saya-{version}-{target}.tar.gz");
    let url = format!("https://github.com/{REPO}/releases/download/{version}/{archive}");
    let checksum_url = format!("{url}.sha256");
    let temp_dir = TempDir::create()?;

    println!("downloading {url}");
    download(&url, &temp_dir.path.join(&archive))?;

    println!("downloading {checksum_url}");
    download(
        &checksum_url,
        &temp_dir.path.join(format!("{archive}.sha256")),
    )?;
    verify_checksum(&temp_dir.path, &archive)?;

    ensure_safe_archive(&temp_dir.path.join(&archive))?;
    extract_archive(&temp_dir.path.join(&archive), &temp_dir.path)?;

    let bin_path = temp_dir.path.join("saya");
    ensure_regular_file(&bin_path)?;
    install_binary(&bin_path, &current_exe)?;

    println!("saya {version} installed to {}", current_exe.display());
    Ok(())
}

fn resolve_latest_version() -> Result<String> {
    let output = Command::new("/usr/bin/curl")
        .args([
            "-fsSLI",
            "-o",
            "/dev/null",
            "-w",
            "%{url_effective}",
            &format!("https://github.com/{REPO}/releases/latest"),
        ])
        .output()
        .context("running curl to resolve latest release")?;
    if !output.status.success() {
        return Err(command_error("curl latest release lookup failed", &output));
    }

    let url = String::from_utf8_lossy(&output.stdout);
    let tag = latest_tag_from_url(url.trim())?;
    Ok(tag.to_string())
}

fn latest_tag_from_url(url: &str) -> Result<&str> {
    let trimmed = url.trim_end_matches('/');
    let tag = trimmed.rsplit('/').next().unwrap_or_default();
    if tag.is_empty() || tag == "latest" {
        bail!("could not resolve latest release tag");
    }
    Ok(tag)
}

fn release_target() -> Result<&'static str> {
    if std::env::consts::OS != "linux" {
        bail!("saya only supports Linux (got: {})", std::env::consts::OS);
    }

    match std::env::consts::ARCH {
        "x86_64" => Ok("x86_64-unknown-linux-musl"),
        "aarch64" => Ok("aarch64-unknown-linux-musl"),
        arch => bail!("unsupported architecture: {arch}"),
    }
}

fn download(url: &str, path: &Path) -> Result<()> {
    let output = Command::new("/usr/bin/curl")
        .args(["-fsSL", url, "-o"])
        .arg(path)
        .output()
        .context("running curl download")?;
    if !output.status.success() {
        return Err(command_error("curl download failed", &output));
    }
    Ok(())
}

fn verify_checksum(dir: &Path, archive: &str) -> Result<()> {
    let output = Command::new("/usr/bin/sha256sum")
        .arg("-c")
        .arg(format!("{archive}.sha256"))
        .current_dir(dir)
        .output()
        .context("running sha256sum")?;
    if !output.status.success() {
        return Err(command_error("sha256sum verification failed", &output));
    }
    Ok(())
}

fn ensure_safe_archive(archive_path: &Path) -> Result<()> {
    let output = Command::new("/usr/bin/tar")
        .arg("-tzf")
        .arg(archive_path)
        .output()
        .context("listing archive contents")?;
    if !output.status.success() {
        return Err(command_error("tar listing failed", &output));
    }

    let listing = String::from_utf8_lossy(&output.stdout);
    for entry in listing.lines() {
        if unsafe_archive_entry(entry) {
            bail!("downloaded archive contains unsafe path: {entry}");
        }
    }
    Ok(())
}

fn unsafe_archive_entry(entry: &str) -> bool {
    if entry.is_empty() || entry.starts_with('/') || entry == ".." || entry.ends_with("/..") {
        return true;
    }
    entry.split('/').any(|part| part == "..")
}

fn extract_archive(archive_path: &Path, dir: &Path) -> Result<()> {
    let output = Command::new("/usr/bin/tar")
        .arg("-xzf")
        .arg(archive_path)
        .arg("-C")
        .arg(dir)
        .output()
        .context("extracting archive")?;
    if !output.status.success() {
        return Err(command_error("tar extraction failed", &output));
    }
    Ok(())
}

fn ensure_regular_file(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path).context("checking downloaded saya binary")?;
    let file_type = metadata.file_type();
    if !file_type.is_file() || file_type.is_symlink() {
        bail!("downloaded archive must contain a regular ./saya binary");
    }
    Ok(())
}

fn install_binary(source: &Path, destination: &Path) -> Result<()> {
    let direct = Command::new("/usr/bin/install")
        .arg("-m")
        .arg("755")
        .arg(source)
        .arg(destination)
        .output()
        .context("running install")?;
    if direct.status.success() {
        return Ok(());
    }

    let elevated = Command::new("/usr/bin/sudo")
        .arg("/usr/bin/install")
        .arg("-m")
        .arg("755")
        .arg(source)
        .arg(destination)
        .status()
        .context("running sudo install")?;
    if !elevated.success() {
        bail!("sudo install failed with {elevated}");
    }
    Ok(())
}

fn command_error(message: &str, output: &std::process::Output) -> anyhow::Error {
    let stderr = String::from_utf8_lossy(&output.stderr);
    anyhow!("{message} with {}: {}", output.status, stderr.trim())
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn create() -> Result<Self> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("reading system time")?
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("saya-self-update-{}-{now}", std::process::id()));
        fs::create_dir_all(&path).context("creating temporary directory")?;
        Ok(Self { path })
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_latest_tag_from_url() {
        assert_eq!(
            latest_tag_from_url("https://github.com/Stasshe/saya/releases/tag/v0.5.0").unwrap(),
            "v0.5.0"
        );
    }

    #[test]
    fn rejects_unresolved_latest_tag() {
        assert!(latest_tag_from_url("https://github.com/Stasshe/saya/releases/latest").is_err());
    }

    #[test]
    fn detects_unsafe_archive_entries() {
        assert!(unsafe_archive_entry("/saya"));
        assert!(unsafe_archive_entry("../saya"));
        assert!(unsafe_archive_entry("dir/../saya"));
        assert!(unsafe_archive_entry("dir/.."));
        assert!(!unsafe_archive_entry("saya"));
        assert!(!unsafe_archive_entry("dir/saya"));
    }
}
