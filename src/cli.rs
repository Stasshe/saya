use clap::{Args, Parser, Subcommand};

use crate::manifest::validate_package_name;

#[derive(Parser)]
#[command(
    name = "saya",
    about = "Thin one-way sync wrapper around your OS package manager"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Update this saya binary from the latest GitHub Release.
    SelfUpdate,
    /// Update package manager metadata.
    Update,
    /// Upgrade installed packages with the detected OS package manager.
    Upgrade,
    /// With no name: install every package listed in the manifest that
    /// isn't installed yet. With a name: install that package and record
    /// it in the manifest on success.
    Install(InstallArgs),
    /// Show which manifest packages are installed/missing without installing.
    Status,
    /// Uninstall a package and remove it from the manifest.
    Uninstall(UninstallArgs),
    /// List manually-installed packages not yet in the manifest.
    Import(ImportArgs),
}

#[derive(Args)]
pub struct InstallArgs {
    /// Package name as known to the detected backend, e.g. "neovim".
    /// Omit to install everything missing from the manifest.
    #[arg(value_parser = parse_package_name)]
    pub name: Option<String>,
}

#[derive(Args)]
pub struct UninstallArgs {
    /// Package name to uninstall and remove from the manifest.
    #[arg(value_parser = parse_package_name)]
    pub name: String,
}

#[derive(Args)]
pub struct ImportArgs {
    /// Source of candidates: currently only the manual-install list.
    #[arg(long)]
    pub manual: bool,
    /// Open an editor to review/edit before saving, instead of just printing.
    #[arg(long)]
    pub edit: bool,
}

fn parse_package_name(value: &str) -> Result<String, String> {
    validate_package_name(value)?;
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_accepts_no_package_arguments() {
        let cli = Cli::try_parse_from(["saya", "install"]).unwrap();

        assert!(matches!(
            cli.command,
            Command::Install(InstallArgs { name: None })
        ));
    }

    #[test]
    fn install_accepts_a_package_name() {
        let cli = Cli::try_parse_from(["saya", "install", "git"]).unwrap();

        assert!(matches!(
            cli.command,
            Command::Install(InstallArgs { name: Some(name) }) if name == "git"
        ));
    }

    #[test]
    fn uninstall_requires_a_package_name() {
        assert!(Cli::try_parse_from(["saya", "uninstall"]).is_err());
        assert!(Cli::try_parse_from(["saya", "uninstall", "git"]).is_ok());
    }

    #[test]
    fn apply_command_is_not_available() {
        assert!(Cli::try_parse_from(["saya", "apply"]).is_err());
    }
}
