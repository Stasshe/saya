use std::path::Path;

use anyhow::Result;

use crate::cli::AddArgs;
use crate::manifest::{Manifest, PackageEntry};
use crate::privilege::{chown_to_user, OriginalUser};

pub fn run_add(
    manifest: &mut Manifest,
    args: &AddArgs,
    path: &Path,
    user: &OriginalUser,
) -> Result<()> {
    manifest.packages.insert(
        args.logical.clone(),
        PackageEntry {
            apt: args.apt.clone(),
            pacman: args.pacman.clone(),
        },
    );
    manifest.save(path)?;
    chown_to_user(path, user)?;
    println!("added: {}", args.logical);
    Ok(())
}

pub fn run_forget(
    manifest: &mut Manifest,
    logical: &str,
    path: &Path,
    user: &OriginalUser,
) -> Result<()> {
    if manifest.packages.remove(logical).is_none() {
        println!("not in manifest: {logical}");
        return Ok(());
    }
    manifest.save(path)?;
    chown_to_user(path, user)?;
    println!("forgot: {logical}");
    Ok(())
}
