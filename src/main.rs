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
        cli::Command::Install(args) => {
            let backend = backend::detect_backend()?;
            match args.name {
                None => {
                    let manifest = manifest::Manifest::load(&path)?;
                    commands::install::run_missing(&manifest, backend.as_ref())
                }
                Some(name) => {
                    let mut manifest = manifest::Manifest::load(&path)?;
                    commands::install::run_named(
                        &mut manifest,
                        &name,
                        backend.as_ref(),
                        &path,
                        &user,
                    )
                }
            }
        }
        cli::Command::Status => {
            let manifest = manifest::Manifest::load(&path)?;
            let backend = backend::detect_backend()?;
            commands::status::run(&manifest, backend.as_ref())
        }
        cli::Command::Uninstall(args) => {
            let mut manifest = manifest::Manifest::load(&path)?;
            let backend = backend::detect_backend()?;
            commands::uninstall::run(&mut manifest, &args.name, backend.as_ref(), &path, &user)
        }
        cli::Command::Import(args) => {
            let mut manifest = manifest::Manifest::load(&path)?;
            let backend = backend::detect_backend()?;
            commands::import::run(&args, &mut manifest, backend.as_ref(), &path, &user)
        }
    }
}
