use std::path::Path;

use anyhow::Result;

use crate::backend::Backend;
use crate::cli::InstallArgs;
use crate::manifest::Manifest;
use crate::privilege::{InvocationUser, drop_to_user};

pub fn run(
    manifest: &mut Manifest,
    args: &InstallArgs,
    backend: &dyn Backend,
    path: &Path,
    user: &InvocationUser,
) -> Result<()> {
    backend.install(&args.packages)?;

    let mut changed = false;
    for real_name in &args.packages {
        if manifest
            .find_logical_name_by_real(real_name, backend.kind())
            .is_none()
        {
            manifest.record(real_name, real_name, backend.kind());
            changed = true;
        }
    }

    if changed {
        drop_to_user(user)?;
        manifest.save(path)?;
    }

    println!("installed: {}", args.packages.join(", "));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::backend::BackendKind;

    struct FakeBackend;

    impl Backend for FakeBackend {
        fn kind(&self) -> BackendKind {
            BackendKind::Apt
        }

        fn is_installed(&self, _real_pkg_name: &str) -> Result<bool> {
            Ok(false)
        }

        fn install(&self, real_pkg_names: &[String]) -> Result<()> {
            assert_eq!(real_pkg_names, ["git".to_string(), "curl".to_string()]);
            Ok(())
        }

        fn list_manually_installed(&self) -> Result<Vec<String>> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn records_packages_after_successful_install() {
        let dir = tempdir();
        let path = dir.join("packages.toml");
        let args = InstallArgs {
            packages: vec!["git".to_string(), "curl".to_string()],
        };
        let user = current_user(dir.clone());
        let mut manifest = Manifest::default();

        run(&mut manifest, &args, &FakeBackend, &path, &user).unwrap();

        let loaded = Manifest::load(&path).unwrap();
        assert!(loaded.packages.contains_key("git"));
        assert!(loaded.packages.contains_key("curl"));
    }

    fn current_user(home: std::path::PathBuf) -> InvocationUser {
        InvocationUser {
            // SAFETY: getuid/getgid take no arguments and cannot fail.
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            home,
        }
    }

    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("saya-test-{}", std::process::id()));
        let dir = dir.join(format!("{:?}", std::time::Instant::now()).replace(['.', ':'], "-"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
