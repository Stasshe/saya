use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::backend::Backend;
use crate::cli::ImportArgs;
use crate::manifest::Manifest;
use crate::privilege::{OriginalUser, drop_to_user};

pub fn run(
    args: &ImportArgs,
    manifest: &mut Manifest,
    backend: &dyn Backend,
    path: &Path,
    user: &OriginalUser,
) -> Result<()> {
    if !args.manual {
        bail!("import currently only supports --manual");
    }

    let candidates: Vec<String> = backend
        .list_manually_installed()?
        .into_iter()
        .filter(|name| {
            manifest
                .find_logical_name_by_real(name, backend.kind())
                .is_none()
        })
        .collect();

    if candidates.is_empty() {
        println!("nothing to import");
        return Ok(());
    }

    if !args.edit {
        for name in &candidates {
            println!("{name}");
        }
        println!("\nrerun with --edit to review and save");
        return Ok(());
    }

    drop_to_user(user)?;
    let selected = edit_candidates(&candidates)?;
    let mut changed = false;
    for name in &selected {
        if manifest
            .find_logical_name_by_real(name, backend.kind())
            .is_none()
        {
            manifest.record(name, name, backend.kind(), user.used_sudo);
            changed = true;
        }
    }
    if changed {
        manifest.save(path)?;
    }
    println!("imported {} package(s)", selected.len());
    Ok(())
}

/// Writes `candidates` to a tmpfile, opens `$EDITOR` (falling back to `vi`)
/// on it, then reads back whatever lines remain.
fn edit_candidates(candidates: &[String]) -> Result<Vec<String>> {
    let tmp_path = std::env::temp_dir().join(format!("saya-import-{}.txt", std::process::id()));
    fs::write(&tmp_path, candidates.join("\n") + "\n")
        .with_context(|| format!("writing {}", tmp_path.display()))?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = Command::new(&editor)
        .arg(&tmp_path)
        .status()
        .with_context(|| format!("running editor {editor}"))?;
    if !status.success() {
        bail!("editor exited with {status}");
    }

    let text =
        fs::read_to_string(&tmp_path).with_context(|| format!("reading {}", tmp_path.display()))?;
    let _ = fs::remove_file(&tmp_path);
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect())
}
