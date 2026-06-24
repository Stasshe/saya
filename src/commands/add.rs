use std::path::Path;

use anyhow::Result;

use crate::cli::AddArgs;
use crate::manifest::{Manifest, PackageEntry};
use crate::privilege::{InvocationUser, drop_to_user};

pub fn run_add(
    manifest: &mut Manifest,
    args: &AddArgs,
    path: &Path,
    user: &InvocationUser,
) -> Result<()> {
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
