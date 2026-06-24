pub mod add;
pub mod apply;
pub mod import;
pub mod status;

use std::path::{Path, PathBuf};

use crate::backend::Backend;
use crate::manifest::Manifest;

pub fn manifest_path(home: &Path) -> PathBuf {
    home.join(".config/saya/packages.toml")
}

/// One (logical name, real package name) pair from the manifest, paired
/// with whether that real package is currently installed.
pub struct PackageStatus {
    pub logical: String,
    pub real_name: String,
    pub installed: bool,
}

pub fn compute_status(
    manifest: &Manifest,
    backend: &dyn Backend,
) -> anyhow::Result<Vec<PackageStatus>> {
    let mut result = Vec::new();
    for (logical, entry) in &manifest.packages {
        for real_name in entry.resolve_names(logical, backend.kind()) {
            let installed = backend.is_installed(&real_name)?;
            result.push(PackageStatus {
                logical: logical.clone(),
                real_name,
                installed,
            });
        }
    }
    Ok(result)
}
