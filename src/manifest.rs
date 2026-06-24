use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::backend::BackendKind;

#[derive(Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct Manifest {
    pub version: u32,
    #[serde(default)]
    pub packages: BTreeMap<String, PackageEntry>,
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct PackageEntry {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub apt: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pacman: Vec<String>,
}

impl PackageEntry {
    /// Real package names for `kind`. Empty list means the logical name
    /// itself is the real package name on every distro.
    pub fn resolve_names(&self, logical: &str, kind: BackendKind) -> Vec<String> {
        let names = match kind {
            BackendKind::Apt => &self.apt,
            BackendKind::Pacman => &self.pacman,
        };
        if names.is_empty() {
            vec![logical.to_string()]
        } else {
            names.clone()
        }
    }

    fn names_for_mut(&mut self, kind: BackendKind) -> &mut Vec<String> {
        match kind {
            BackendKind::Apt => &mut self.apt,
            BackendKind::Pacman => &mut self.pacman,
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
        Ok(manifest)
    }

    /// Atomic write: write to a sibling `.tmp` file then rename over the target.
    pub fn save(&self, path: &Path) -> Result<()> {
        let text = toml::to_string_pretty(self).context("serializing manifest")?;
        let tmp_path = path.with_extension("tmp");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating manifest dir {}", parent.display()))?;
        }
        fs::write(&tmp_path, text)
            .with_context(|| format!("writing {}", tmp_path.display()))?;
        fs::rename(&tmp_path, path)
            .with_context(|| format!("renaming {} to {}", tmp_path.display(), path.display()))?;
        Ok(())
    }

    /// Finds the logical name whose real package list (for `kind`) already
    /// contains `real_name`, or whose logical name equals `real_name` with an
    /// empty list for `kind` (implicit real == logical).
    pub fn find_logical_name_by_real(&self, real_name: &str, kind: BackendKind) -> Option<String> {
        for (logical, entry) in &self.packages {
            let names = match kind {
                BackendKind::Apt => &entry.apt,
                BackendKind::Pacman => &entry.pacman,
            };
            if names.iter().any(|n| n == real_name) {
                return Some(logical.clone());
            }
            if names.is_empty() && logical == real_name {
                return Some(logical.clone());
            }
        }
        None
    }

    /// Records `real_name` under `logical` for `kind`, creating the entry if needed.
    /// No-op if already recorded.
    pub fn record(&mut self, logical: &str, real_name: &str, kind: BackendKind) {
        let entry = self.packages.entry(logical.to_string()).or_default();
        if logical == real_name {
            // implicit form: leave the per-backend list empty.
            return;
        }
        let names = entry.names_for_mut(kind);
        if !names.iter().any(|n| n == real_name) {
            names.push(real_name.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
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
        manifest.record("git", "git", BackendKind::Apt);
        manifest.record("neovim", "neovim", BackendKind::Pacman);
        manifest.save(&path).unwrap();

        let loaded = Manifest::load(&path).unwrap();
        assert_eq!(loaded, manifest);
        assert!(!path.with_extension("tmp").exists());
    }

    #[test]
    fn resolve_names_falls_back_to_logical_when_empty() {
        let entry = PackageEntry::default();
        assert_eq!(entry.resolve_names("git", BackendKind::Apt), vec!["git"]);
    }

    #[test]
    fn resolve_names_uses_explicit_list_when_present() {
        let entry = PackageEntry {
            apt: vec!["neovim".to_string()],
            pacman: vec![],
        };
        assert_eq!(
            entry.resolve_names("nvim", BackendKind::Apt),
            vec!["neovim"]
        );
        assert_eq!(entry.resolve_names("nvim", BackendKind::Pacman), vec!["nvim"]);
    }

    #[test]
    fn find_logical_name_by_real_matches_explicit_list() {
        let mut manifest = Manifest::default();
        manifest.record("nvim", "neovim", BackendKind::Apt);
        assert_eq!(
            manifest.find_logical_name_by_real("neovim", BackendKind::Apt),
            Some("nvim".to_string())
        );
        assert_eq!(
            manifest.find_logical_name_by_real("neovim", BackendKind::Pacman),
            None
        );
    }

    #[test]
    fn find_logical_name_by_real_matches_implicit_logical_name() {
        let mut manifest = Manifest::default();
        manifest.record("git", "git", BackendKind::Apt);
        assert_eq!(
            manifest.find_logical_name_by_real("git", BackendKind::Apt),
            Some("git".to_string())
        );
    }

    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("saya-test-{}", std::process::id()));
        let dir = dir.join(format!("{:?}", std::time::Instant::now()).replace(['.', ':'], "-"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
