pub mod import;
pub mod install;
pub mod self_update;
pub mod status;
pub mod uninstall;

use std::path::{Path, PathBuf};

use crate::backend::Backend;
use crate::manifest::Manifest;

pub fn manifest_path(home: &Path) -> PathBuf {
    home.join(".config/saya/packages.toml")
}

/// One manifest package name for the detected backend, paired with whether
/// it is currently installed.
pub struct PackageStatus {
    pub name: String,
    pub desired_present: bool,
    pub installed: bool,
}

pub fn compute_status(
    manifest: &Manifest,
    backend: &dyn Backend,
) -> anyhow::Result<Vec<PackageStatus>> {
    manifest
        .names(backend.kind())
        .iter()
        .map(|name| (name, true))
        .chain(
            manifest
                .absent_names(backend.kind())
                .iter()
                .map(|name| (name, false)),
        )
        .map(|(name, desired_present)| {
            let installed = backend.is_installed(name)?;
            Ok(PackageStatus {
                name: name.clone(),
                desired_present,
                installed,
            })
        })
        .collect()
}
