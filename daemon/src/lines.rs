// SPDX-License-Identifier: GPL-3.0-or-later

//! The text the watermark shows, after placeholder expansion.

use activate_linux_config::{Config, SystemInfo, expand_placeholders};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lines {
    pub main: String,
    pub secondary: String,
}

/// Expanded text, or `None` when both lines would be blank, so nothing
/// (not even the background plate) is drawn.
#[must_use]
pub fn lines(config: &Config, info: &SystemInfo) -> Option<Lines> {
    let main = expand_placeholders(&config.main_message, info);
    let secondary = expand_placeholders(&config.secondary_message, info);
    if main.trim().is_empty() && secondary.trim().is_empty() {
        None
    } else {
        Some(Lines { main, secondary })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> SystemInfo {
        SystemInfo::from_sources(Some("NAME=Arch"), None, None, Some("wayland".into()))
    }

    #[test]
    fn expands_both_lines() {
        let c = Config {
            main_message: "Activate {{OS}}".into(),
            ..Config::default()
        };
        let l = lines(&c, &info()).unwrap();
        assert_eq!(l.main, "Activate Arch");
        assert_eq!(l.secondary, "Go to Settings to activate COSMIC.");
    }

    #[test]
    fn nothing_to_show_when_both_lines_are_blank() {
        let c = Config {
            main_message: "  ".into(),
            secondary_message: String::new(),
            ..Config::default()
        };
        assert_eq!(lines(&c, &info()), None);
    }

    #[test]
    fn nothing_to_show_when_placeholders_expand_to_nothing() {
        let c = Config {
            main_message: "{{X11}}".into(),
            secondary_message: String::new(),
            ..Config::default()
        };
        assert_eq!(lines(&c, &info()), None);
    }

    #[test]
    fn one_blank_line_is_fine() {
        let c = Config {
            secondary_message: String::new(),
            ..Config::default()
        };
        let l = lines(&c, &info()).unwrap();
        assert_eq!(l.secondary, "");
    }
}
