use clap::Parser;
use clap::error::ErrorKind;

use super::{Cli, Command, InstallArgs};

#[test]
fn version_flags_print_package_version() {
    for flag in ["-v", "--version"] {
        let error = Cli::try_parse_from(["saya", flag])
            .err()
            .expect("version flags should exit after printing the version");

        assert_eq!(error.kind(), ErrorKind::DisplayVersion);
        assert_eq!(
            error.to_string(),
            format!("saya {}\n", env!("CARGO_PKG_VERSION"))
        );
    }
}

#[test]
fn install_accepts_no_package_arguments() {
    let cli = Cli::try_parse_from(["saya", "install"]).unwrap();

    assert!(matches!(
        cli.command,
        Command::Install(InstallArgs {
            yes: false,
            names,
            backend_args,
        })
            if names.is_empty() && backend_args.is_empty()
    ));
}

#[test]
fn install_accepts_yes_before_package_names() {
    let cli = Cli::try_parse_from(["saya", "install", "-y", "openssh-server"]).unwrap();

    assert!(matches!(
        cli.command,
        Command::Install(InstallArgs {
            yes: true,
            names,
            backend_args,
        }) if names == ["openssh-server"] && backend_args.is_empty()
    ));
}

#[test]
fn install_accepts_multiple_package_names() {
    let cli = Cli::try_parse_from(["saya", "install", "adb", "fastboot"]).unwrap();

    assert!(matches!(
        cli.command,
        Command::Install(InstallArgs {
            yes: false,
            names,
            backend_args,
        })
            if names == ["adb", "fastboot"] && backend_args.is_empty()
    ));
}

#[test]
fn install_accepts_backend_arguments_after_separator() {
    let cli = Cli::try_parse_from([
        "saya",
        "install",
        "neovim",
        "--",
        "--config",
        "/tmp/yay.conf",
    ])
    .unwrap();

    assert!(matches!(
        cli.command,
        Command::Install(InstallArgs {
            yes: false,
            names,
            backend_args,
        })
            if names == ["neovim"] && backend_args == ["--config", "/tmp/yay.conf"]
    ));
}

#[test]
fn install_accepts_bulk_packages_with_asexplicit() {
    let packages = [
        "accountsservice",
        "archlinux-wallpaper",
        "bibata-cursor-theme",
        "bluez",
        "brightnessctl",
        "fcitx5",
        "fcitx5-gtk",
        "fcitx5-mozc",
        "fcitx5-qt",
        "gnome-keyring",
        "hyprland",
        "kitty",
        "loupe",
        "nautilus",
        "networkmanager",
        "noctalia",
        "noctalia-greeter",
        "pavucontrol",
        "pipewire-pulse",
        "ttf-jetbrains-mono-nerd",
        "upower",
        "wl-clipboard",
        "wireplumber",
        "xdg-desktop-portal-gtk",
        "xdg-desktop-portal-hyprland",
    ];
    let args = ["saya", "install", "-y"]
        .into_iter()
        .chain(packages)
        .chain(["--", "--asexplicit"]);
    let cli = Cli::try_parse_from(args).unwrap();

    assert!(matches!(
        cli.command,
        Command::Install(InstallArgs {
            yes: true,
            names,
            backend_args,
        }) if names == packages && backend_args == ["--asexplicit"]
    ));
}

#[test]
fn install_requires_separator_before_backend_arguments() {
    assert!(Cli::try_parse_from(["saya", "install", "neovim", "-C"]).is_err());
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
