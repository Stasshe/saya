use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

const SHIM_NAMES: &[&str] = &["apt", "apt-get", "pacman"];
const SHIM_DIR: &str = "/usr/local/bin";
const REAL_BIN_DIR: &str = "/usr/bin";

pub struct ShimHealth {
    pub name: &'static str,
    pub symlink_ok: bool,
    pub real_binary_exists: bool,
}

pub struct DoctorReport {
    pub shims: Vec<ShimHealth>,
    pub path_local_bin_first: bool,
}

impl DoctorReport {
    pub fn all_ok(&self) -> bool {
        self.path_local_bin_first
            && self
                .shims
                .iter()
                .all(|s| s.symlink_ok && s.real_binary_exists)
    }
}

/// Idempotently points `/usr/local/bin/{apt,apt-get,pacman}` at the saya
/// binary. Refuses to clobber a path that isn't already a saya symlink.
pub fn enable() -> Result<()> {
    let exe = current_exe()?;
    for name in SHIM_NAMES {
        let link_path = Path::new(SHIM_DIR).join(name);
        if is_saya_symlink(&link_path, &exe)? {
            continue;
        }
        if link_path.exists() || link_path.symlink_metadata().is_ok() {
            bail!(
                "{} exists and is not a saya symlink; remove it manually first",
                link_path.display()
            );
        }
        symlink(&exe, &link_path)
            .with_context(|| format!("creating symlink {}", link_path.display()))?;
        println!("linked {} -> {}", link_path.display(), exe.display());
    }
    Ok(())
}

/// Removes shim symlinks, but only the ones that point at the saya binary.
pub fn disable() -> Result<()> {
    let exe = current_exe()?;
    for name in SHIM_NAMES {
        let link_path = Path::new(SHIM_DIR).join(name);
        if is_saya_symlink(&link_path, &exe)? {
            fs::remove_file(&link_path)
                .with_context(|| format!("removing symlink {}", link_path.display()))?;
            println!("unlinked {}", link_path.display());
        }
    }
    Ok(())
}

pub fn doctor() -> Result<DoctorReport> {
    let exe = current_exe()?;
    let mut shims = Vec::new();
    for name in SHIM_NAMES {
        let link_path = Path::new(SHIM_DIR).join(name);
        let real_path = Path::new(REAL_BIN_DIR).join(name);
        shims.push(ShimHealth {
            name,
            symlink_ok: is_saya_symlink(&link_path, &exe)?,
            real_binary_exists: real_path.exists(),
        });
    }
    let path_local_bin_first =
        path_var_has_local_bin_first(&std::env::var("PATH").unwrap_or_default());
    Ok(DoctorReport {
        shims,
        path_local_bin_first,
    })
}

fn is_saya_symlink(link_path: &Path, exe: &Path) -> Result<bool> {
    match fs::symlink_metadata(link_path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            let target = fs::read_link(link_path)
                .with_context(|| format!("reading symlink {}", link_path.display()))?;
            Ok(target == exe)
        }
        Ok(_) => Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e).with_context(|| format!("checking {}", link_path.display())),
    }
}

fn current_exe() -> Result<PathBuf> {
    std::env::current_exe().context("resolving current executable path")
}

fn path_var_has_local_bin_first(path_var: &str) -> bool {
    let mut local_bin_idx = None;
    let mut real_bin_idx = None;
    for (i, dir) in path_var.split(':').enumerate() {
        if dir == SHIM_DIR && local_bin_idx.is_none() {
            local_bin_idx = Some(i);
        }
        if dir == REAL_BIN_DIR && real_bin_idx.is_none() {
            real_bin_idx = Some(i);
        }
    }
    match (local_bin_idx, real_bin_idx) {
        (Some(l), Some(r)) => l < r,
        (Some(_), None) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_with_local_bin_first_is_ok() {
        assert!(path_var_has_local_bin_first("/usr/local/bin:/usr/bin:/bin"));
    }

    #[test]
    fn path_with_local_bin_after_usr_bin_is_not_ok() {
        assert!(!path_var_has_local_bin_first("/usr/bin:/usr/local/bin"));
    }

    #[test]
    fn path_missing_local_bin_is_not_ok() {
        assert!(!path_var_has_local_bin_first("/usr/bin:/bin"));
    }

    #[test]
    fn path_with_only_local_bin_is_ok() {
        assert!(path_var_has_local_bin_first("/usr/local/bin"));
    }
}
