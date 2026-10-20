// SPDX-License-Identifier: GPL-3.0-or-later

//! Command-line parsing. Small enough that a parser crate isn't worth it.

pub const USAGE: &str = "\
Usage: cosmic-ext-activate-linux-daemon [COMMAND]

Draws the Activate Linux watermark. With no command, runs the watermark.

Commands:
  toggle-overlay   Flip \"show over all windows\" and exit
                   (bind this to a custom shortcut, e.g. Super+Alt+W)

Options:
  -h, --help       Print this help
  -V, --version    Print the version";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Run,
    ToggleOverlay,
    Help,
    Version,
}

/// Parse the arguments after the program name.
///
/// # Errors
/// A message naming the first argument that isn't understood.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut args = args.into_iter();
    let command = match args.next().as_deref() {
        None => Command::Run,
        Some("toggle-overlay") => Command::ToggleOverlay,
        Some("-h" | "--help") => Command::Help,
        Some("-V" | "--version") => Command::Version,
        Some(other) => return Err(format!("unknown argument: {other}")),
    };
    match args.next() {
        None => Ok(command),
        Some(extra) => Err(format!("unexpected argument: {extra}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<Command, String> {
        parse(args.iter().map(|s| (*s).to_owned()))
    }

    #[test]
    fn commands() {
        assert_eq!(p(&[]), Ok(Command::Run));
        assert_eq!(p(&["toggle-overlay"]), Ok(Command::ToggleOverlay));
        assert_eq!(p(&["-h"]), Ok(Command::Help));
        assert_eq!(p(&["--help"]), Ok(Command::Help));
        assert_eq!(p(&["-V"]), Ok(Command::Version));
        assert_eq!(p(&["--version"]), Ok(Command::Version));
    }

    #[test]
    fn rejects_unknown_and_extra_arguments() {
        assert_eq!(p(&["bogus"]), Err("unknown argument: bogus".into()));
        assert_eq!(
            p(&["toggle-overlay", "now"]),
            Err("unexpected argument: now".into())
        );
    }
}
