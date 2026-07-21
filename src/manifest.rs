use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::backend::BackendKind;

pub const CURRENT_SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub apt: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pacman: Vec<String>,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            apt: Vec::new(),
            pacman: Vec::new(),
        }
    }
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path)
            .with_context(|| format!("reading manifest at {}", path.display()))?;
        let manifest: Self = toml::from_str(&text)
            .with_context(|| format!("parsing manifest at {}", path.display()))?;
        manifest
            .validate()
            .with_context(|| format!("validating manifest at {}", path.display()))?;
        Ok(manifest)
    }

    /// Atomic write: write to a unique sibling temporary file then rename over the target.
    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()
            .with_context(|| format!("validating manifest at {}", path.display()))?;
        let text = toml::to_string_pretty(self).context("serializing manifest")?;
        if fs::read(path).is_ok_and(|current| current == text.as_bytes()) {
            fs::set_permissions(path, fs::Permissions::from_mode(0o644))
                .with_context(|| format!("setting permissions on {}", path.display()))?;
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating manifest dir {}", parent.display()))?;
        }
        let (tmp_path, mut tmp_file) = create_tempfile(path)?;
        tmp_file
            .write_all(text.as_bytes())
            .with_context(|| format!("writing {}", tmp_path.display()))?;
        tmp_file
            .set_permissions(fs::Permissions::from_mode(0o644))
            .with_context(|| format!("setting permissions on {}", tmp_path.display()))?;
        tmp_file
            .sync_all()
            .with_context(|| format!("syncing {}", tmp_path.display()))?;
        drop(tmp_file);
        fs::rename(&tmp_path, path)
            .with_context(|| format!("renaming {} to {}", tmp_path.display(), path.display()))?;
        Ok(())
    }

    pub fn names(&self, kind: BackendKind) -> &[String] {
        match kind {
            BackendKind::Apt => &self.apt,
            BackendKind::Pacman => &self.pacman,
        }
    }

    fn names_mut(&mut self, kind: BackendKind) -> &mut Vec<String> {
        match kind {
            BackendKind::Apt => &mut self.apt,
            BackendKind::Pacman => &mut self.pacman,
        }
    }

    pub fn contains(&self, name: &str, kind: BackendKind) -> bool {
        self.names(kind).iter().any(|n| n == name)
    }

    /// Appends `name` to `kind`'s list. No-op if already present.
    pub fn record(&mut self, name: &str, kind: BackendKind) {
        debug_assert!(validate_package_name(name).is_ok());
        let names = self.names_mut(kind);
        if !names.iter().any(|n| n == name) {
            names.push(name.to_string());
        }
    }

    /// Removes `name` from `kind`'s list. Returns whether it was present.
    pub fn remove(&mut self, name: &str, kind: BackendKind) -> bool {
        let names = self.names_mut(kind);
        let len_before = names.len();
        names.retain(|n| n != name);
        names.len() != len_before
    }

    fn validate(&self) -> Result<()> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            anyhow::bail!(
                "unsupported manifest schema_version {}; expected {}",
                self.schema_version,
                CURRENT_SCHEMA_VERSION
            );
        }

        for name in self.apt.iter().chain(self.pacman.iter()) {
            validate_package_name(name)
                .map_err(anyhow::Error::msg)
                .with_context(|| format!("invalid package name {name:?}"))?;
        }
        Ok(())
    }
}

fn create_tempfile(path: &Path) -> Result<(std::path::PathBuf, fs::File)> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("packages.toml");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("reading system time")?
        .as_nanos();

    for attempt in 0..100 {
        let tmp_path = parent.join(format!(
            ".{stem}.{}.{}.{attempt}.tmp",
            std::process::id(),
            now
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp_path)
        {
            Ok(file) => return Ok((tmp_path, file)),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => {
                return Err(err).with_context(|| format!("creating {}", tmp_path.display()));
            }
        }
    }

    anyhow::bail!("could not create manifest tempfile after 100 attempts")
}

pub fn validate_package_name(name: &str) -> std::result::Result<(), String> {
    if name.is_empty() {
        return Err("package name cannot be empty".to_string());
    }

    let mut chars = name.chars();
    let first = chars
        .next()
        .expect("non-empty string has a first character");
    if !first.is_ascii_alphanumeric() {
        return Err("package name must start with an ASCII letter or digit".to_string());
    }

    if let Some(ch) = chars.find(|ch| !is_allowed_package_name_char(*ch)) {
        return Err(format!(
            "package name contains unsupported character {ch:?}"
        ));
    }

    Ok(())
}

