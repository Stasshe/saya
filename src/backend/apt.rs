use std::process::Command;

use anyhow::{Context, Result, bail};

use super::{Backend, BackendKind};

pub struct AptBackend;

impl Backend for AptBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Apt
    }

    fn update(&self) -> Result<()> {
        let status = super::package_manager_command("/usr/bin/apt-get")
            .arg("update")
            .status()
            .context("running apt-get update")?;
        if !status.success() {
            bail!("apt-get update failed with {status}");
        }
        Ok(())
    }

    fn upgrade(&self) -> Result<()> {
        let status = super::package_manager_command("/usr/bin/apt-get")
            .arg("upgrade")
            .status()
            .context("running apt-get upgrade")?;
        if !status.success() {
            bail!("apt-get upgrade failed with {status}");
        }
        Ok(())
    }

    fn is_installed(&self, real_pkg_name: &str) -> Result<bool> {
        let output = Command::new("/usr/bin/dpkg-query")
            .args(["-W", "-f=${Status}", real_pkg_name])
            .output()
            .context("running dpkg-query")?;
        if !output.status.success() {
            if output.status.code() == Some(1) {
                // dpkg-query exits 1 when the package is unknown.
                return Ok(false);
            }
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "dpkg-query failed for {real_pkg_name} with {}: {}",
                output.status,
                stderr.trim()
            );
        }
        let status = String::from_utf8_lossy(&output.stdout);
        Ok(status.contains("install ok installed"))
    }

    fn install(&self, real_pkg_names: &[String]) -> Result<()> {
        if real_pkg_names.is_empty() {
            return Ok(());
        }
        let status = super::package_manager_command("/usr/bin/apt-get")
            .arg("install")
            .arg("-y")
            .arg("--")
            .args(real_pkg_names)
            .status()
            .context("running apt-get install")?;
        if !status.success() {
            bail!("apt-get install failed with {status}");
        }
        Ok(())
    }

    fn list_manually_installed(&self) -> Result<Vec<String>> {
        let output = Command::new("/usr/bin/apt-mark")
            .arg("showmanual")
            .output()
            .context("running apt-mark showmanual")?;
        if !output.status.success() {
            bail!("apt-mark showmanual failed with {}", output.status);
        }
        Ok(parse_showmanual(&String::from_utf8_lossy(&output.stdout)))
    }
}

fn parse_showmanual(stdout: &str) -> Vec<String> {
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
    fn parses_showmanual_output() {
        let stdout = "git\nneovim\ncurl\n";
        assert_eq!(
            parse_showmanual(stdout),
            vec!["git".to_string(), "neovim".to_string(), "curl".to_string()]
        );
    }

    #[test]
    fn parses_showmanual_ignores_blank_lines() {
        let stdout = "git\n\nneovim\n\n";
        assert_eq!(
            parse_showmanual(stdout),
            vec!["git".to_string(), "neovim".to_string()]
        );
    }

    #[test]
    fn parses_empty_showmanual_output() {
        assert_eq!(parse_showmanual(""), Vec::<String>::new());
    }
}
