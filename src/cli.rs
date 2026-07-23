use clap::{ArgAction, Args, Parser, Subcommand};

use crate::manifest::validate_package_name;

#[derive(Parser)]
#[command(
    name = "saya",
    about = "Thin one-way sync wrapper around your OS package manager",
    version,
    disable_version_flag = true
)]
pub struct Cli {
    /// Print version.
    #[arg(short = 'v', long = "version", action = ArgAction::Version)]
    pub version: Option<bool>,
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
    /// Accept the familiar package-manager confirmation flag.
    #[arg(short = 'y')]
    pub yes: bool,
    /// Package names as known to the detected backend, e.g. "neovim".
    /// Omit to install everything missing from the manifest.
    #[arg(value_name = "PACKAGE", value_parser = parse_package_name)]
    pub names: Vec<String>,
    /// Arguments passed unchanged to apt-get or yay. Must follow `--`.
    #[arg(last = true, value_name = "ARG", allow_hyphen_values = true)]
    pub backend_args: Vec<String>,
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
mod tests;