fn is_allowed_package_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '+' | '-' | '@' | ':')
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::MetadataExt;

    use super::*;

    #[test]
    fn load_missing_file_returns_default() {
        let dir = tempdir();
        let path = dir.join("packages.toml");
        let manifest = Manifest::load(&path).unwrap();
        assert_eq!(manifest, Manifest::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempdir();
        let path = dir.join("packages.toml");
        let mut manifest = Manifest::default();
        manifest.record("git", BackendKind::Apt);
        manifest.record("neovim", BackendKind::Pacman);
        manifest.save(&path).unwrap();

        let loaded = Manifest::load(&path).unwrap();
        assert_eq!(loaded, manifest);
        assert_eq!(std::fs::metadata(&path).unwrap().mode() & 0o777, 0o644);
        assert!(!path.with_extension("tmp").exists());
    }

    #[test]
    fn save_does_not_reuse_fixed_tmp_path() {
        let dir = tempdir();
        let path = dir.join("packages.toml");
        let old_fixed_tmp_path = path.with_extension("tmp");
        std::fs::write(&old_fixed_tmp_path, "do not touch").unwrap();

        let manifest = Manifest::default();
        manifest.save(&path).unwrap();

        assert_eq!(
            std::fs::read_to_string(old_fixed_tmp_path).unwrap(),
            "do not touch"
        );
        assert_eq!(Manifest::load(&path).unwrap(), manifest);
    }

    #[test]
    fn save_does_not_replace_an_unchanged_file() {
        let dir = tempdir();
        let path = dir.join("packages.toml");
        let manifest = Manifest::default();
        manifest.save(&path).unwrap();
        let inode = std::fs::metadata(&path).unwrap().ino();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();

        manifest.save(&path).unwrap();

        assert_eq!(std::fs::metadata(&path).unwrap().ino(), inode);
        assert_eq!(std::fs::metadata(&path).unwrap().mode() & 0o777, 0o644);
    }

    #[test]
    fn record_is_idempotent() {
        let mut manifest = Manifest::default();
        manifest.record("git", BackendKind::Apt);
        manifest.record("git", BackendKind::Apt);
        assert_eq!(manifest.apt, vec!["git".to_string()]);
    }

    #[test]
    fn record_keeps_backends_independent() {
        let mut manifest = Manifest::default();
        manifest.record("neovim", BackendKind::Apt);
        assert!(manifest.contains("neovim", BackendKind::Apt));
        assert!(!manifest.contains("neovim", BackendKind::Pacman));
    }

    #[test]
    fn remove_removes_only_from_matching_backend() {
        let mut manifest = Manifest::default();
        manifest.record("neovim", BackendKind::Apt);
        manifest.record("neovim", BackendKind::Pacman);

        assert!(manifest.remove("neovim", BackendKind::Apt));
        assert!(!manifest.contains("neovim", BackendKind::Apt));
        assert!(manifest.contains("neovim", BackendKind::Pacman));
    }

    #[test]
    fn remove_returns_false_when_absent() {
        let mut manifest = Manifest::default();
        assert!(!manifest.remove("neovim", BackendKind::Apt));
    }

    #[test]
    fn load_rejects_unsupported_manifest_version() {
        let dir = tempdir();
        let path = dir.join("packages.toml");
        std::fs::write(&path, "schema_version = 999\n").unwrap();

        let err = Manifest::load(&path).unwrap_err().to_string();
        assert!(err.contains("validating manifest"));
    }

    #[test]
    fn load_rejects_old_schema_shape() {
        let dir = tempdir();
        let path = dir.join("packages.toml");
        std::fs::write(&path, "schema_version = 2\n[packages.git]\napt = []\n").unwrap();

        assert!(Manifest::load(&path).is_err());
    }

    #[test]
    fn validate_package_name_rejects_option_like_name() {
        assert!(validate_package_name("--download-only").is_err());
    }

    #[test]
    fn validate_package_name_rejects_path_like_name() {
        assert!(validate_package_name("foo/bar").is_err());
    }

    #[test]
    fn validate_package_name_rejects_whitespace_and_control_chars() {
        assert!(validate_package_name("git curl").is_err());
        assert!(validate_package_name("git\ncurl").is_err());
        assert!(validate_package_name("git\tcurl").is_err());
    }

    #[test]
    fn validate_package_name_accepts_common_package_names() {
        assert!(validate_package_name("libssl-dev:amd64").is_ok());
        assert!(validate_package_name("python-pynvim").is_ok());
        assert!(validate_package_name("mingw-w64-gcc").is_ok());
    }

    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("saya-test-{}", std::process::id()));
        let dir = dir.join(format!("{:?}", std::time::Instant::now()).replace(['.', ':'], "-"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
