use std::path::Path;

use anyhow::Result;

use crate::manifest::{CURRENT_SCHEMA_VERSION, Manifest};
use crate::privilege::{InvocationUser, drop_to_user};

pub fn run(path: &Path, user: &InvocationUser) -> Result<()> {
    let manifest = Manifest::load_previous(path)?;
    drop_to_user(user)?;
    manifest.save(path)?;
    println!(
        "migrated manifest schema {} -> {}",
        CURRENT_SCHEMA_VERSION - 1,
        CURRENT_SCHEMA_VERSION
    );
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
        }
    }

    #[test]
    fn migrates_previous_manifest_in_place() {
        let dir = tempdir();
        let path = dir.join("packages.toml");
        let user = current_user(dir.clone());
        std::fs::write(
            &path,
            "schema_version = 4\napt = [\"git\"]\nyay = [\"neovim\"]\n",
        )
        .unwrap();

        run(&path, &user).unwrap();

        let manifest = Manifest::load(&path).unwrap();
        assert_eq!(manifest.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(manifest.apt.present, vec!["git"]);
        assert_eq!(manifest.yay.present, vec!["neovim"]);
    }

    #[test]
    fn leaves_manifest_unchanged_when_schema_is_not_previous() {
        let dir = tempdir();
        let path = dir.join("packages.toml");
        let user = current_user(dir.clone());
        let original = "schema_version = 3\napt = [\"git\"]\nyay = []\n";
        std::fs::write(&path, original).unwrap();

        assert!(run(&path, &user).is_err());

        assert_eq!(std::fs::read_to_string(path).unwrap(), original);
    }

    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("saya-test-{}", std::process::id()));
        let dir = dir.join(format!("{:?}", std::time::Instant::now()).replace(['.', ':'], "-"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
