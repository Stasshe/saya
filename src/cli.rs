use clap::{Args, Parser, Subcommand};

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
    /// Install every package listed in the manifest that isn't installed yet.
    Apply,
    /// Show which manifest packages are installed/missing without installing.
    Status,
    /// Record a package in the manifest.
    Add(AddArgs),
    /// Remove a package from the manifest.
    Forget(ForgetArgs),
    /// List manually-installed packages not yet in the manifest.
    Import(ImportArgs),
    /// Manage the apt/apt-get/pacman shims under /usr/local/bin.
    Capture(CaptureArgs),
    /// Verify shim symlinks and PATH ordering are healthy.
    Doctor,
}

#[derive(Args)]
pub struct AddArgs {
    /// Logical package name, e.g. "neovim".
    pub logical: String,
    /// Real apt package name(s), if different from the logical name.
    #[arg(long = "apt")]
    pub apt: Vec<String>,
    /// Real pacman package name(s), if different from the logical name.
    #[arg(long = "pacman")]
    pub pacman: Vec<String>,
}

#[derive(Args)]
pub struct ForgetArgs {
    /// Logical package name to remove from the manifest.
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

#[derive(Args)]
pub struct CaptureArgs {
    #[command(subcommand)]
    pub action: CaptureAction,
}

#[derive(Subcommand)]
pub enum CaptureAction {
    /// Install the apt/apt-get/pacman shim symlinks.
    Enable,
    /// Remove the shim symlinks saya owns.
    Disable,
}
