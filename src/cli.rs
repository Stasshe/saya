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
    /// Install every package listed in the manifest that isn't installed yet.
    Install,
    /// Show which manifest packages are installed/missing without installing.
    Status,
    /// Install a package and record it in the manifest on success.
    Add(AddArgs),
    /// Remove a package from the manifest.
    Forget(ForgetArgs),
    /// List manually-installed packages not yet in the manifest.
    Import(ImportArgs),
}

#[derive(Args)]
pub struct AddArgs {
    /// Logical package name, e.g. "neovim".
    #[arg(value_parser = parse_package_name)]
    pub logical: String,
    /// Real apt package name(s), if different from the logical name.
    #[arg(long = "apt", value_parser = parse_package_name)]
    pub apt: Vec<String>,
    /// Real pacman package name(s), if different from the logical name.
    #[arg(long = "pacman", value_parser = parse_package_name)]
    pub pacman: Vec<String>,
}

#[derive(Args)]
pub struct ForgetArgs {
    /// Logical package name to remove from the manifest.
    #[arg(value_parser = parse_package_name)]
    pub logical: String,
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

        assert!(matches!(cli.command, Command::Install));
        assert!(Cli::try_parse_from(["saya", "install", "git"]).is_err());
    }

    #[test]
    fn apply_command_is_not_available() {
        assert!(Cli::try_parse_from(["saya", "apply"]).is_err());
    }
}
