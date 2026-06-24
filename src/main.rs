mod backend;
mod capture;
mod cli;
mod commands;
mod manifest;
mod privilege;
mod shim;

use std::path::Path;

use anyhow::Result;
use clap::Parser;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let basename = Path::new(&args[0])
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if let Some(kind) = shim::ShimKind::from_basename(basename) {
        match shim::run(kind, &args[1..]) {
            Ok(code) => std::process::exit(code),
            Err(e) => {
                eprintln!("saya shim error: {e:#}");
                std::process::exit(1);
            }
        }
    }

    if let Err(e) = run_saya_cli() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run_saya_cli() -> Result<()> {
    let cli = cli::Cli::parse();

    let user = privilege::resolve_original_user()?;
    let path = commands::manifest_path(&user.home);

    match cli.command {
        cli::Command::Apply => {
            privilege::require_root()?;
            let manifest = manifest::Manifest::load(&path)?;
            let backend = backend::detect_backend()?;
            commands::apply::run(&manifest, backend.as_ref())
        }
        cli::Command::Status => {
            let manifest = manifest::Manifest::load(&path)?;
            let backend = backend::detect_backend()?;
            commands::status::run(&manifest, backend.as_ref())
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
        cli::Command::Capture(args) => match args.action {
            cli::CaptureAction::Enable => {
                privilege::require_root()?;
                capture::enable()
            }
            cli::CaptureAction::Disable => {
                privilege::require_root()?;
                capture::disable()
            }
        },
        cli::Command::Doctor => {
            let report = capture::doctor()?;
            print_doctor_report(&report);
            Ok(())
        }
    }
}

fn print_doctor_report(report: &capture::DoctorReport) {
    for shim in &report.shims {
        println!(
            "{:<10} symlink={} real_binary={}",
            shim.name,
            if shim.symlink_ok { "ok" } else { "MISSING" },
            if shim.real_binary_exists {
                "ok"
            } else {
                "MISSING"
            }
        );
    }
    println!(
        "PATH order: {}",
        if report.path_local_bin_first {
            "ok"
        } else {
            "MISSING (/usr/local/bin not before /usr/bin)"
        }
    );
    if !report.all_ok() {
        println!("\nrun `sudo saya capture enable` to fix");
    }
}
