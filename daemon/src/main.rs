// SPDX-License-Identifier: GPL-3.0-or-later

mod app;
mod cli;
mod fonts;
mod layout;
mod lines;
mod outputs;
mod reconcile;
mod view;

use std::process::ExitCode;

use cli::Command;

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    match cli::parse(std::env::args().skip(1)) {
        Err(message) => {
            eprintln!("{message}\n\n{}", cli::USAGE);
            ExitCode::from(2)
        }
        Ok(Command::Help) => {
            println!("{}", cli::USAGE);
            ExitCode::SUCCESS
        }
        Ok(Command::Version) => {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Command::ToggleOverlay) => toggle_overlay(),
        Ok(Command::Run) => run(),
    }
}

/// Flip the overlay setting without touching the GUI or Wayland.
fn toggle_overlay() -> ExitCode {
    let result = activate_linux_config::open()
        .and_then(|handle| activate_linux_config::toggle_overlay(&handle));
    match result {
        Ok(on) => {
            println!("show over all windows: {}", if on { "on" } else { "off" });
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("failed to update the config: {why}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> ExitCode {
    // libcosmic gives no clean "no layer-shell" signal, so refuse to start
    // outside a Wayland session, where there can't be one.
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        tracing::error!(
            "not a Wayland session (WAYLAND_DISPLAY is unset); the watermark needs COSMIC's layer-shell"
        );
        return ExitCode::FAILURE;
    }
    match app::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            tracing::error!(%why, "the watermark stopped");
            ExitCode::FAILURE
        }
    }
}
