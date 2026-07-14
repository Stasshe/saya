use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::backend::Backend;
use crate::cli::ImportArgs;
use crate::manifest::{Manifest, validate_package_name};
use crate::privilege::{InvocationUser, drop_to_user};

pub fn run(
    args: &ImportArgs,
    manifest: &mut Manifest,
    backend: &dyn Backend,
    path: &Path,
    user: &InvocationUser,
) -> Result<()> {
    if !args.manual {
        bail!("import currently only supports --manual");
    }

    let candidates: Vec<String> = backend
        .list_manually_installed()?
        .into_iter()
        .filter(|name| !manifest.contains(name, backend.kind()))
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
        if !manifest.contains(name, backend.kind()) {
            manifest.record(name, backend.kind());
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
    let tmp_path = write_candidates_to_tempfile(candidates)?;

    let editor = editor_command();
    let mut command = editor.split_whitespace();
    let program = command.next().unwrap_or("vi");
    let status = Command::new(program)
        .args(command)
        .arg(&tmp_path)
        .status()
        .with_context(|| format!("running editor {editor}"))?;
    if !status.success() {
        let _ = fs::remove_file(&tmp_path);
        bail!("editor exited with {status}");
    }

    let text = match fs::read_to_string(&tmp_path) {
        Ok(text) => text,
        Err(err) => {
            let _ = fs::remove_file(&tmp_path);
            return Err(err).with_context(|| format!("reading {}", tmp_path.display()));
        }
    };
    let _ = fs::remove_file(&tmp_path);
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|line| {
            validate_package_name(line)
                .map_err(anyhow::Error::msg)
                .with_context(|| format!("invalid package name from editor: {line:?}"))?;
            Ok(line.to_string())
        })
        .collect()
}

fn write_candidates_to_tempfile(candidates: &[String]) -> Result<PathBuf> {
    let mut last_error = None;
    for attempt in 0..100 {
        let tmp_path =
            std::env::temp_dir().join(format!("saya-import-{}-{attempt}.txt", std::process::id()));
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp_path)
        {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                last_error = Some(err);
                continue;
            }
            Err(err) => {
                return Err(err).with_context(|| format!("creating {}", tmp_path.display()));
            }
        };
        file.write_all((candidates.join("\n") + "\n").as_bytes())
            .with_context(|| format!("writing {}", tmp_path.display()))?;
        return Ok(tmp_path);
    }

    let message = match last_error {
        Some(err) => format!("could not create import tempfile after 100 attempts: {err}"),
        None => "could not create import tempfile after 100 attempts".to_string(),
    };
    bail!(message)
}

fn editor_command() -> String {
    editor_command_from(std::env::var("VISUAL").ok(), std::env::var("EDITOR").ok())
}

fn editor_command_from(visual: Option<String>, editor: Option<String>) -> String {
    visual
        .filter(|value| !value.trim().is_empty())
        .or_else(|| editor.filter(|value| !value.trim().is_empty()))
        .unwrap_or_else(|| "vi".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_command_prefers_visual() {
        assert_eq!(
            editor_command_from(Some("code --wait".to_string()), Some("vim".to_string())),
            "code --wait"
        );
    }

    #[test]
    fn editor_command_falls_back_to_editor() {
        assert_eq!(
            editor_command_from(None, Some("vim -f".to_string())),
            "vim -f"
        );
    }

    #[test]
    fn editor_command_uses_vi_for_empty_env() {
        assert_eq!(editor_command_from(Some(" ".to_string()), None), "vi");
    }
}
