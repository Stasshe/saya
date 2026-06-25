// NOTE: untested on real Arch system — this dev environment has no pacman.
// Implemented from `pacman` man page; compiles, but behavior is unverified.
use std::process::Command;

use anyhow::{Context, Result, bail};

use super::{Backend, BackendKind};

pub struct PacmanBackend;

impl Backend for PacmanBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Pacman
    }

    fn update(&self) -> Result<()> {
        let status = super::package_manager_command("/usr/bin/pacman")
            .arg("-Sy")
            .status()
            .context("running pacman -Sy")?;
        if !status.success() {
            bail!("pacman -Sy failed with {status}");
        }
        Ok(())
    }

    fn upgrade(&self) -> Result<()> {
        let status = super::package_manager_command("/usr/bin/pacman")
            .arg("-Syu")
            .status()
            .context("running pacman -Syu")?;
        if !status.success() {
            bail!("pacman -Syu failed with {status}");
        }
        Ok(())
    }

    fn is_installed(&self, real_pkg_name: &str) -> Result<bool> {
        let output = Command::new("/usr/bin/pacman")
            .args(["-Q", real_pkg_name])
            .output()
            .context("running pacman -Q")?;
        if output.status.success() {
            return Ok(true);
        }
        if output.status.code() == Some(1) {
            return Ok(false);
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "pacman -Q failed for {real_pkg_name} with {}: {}",
            output.status,
            stderr.trim()
        );
    }

    fn install(&self, real_pkg_names: &[String]) -> Result<()> {
        if real_pkg_names.is_empty() {
            return Ok(());
        }
        let status = super::package_manager_command("/usr/bin/pacman")
            .arg("-S")
            .arg("--noconfirm")
            .arg("--")
            .args(real_pkg_names)
            .status()
            .context("running pacman -S")?;
        if !status.success() {
            bail!("pacman -S failed with {status}");
        }
        Ok(())
    }

    fn list_manually_installed(&self) -> Result<Vec<String>> {
        let output = Command::new("/usr/bin/pacman")
            .args(["-Qqe"])
            .output()
            .context("running pacman -Qqe")?;
        if !output.status.success() {
            bail!("pacman -Qqe failed with {}", output.status);
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect())
    }
}
