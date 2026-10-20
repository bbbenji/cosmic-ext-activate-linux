// SPDX-License-Identifier: GPL-3.0-or-later

//! "Start at login" through a user override of the system autostart entry.
//! Disabling writes `Hidden=true` to `~/.config/autostart/<id>.desktop`, which
//! XDG autostart treats as removing the system entry; enabling deletes it.

use std::io;
use std::path::PathBuf;

pub const FILE_NAME: &str = "io.github.bbbenji.ActivateLinux.Daemon.desktop";

const DISABLED_OVERRIDE: &str = "[Desktop Entry]\n\
Type=Application\n\
Name=Activate Linux watermark\n\
Exec=cosmic-ext-activate-linux-daemon\n\
Hidden=true\n";

#[derive(Debug, Clone)]
pub struct Autostart {
    dir: PathBuf,
}

impl Autostart {
    /// The current user's autostart directory (`$XDG_CONFIG_HOME/autostart`).
    #[must_use]
    pub fn user() -> Option<Self> {
        dirs::config_dir().map(|dir| Self::in_dir(dir.join("autostart")))
    }

    #[must_use]
    pub fn in_dir(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn path(&self) -> PathBuf {
        self.dir.join(FILE_NAME)
    }

    /// On unless a user override sets `Hidden=true`.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        std::fs::read_to_string(self.path()).map_or(true, |contents| {
            !contents.lines().any(|line| line.trim() == "Hidden=true")
        })
    }

    /// # Errors
    /// If the override can't be written or removed.
    pub fn set_enabled(&self, enabled: bool) -> io::Result<()> {
        if enabled {
            match std::fs::remove_file(self.path()) {
                Err(why) if why.kind() == io::ErrorKind::NotFound => Ok(()),
                result => result,
            }
        } else {
            std::fs::create_dir_all(&self.dir)?;
            std::fs::write(self.path(), DISABLED_OVERRIDE)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_when_no_override_exists() {
        let dir = tempfile::tempdir().unwrap();
        assert!(Autostart::in_dir(dir.path().join("autostart")).is_enabled());
    }

    #[test]
    fn disable_then_enable() {
        let dir = tempfile::tempdir().unwrap();
        let a = Autostart::in_dir(dir.path().join("autostart"));

        a.set_enabled(false).unwrap();
        assert!(!a.is_enabled());
        let written =
            std::fs::read_to_string(dir.path().join("autostart").join(FILE_NAME)).unwrap();
        assert!(written.lines().any(|l| l == "Hidden=true"));

        a.set_enabled(true).unwrap();
        assert!(a.is_enabled());
        assert!(!dir.path().join("autostart").join(FILE_NAME).exists());
    }

    #[test]
    fn enabling_twice_is_fine() {
        let dir = tempfile::tempdir().unwrap();
        let a = Autostart::in_dir(dir.path().to_path_buf());
        a.set_enabled(true).unwrap();
        a.set_enabled(true).unwrap();
        assert!(a.is_enabled());
    }

    #[test]
    fn override_without_hidden_counts_as_enabled() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(FILE_NAME),
            "[Desktop Entry]\nHidden=false\n",
        )
        .unwrap();
        assert!(Autostart::in_dir(dir.path().to_path_buf()).is_enabled());
    }
}
