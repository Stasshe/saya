use std::path::Path;

use anyhow::Result;

use crate::backend::Backend;
use crate::manifest::Manifest;
use crate::privilege::{InvocationUser, drop_to_user};

/// `saya uninstall <names...>`: uninstall through the detected backend, then
/// record the packages as desired absent.
pub fn run(
    manifest: &mut Manifest,
    names: &[String],
    backend: &dyn Backend,
    path: &Path,
    user: &InvocationUser,
) -> Result<()> {
    backend.uninstall(names)?;

    let newly_absent: Vec<&str> = names
        .iter()
        .filter(|name| !manifest.is_absent(name, backend.kind()))
        .map(String::as_str)
        .collect();
    if newly_absent.is_empty() {
        println!("already recorded absent: {}", names.join(", "));
        return Ok(());
    }

    for name in names {
        manifest.record_absent(name, backend.kind());
    }
    drop_to_user(user)?;
    manifest.save(path)?;
    println!("recorded absent: {}", newly_absent.join(", "));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::backend::BackendKind;

    struct FakeBackend {
        expected: Vec<String>,
    }

    impl Backend for FakeBackend {
        fn kind(&self) -> BackendKind {
            BackendKind::Apt
        }

        fn update(&self) -> Result<()> {
            Ok(())
        }

        fn upgrade(&self) -> Result<()> {
            Ok(())
        }

        fn is_installed(&self, _real_pkg_name: &str) -> Result<bool> {
            unreachable!("uninstall command never checks installation")
        }

        fn install(&self, _real_pkg_names: &[String], _backend_args: &[String]) -> Result<()> {
            unreachable!("uninstall command never installs")
        }

        fn uninstall(&self, real_pkg_names: &[String]) -> Result<()> {
            assert_eq!(real_pkg_names, self.expected);
            Ok(())
        }

        fn list_manually_installed(&self) -> Result<Vec<String>> {
            Ok(Vec::new())
        }
    }

    fn current_user(home: std::path::PathBuf) -> InvocationUser {
        InvocationUser {
            // SAFETY: getuid/getgid take no arguments and cannot fail.
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            home,
        }
    }

    #[test]
    fn uninstalls_and_records_absent_for_current_backend_only() {
        let dir = tempdir();
        let path = dir.join("packages.toml");
        let user = current_user(dir.clone());
        let mut manifest = Manifest::default();
        manifest.record("neovim", BackendKind::Apt);
        manifest.record("git", BackendKind::Apt);
        manifest.record("neovim", BackendKind::Yay);
        manifest.save(&path).unwrap();
        let backend = FakeBackend {
            expected: vec!["neovim".to_string(), "git".to_string()],
        };

        run(
            &mut manifest,
            &["neovim".to_string(), "git".to_string()],
            &backend,
            &path,
            &user,
        )
        .unwrap();

        let loaded = Manifest::load(&path).unwrap();
        assert!(loaded.apt.present.is_empty());
        assert_eq!(loaded.apt.absent, vec!["neovim", "git"]);
        assert_eq!(loaded.yay.present, vec!["neovim"]);
    }

    #[test]
    fn uninstalls_when_not_in_manifest() {
        let dir = tempdir();
        let path = dir.join("packages.toml");
        let user = current_user(dir.clone());
        let mut manifest = Manifest::default();
        manifest.save(&path).unwrap();
        let backend = FakeBackend {
            expected: vec!["neovim".to_string(), "git".to_string()],
        };

        run(
            &mut manifest,
            &["neovim".to_string(), "git".to_string()],
            &backend,
            &path,
            &user,
        )
        .unwrap();

        let loaded = Manifest::load(&path).unwrap();
        assert_eq!(loaded.apt.absent, vec!["neovim", "git"]);
    }

    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("saya-test-{}", std::process::id()));
        let dir = dir.join(format!("{:?}", std::time::Instant::now()).replace(['.', ':'], "-"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
