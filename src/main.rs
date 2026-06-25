mod backend;
mod cli;
mod commands;
mod manifest;
mod privilege;

use anyhow::Result;
use clap::Parser;

fn main() {
    if let Err(e) = run_saya_cli() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run_saya_cli() -> Result<()> {
    let cli = cli::Cli::parse();

    let user = privilege::resolve_invocation_user()?;
    let path = commands::manifest_path(&user.home);

    match cli.command {
        cli::Command::SelfUpdate => commands::self_update::run(),
        cli::Command::Update => {
            let backend = backend::detect_backend()?;
            backend.update()
        }
        cli::Command::Upgrade => {
            let backend = backend::detect_backend()?;
            backend.upgrade()
        }
        cli::Command::Apply => {
            let manifest = manifest::Manifest::load(&path)?;
            let backend = backend::detect_backend()?;
            commands::apply::run(&manifest, backend.as_ref())
        }
        cli::Command::Status => {
            let manifest = manifest::Manifest::load(&path)?;
            let backend = backend::detect_backend()?;
            commands::status::run(&manifest, backend.as_ref())
        }
        cli::Command::Install(args) => {
            let mut manifest = manifest::Manifest::load(&path)?;
            let backend = backend::detect_backend()?;
            commands::install::run(&mut manifest, &args, backend.as_ref(), &path, &user)
        }
        cli::Command::Add(args) => {
            let mut manifest = manifest::Manifest::load(&path)?;
            commands::add::run_add(&mut manifest, &args, &path, &user)
        }
        cli::Command::Forget(args) => {
            let mut manifest = manifest::Manifest::load(&path)?;
            commands::add::run_forget(&mut manifest, &args.logical, &path, &user)
        }
        cli::Command::Import(args) => {
            let mut manifest = manifest::Manifest::load(&path)?;
            let backend = backend::detect_backend()?;
            commands::import::run(&args, &mut manifest, backend.as_ref(), &path, &user)
        }
    }
}
