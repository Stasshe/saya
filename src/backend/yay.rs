// NOTE: This backend is exercised on Arch, but package-changing operations are
// not automated because they modify the host system.
use std::process::Command;

use anyhow::{Context, Result, bail};

use super::{Backend, BackendKind};

const YAY: &str = "/usr/bin/yay";

pub struct YayBackend;

impl Backend for YayBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Yay
    }

    fn update(&self) -> Result<()> {
        run(&["-Sy"], "yay -Sy")
    }

    fn upgrade(&self) -> Result<()> {
        run(&["-Syu"], "yay -Syu")
    }

    fn is_installed(&self, real_pkg_name: &str) -> Result<bool> {
        let output = Command::new(YAY)
            .args(["-Q", "--", real_pkg_name])
            .output()
            .context("running yay -Q")?;
        if output.status.success() {
            return Ok(true);
        }
        if output.status.code() == Some(1) {
            return Ok(false);
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "yay -Q failed for {real_pkg_name} with {}: {}",
            output.status,
            stderr.trim()
        );
    }

    fn install(&self, real_pkg_names: &[String], backend_args: &[String]) -> Result<()> {
        if real_pkg_names.is_empty() {
            return Ok(());
        }
        let status = Command::new(YAY)
            .arg("-S")
            .arg("--noconfirm")
            .args(backend_args)
            .arg("--")
            .args(real_pkg_names)
            .status()
            .context("running yay -S")?;
        if !status.success() {
            bail!("yay -S failed with {status}");
        }
        Ok(())
    }

    fn uninstall(&self, real_pkg_names: &[String]) -> Result<()> {
        if real_pkg_names.is_empty() {
            return Ok(());
        }
        let status = Command::new(YAY)
            .arg("-Rns")
            .arg("--noconfirm")
            .arg("--")
            .args(real_pkg_names)
            .status()
            .context("running yay -Rns")?;
        if !status.success() {
            bail!("yay -Rns failed with {status}");
        }
        Ok(())
    }

    fn list_manually_installed(&self) -> Result<Vec<String>> {
        let output = Command::new(YAY)
            .arg("-Qqe")
            .output()
            .context("running yay -Qqe")?;
        if !output.status.success() {
            bail!("yay -Qqe failed with {}", output.status);
        }
        Ok(parse_package_list(&String::from_utf8_lossy(&output.stdout)))
    }
}

fn run(args: &[&str], operation: &str) -> Result<()> {
    let status = Command::new(YAY)
        .args(args)
        .status()
        .with_context(|| format!("running {operation}"))?;
    if !status.success() {
        bail!("{operation} failed with {status}");
    }
    Ok(())
}

fn parse_package_list(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_package_list() {
        assert_eq!(
            parse_package_list("git\nyay-bin\n\n"),
            vec!["git".to_string(), "yay-bin".to_string()]
        );
    }
}
