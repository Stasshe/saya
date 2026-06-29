use std::path::Path;

use anyhow::Result;

use crate::backend::Backend;
use crate::cli::AddArgs;
use crate::manifest::{Manifest, PackageEntry};
use crate::privilege::{InvocationUser, drop_to_user};

pub fn run_add(
    manifest: &mut Manifest,
    args: &AddArgs,
    backend: &dyn Backend,
    path: &Path,
    user: &InvocationUser,
) -> Result<()> {
    let entry = PackageEntry {
        apt: args.apt.clone(),
        pacman: args.pacman.clone(),
    };
    let real_names = entry.resolve_names(&args.logical, backend.kind());

    backend.install(&real_names)?;

    if manifest.packages.get(&args.logical) == Some(&entry) {
        println!("already recorded: {}", args.logical);
        return Ok(());
    }

    manifest.packages.insert(args.logical.clone(), entry);
    drop_to_user(user)?;
    manifest.save(path)?;
    println!("added: {}", args.logical);
    Ok(())
}

pub fn run_forget(
    manifest: &mut Manifest,
    logical: &str,
    path: &Path,
    user: &InvocationUser,
) -> Result<()> {
    if manifest.packages.remove(logical).is_none() {
        println!("not in manifest: {logical}");
        return Ok(());
    }
    drop_to_user(user)?;
    manifest.save(path)?;
    println!("forgot: {logical}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::MetadataExt;

    use super::*;

    use crate::backend::BackendKind;

    struct FakeBackend;

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
            Ok(false)
        }

        fn install(&self, real_pkg_names: &[String]) -> Result<()> {
            assert_eq!(real_pkg_names, ["git-core".to_string()]);
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
    fn add_installs_and_records_package() {
        let dir = tempdir();
        let path = dir.join("packages.toml");
        let user = current_user(dir.clone());
        let mut manifest = Manifest::default();
        let args = AddArgs {
            logical: "git".to_string(),
            apt: vec!["git-core".to_string()],
            pacman: Vec::new(),
        };

        run_add(&mut manifest, &args, &FakeBackend, &path, &user).unwrap();

        let loaded = Manifest::load(&path).unwrap();
        assert_eq!(loaded.packages["git"].apt, vec!["git-core"]);
    }

    #[test]
    fn add_keeps_identical_manifest_unchanged() {
        let dir = tempdir();
        let path = dir.join("packages.toml");
        let user = current_user(dir.clone());
        let entry = PackageEntry {
            apt: vec!["git-core".to_string()],
            pacman: Vec::new(),
        };
        let mut manifest = Manifest::default();
        manifest.packages.insert("git".to_string(), entry);
        manifest.save(&path).unwrap();
        let inode = std::fs::metadata(&path).unwrap().ino();
        let args = AddArgs {
            logical: "git".to_string(),
            apt: vec!["git-core".to_string()],
            pacman: Vec::new(),
        };

        run_add(&mut manifest, &args, &FakeBackend, &path, &user).unwrap();

        assert_eq!(std::fs::metadata(path).unwrap().ino(), inode);
    }

    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("saya-test-{}", std::process::id()));
        let dir = dir.join(format!("{:?}", std::time::Instant::now()).replace(['.', ':'], "-"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
