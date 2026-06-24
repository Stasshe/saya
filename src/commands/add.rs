use std::path::Path;

use anyhow::{Result, bail};

use crate::cli::AddArgs;
use crate::manifest::{Manifest, PackageEntry};
use crate::privilege::{InvocationUser, drop_to_user};

pub fn run_add(
    manifest: &mut Manifest,
    args: &AddArgs,
    path: &Path,
    user: &InvocationUser,
) -> Result<()> {
    if manifest.packages.contains_key(&args.logical) {
        bail!("package already exists in manifest: {}", args.logical);
    }

    manifest.packages.insert(
        args.logical.clone(),
        PackageEntry {
            sudo: Some(user.used_sudo),
            apt: args.apt.clone(),
            pacman: args.pacman.clone(),
        },
    );
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
    use super::*;

    fn current_user(home: std::path::PathBuf) -> InvocationUser {
        InvocationUser {
            // SAFETY: getuid/getgid take no arguments and cannot fail.
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            home,
            used_sudo: false,
        }
    }

    #[test]
    fn add_rejects_existing_logical_name() {
        let dir = tempdir();
        let path = dir.join("packages.toml");
        let user = current_user(dir);
        let mut manifest = Manifest::default();
        manifest.packages.insert(
            "git".to_string(),
            PackageEntry {
                sudo: Some(true),
                apt: vec!["git-core".to_string()],
                pacman: Vec::new(),
            },
        );
        let args = AddArgs {
            logical: "git".to_string(),
            apt: Vec::new(),
            pacman: Vec::new(),
        };

        let err = run_add(&mut manifest, &args, &path, &user).unwrap_err();

        assert!(err.to_string().contains("already exists"));
        assert_eq!(manifest.packages["git"].apt, vec!["git-core"]);
        assert!(!path.exists());
    }

    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("saya-test-{}", std::process::id()));
        let dir = dir.join(format!("{:?}", std::time::Instant::now()).replace(['.', ':'], "-"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
