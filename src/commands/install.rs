use std::path::Path;

use anyhow::Result;

use crate::backend::Backend;
use crate::manifest::Manifest;
use crate::privilege::{InvocationUser, drop_to_user};

/// `saya install` with no names: install everything missing from the manifest.
pub fn run_missing(
    manifest: &Manifest,
    backend_args: &[String],
    backend: &dyn Backend,
) -> Result<()> {
    let statuses = super::compute_status(manifest, backend)?;
    let missing: Vec<String> = statuses
        .into_iter()
        .filter(|status| !status.installed)
        .map(|status| status.name)
        .collect();

    if missing.is_empty() {
        println!("already up to date");
        return Ok(());
    }

    println!("installing: {}", missing.join(", "));
    backend.install(&missing, backend_args)
}

/// `saya install <names...>`: install packages and record them on success.
pub fn run_packages(
    manifest: &mut Manifest,
    names: &[String],
    backend_args: &[String],
    backend: &dyn Backend,
    path: &Path,
    user: &InvocationUser,
) -> Result<()> {
    backend.install(names, backend_args)?;

    let added: Vec<String> = names
        .iter()
        .filter(|name| !manifest.contains(name, backend.kind()))
        .cloned()
        .collect();
    if added.is_empty() {
        println!("already recorded: {}", names.join(", "));
        return Ok(());
    }

    for name in &added {
        manifest.record(name, backend.kind());
    }
    drop_to_user(user)?;
    manifest.save(path)?;
    println!("added: {}", added.join(", "));
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::MetadataExt;

    use super::*;

    use crate::backend::BackendKind;

    struct FakeBackend {
        installed: Vec<String>,
        expected: Vec<String>,
        expected_backend_args: Vec<String>,
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

        fn is_installed(&self, real_pkg_name: &str) -> Result<bool> {
            Ok(self.installed.iter().any(|name| name == real_pkg_name))
        }

        fn install(&self, real_pkg_names: &[String], backend_args: &[String]) -> Result<()> {
            assert_eq!(real_pkg_names, self.expected);
            assert_eq!(backend_args, self.expected_backend_args);
            Ok(())
        }

        fn uninstall(&self, _real_pkg_names: &[String]) -> Result<()> {
            unreachable!("install command never uninstalls")
        }

        fn list_manually_installed(&self) -> Result<Vec<String>> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn installs_only_missing_manifest_packages() {
        let mut manifest = Manifest::default();
        manifest.record("git", BackendKind::Apt);
        manifest.record("curl", BackendKind::Apt);
        let backend = FakeBackend {
            installed: vec!["git".to_string()],
            expected: vec!["curl".to_string()],
            expected_backend_args: Vec::new(),
        };

        run_missing(&manifest, &[], &backend).unwrap();
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
    fn run_packages_installs_and_records_packages() {
        let dir = tempdir();
        let path = dir.join("packages.toml");
        let user = current_user(dir.clone());
        let mut manifest = Manifest::default();
        let backend = FakeBackend {
            installed: Vec::new(),
            expected: vec!["neovim".to_string(), "git".to_string()],
            expected_backend_args: vec!["--config".to_string(), "/tmp/yay.conf".to_string()],
        };

        run_packages(
            &mut manifest,
            &["neovim".to_string(), "git".to_string()],
            &["--config".to_string(), "/tmp/yay.conf".to_string()],
            &backend,
            &path,
            &user,
        )
        .unwrap();

        let loaded = Manifest::load(&path).unwrap();
        assert_eq!(loaded.apt, vec!["neovim", "git"]);
    }

    #[test]
    fn run_packages_keeps_identical_manifest_unchanged() {
        let dir = tempdir();
        let path = dir.join("packages.toml");
        let user = current_user(dir.clone());
        let mut manifest = Manifest::default();
        manifest.record("neovim", BackendKind::Apt);
        manifest.save(&path).unwrap();
        let inode = std::fs::metadata(&path).unwrap().ino();
        let backend = FakeBackend {
            installed: Vec::new(),
            expected: vec!["neovim".to_string()],
            expected_backend_args: Vec::new(),
        };

        run_packages(
            &mut manifest,
            &["neovim".to_string()],
            &[],
            &backend,
            &path,
            &user,
        )
        .unwrap();

        assert_eq!(std::fs::metadata(path).unwrap().ino(), inode);
    }

    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("saya-test-{}", std::process::id()));
        let dir = dir.join(format!("{:?}", std::time::Instant::now()).replace(['.', ':'], "-"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
