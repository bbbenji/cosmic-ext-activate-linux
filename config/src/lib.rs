// SPDX-License-Identifier: GPL-3.0-or-later

//! Settings shared by the Activate Linux daemon and settings app.

mod model;
mod placeholders;
mod system;
mod targets;
mod update;

pub use cosmic_config;
pub use model::*;
pub use placeholders::expand_placeholders;
pub use system::{SystemInfo, parse_os_name};
pub use targets::{Targets, resolve_targets};
pub use update::{Update, classify};

/// Settings app ID, also the cosmic-config ID.
pub const APP_ID: &str = "io.github.bbbenji.ActivateLinux";

/// Daemon ID: its D-Bus name and autostart entry name.
pub const DAEMON_ID: &str = "io.github.bbbenji.ActivateLinux.Daemon";

use cosmic_config::CosmicConfigEntry;

/// Open the user's config for this app, creating its directory if needed.
///
/// # Errors
/// If the config directory can't be found or created.
pub fn open() -> Result<cosmic_config::Config, cosmic_config::Error> {
    cosmic_config::Config::new(APP_ID, Config::VERSION)
}

/// Read the config. A field that is missing or fails to parse keeps its default.
#[must_use]
pub fn load(handle: &cosmic_config::Config) -> Config {
    Config::get_entry(handle).unwrap_or_else(|(errors, config)| {
        for why in errors {
            tracing::warn!(%why, "invalid config field, using its default");
        }
        config
    })
}

/// Flip `over_windows` and persist it.
///
/// # Errors
/// If the new value can't be written.
pub fn toggle_overlay(handle: &cosmic_config::Config) -> Result<bool, cosmic_config::Error> {
    let mut config = load(handle);
    let value = !config.over_windows;
    config.set_over_windows(handle, value)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_handle(dir: &tempfile::TempDir) -> cosmic_config::Config {
        cosmic_config::Config::with_custom_path(APP_ID, Config::VERSION, dir.path().to_path_buf())
            .unwrap()
    }

    #[test]
    fn load_of_an_empty_config_is_the_default() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(&temp_handle(&dir)), Config::default());
    }

    #[test]
    fn toggle_overlay_works_before_anything_was_written() {
        let dir = tempfile::tempdir().unwrap();
        let handle = temp_handle(&dir);
        assert!(toggle_overlay(&handle).unwrap());
        assert!(load(&handle).over_windows);
        assert!(!toggle_overlay(&handle).unwrap());
        assert!(!load(&handle).over_windows);
    }

    #[test]
    fn load_keeps_good_fields_when_one_is_corrupt() {
        let dir = tempfile::tempdir().unwrap();
        let handle = temp_handle(&dir);
        let mut c = Config::default();
        c.set_main_message(&handle, "Hello".into()).unwrap();
        let corner = dir
            .path()
            .join("cosmic")
            .join(APP_ID)
            .join("v1")
            .join("corner");
        std::fs::write(corner, "NotACorner").unwrap();

        let loaded = load(&handle);
        assert_eq!(loaded.main_message, "Hello");
        assert_eq!(loaded.corner, Corner::BottomRight);
    }
}
