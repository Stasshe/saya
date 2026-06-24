mod apt;
mod pacman;

use anyhow::{Context, Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Apt,
    Pacman,
}

pub trait Backend {
    fn kind(&self) -> BackendKind;
    fn is_installed(&self, real_pkg_name: &str) -> Result<bool>;
    fn install(&self, real_pkg_names: &[String]) -> Result<()>;
    fn list_manually_installed(&self) -> Result<Vec<String>>;
}

/// Picks a backend by reading `ID`/`ID_LIKE` from `/etc/os-release`.
pub fn detect_backend() -> Result<Box<dyn Backend>> {
    let text = std::fs::read_to_string("/etc/os-release").context("reading /etc/os-release")?;
    detect_backend_from_os_release(&text)
}

fn detect_backend_from_os_release(text: &str) -> Result<Box<dyn Backend>> {
    let mut id = String::new();
    let mut id_like = String::new();
    for line in text.lines() {
        if let Some(val) = line.strip_prefix("ID=") {
            id = unquote(val);
        } else if let Some(val) = line.strip_prefix("ID_LIKE=") {
            id_like = unquote(val);
        }
    }
    let haystack = format!("{id} {id_like}");
    if haystack
        .split_whitespace()
        .any(|tok| matches!(tok, "debian" | "ubuntu"))
    {
        return Ok(Box::new(apt::AptBackend));
    }
    if haystack.split_whitespace().any(|tok| tok == "arch") {
        return Ok(Box::new(pacman::PacmanBackend));
    }
    bail!("unsupported distro (ID={id}, ID_LIKE={id_like})");
}

fn unquote(val: &str) -> String {
    val.trim().trim_matches('"').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_debian() {
        let os_release = "ID=debian\nID_LIKE=\n";
        let backend = detect_backend_from_os_release(os_release).unwrap();
        assert_eq!(backend.kind(), BackendKind::Apt);
    }

    #[test]
    fn detects_ubuntu_via_id_like() {
        let os_release = "ID=ubuntu\nID_LIKE=debian\n";
        let backend = detect_backend_from_os_release(os_release).unwrap();
        assert_eq!(backend.kind(), BackendKind::Apt);
    }

    #[test]
    fn detects_arch() {
        let os_release = "ID=arch\n";
        let backend = detect_backend_from_os_release(os_release).unwrap();
        assert_eq!(backend.kind(), BackendKind::Pacman);
    }

    #[test]
    fn rejects_unknown_distro() {
        let os_release = "ID=fedora\n";
        assert!(detect_backend_from_os_release(os_release).is_err());
    }
}
