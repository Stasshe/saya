use std::process::Command;

use anyhow::{Context, Result};

use crate::backend::BackendKind;
use crate::commands::manifest_path;
use crate::manifest::Manifest;
use crate::privilege::{drop_to_user, resolve_invocation_user};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShimKind {
    Apt,
    AptGet,
    Pacman,
}

impl ShimKind {
    pub fn from_basename(name: &str) -> Option<Self> {
        match name {
            "apt" => Some(Self::Apt),
            "apt-get" => Some(Self::AptGet),
            "pacman" => Some(Self::Pacman),
            _ => None,
        }
    }

    fn real_path(&self) -> &'static str {
        match self {
            Self::Apt => "/usr/bin/apt",
            Self::AptGet => "/usr/bin/apt-get",
            Self::Pacman => "/usr/bin/pacman",
        }
    }

    fn backend_kind(&self) -> BackendKind {
        match self {
            Self::Apt | Self::AptGet => BackendKind::Apt,
            Self::Pacman => BackendKind::Pacman,
        }
    }
}

/// Runs the real package manager, then on success records any directly
/// requested package names into the manifest. Returns the real command's
/// exit code so the caller can `process::exit` with it unchanged.
pub fn run(kind: ShimKind, args: &[String]) -> Result<i32> {
    let status = Command::new(kind.real_path())
        .args(args)
        .status()
        .with_context(|| format!("running {}", kind.real_path()))?;

    if status.success() {
        let targets = parse_install_targets(kind.backend_kind(), args);
        if !targets.is_empty() {
            record_targets(kind.backend_kind(), &targets)?;
        }
    }

    Ok(status.code().unwrap_or(1))
}

fn record_targets(kind: BackendKind, targets: &[String]) -> Result<()> {
    let user = resolve_invocation_user()?;
    let path = manifest_path(&user.home);
    let mut manifest = Manifest::load(&path)?;

    let mut changed = false;
    for real_name in targets {
        if manifest
            .find_logical_name_by_real(real_name, kind)
            .is_none()
        {
            manifest.record(real_name, real_name, kind, user.used_sudo);
            changed = true;
        }
    }

    if changed {
        drop_to_user(&user)?;
        manifest.save(&path)?;
    }
    Ok(())
}

/// Extracts directly-specified package names from an install invocation.
/// Dependency-resolved packages never appear on the command line, so this
/// is naturally limited to what the user actually typed.
fn parse_install_targets(kind: BackendKind, args: &[String]) -> Vec<String> {
    match kind {
        BackendKind::Apt => parse_apt_install_targets(args),
        BackendKind::Pacman => parse_pacman_install_targets(args),
    }
}

fn parse_apt_install_targets(args: &[String]) -> Vec<String> {
    let mut positionals = args.iter().filter(|a| !a.starts_with('-'));
    let Some(subcommand) = positionals.next() else {
        return Vec::new();
    };
    if subcommand != "install" {
        return Vec::new();
    }
    positionals
        .filter(|pkg| is_plain_apt_package_name(pkg))
        .cloned()
        .collect()
}

fn is_plain_apt_package_name(s: &str) -> bool {
    !(s.ends_with(".deb") || s.contains("://") || s.contains('=') || s.contains('/'))
}

fn parse_pacman_install_targets(args: &[String]) -> Vec<String> {
    let positionals: Vec<String> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .collect();
    if positionals.is_empty() {
        return Vec::new();
    }

    let flags: Vec<&str> = args
        .iter()
        .filter(|a| a.starts_with('-'))
        .map(String::as_str)
        .collect();
    let has_remove = flags
        .iter()
        .any(|f| f.starts_with("-R") || *f == "--remove");
    let has_sync = flags
        .iter()
        .any(|f| f.starts_with("-S") && !f.contains('c'));
    if has_remove || !has_sync {
        return Vec::new();
    }
    positionals
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn apt_install_records_plain_package_names() {
        let targets = parse_install_targets(BackendKind::Apt, &args(&["install", "git", "curl"]));
        assert_eq!(targets, vec!["git".to_string(), "curl".to_string()]);
    }

    #[test]
    fn apt_install_excludes_leading_options() {
        let targets = parse_install_targets(BackendKind::Apt, &args(&["-y", "install", "git"]));
        assert_eq!(targets, vec!["git".to_string()]);
    }

    #[test]
    fn apt_ignores_non_install_subcommands() {
        for subcommand in ["update", "upgrade", "remove", "autoremove", "purge"] {
            let targets = parse_install_targets(BackendKind::Apt, &args(&[subcommand, "git"]));
            assert!(targets.is_empty(), "expected empty for {subcommand}");
        }
    }

    #[test]
    fn apt_install_excludes_deb_file() {
        let targets = parse_install_targets(BackendKind::Apt, &args(&["install", "./foo.deb"]));
        assert!(targets.is_empty());
    }

    #[test]
    fn apt_install_excludes_url() {
        let targets = parse_install_targets(
            BackendKind::Apt,
            &args(&["install", "https://example.com/x.deb"]),
        );
        assert!(targets.is_empty());
    }

    #[test]
    fn apt_install_excludes_pinned_version() {
        let targets = parse_install_targets(BackendKind::Apt, &args(&["install", "git=1.2.3"]));
        assert!(targets.is_empty());
    }

    #[test]
    fn apt_install_excludes_path_with_slash() {
        let targets = parse_install_targets(BackendKind::Apt, &args(&["install", "ppa/git"]));
        assert!(targets.is_empty());
    }

    #[test]
    fn apt_install_mixed_targets_keeps_only_plain_names() {
        let targets = parse_install_targets(
            BackendKind::Apt,
            &args(&["install", "git", "./foo.deb", "curl", "pkg=1.0"]),
        );
        assert_eq!(targets, vec!["git".to_string(), "curl".to_string()]);
    }

    #[test]
    fn apt_install_with_no_subcommand_is_empty() {
        let targets = parse_install_targets(BackendKind::Apt, &args(&["-y"]));
        assert!(targets.is_empty());
    }

    #[test]
    fn pacman_sync_with_package_records() {
        let targets = parse_install_targets(BackendKind::Pacman, &args(&["-S", "foo"]));
        assert_eq!(targets, vec!["foo".to_string()]);
    }

    #[test]
    fn pacman_syu_with_package_records() {
        let targets = parse_install_targets(BackendKind::Pacman, &args(&["-Syu", "foo"]));
        assert_eq!(targets, vec!["foo".to_string()]);
    }

    #[test]
    fn pacman_syu_alone_is_ignored() {
        let targets = parse_install_targets(BackendKind::Pacman, &args(&["-Syu"]));
        assert!(targets.is_empty());
    }

    #[test]
    fn pacman_remove_is_ignored() {
        let targets = parse_install_targets(BackendKind::Pacman, &args(&["-R", "foo"]));
        assert!(targets.is_empty());
    }

    #[test]
    fn pacman_clean_is_ignored() {
        let targets = parse_install_targets(BackendKind::Pacman, &args(&["-Sc"]));
        assert!(targets.is_empty());
    }

    #[test]
    fn shim_kind_from_basename() {
        assert_eq!(ShimKind::from_basename("apt"), Some(ShimKind::Apt));
        assert_eq!(ShimKind::from_basename("apt-get"), Some(ShimKind::AptGet));
        assert_eq!(ShimKind::from_basename("pacman"), Some(ShimKind::Pacman));
        assert_eq!(ShimKind::from_basename("ls"), None);
    }
}
