// SPDX-License-Identifier: GPL-3.0-or-later

//! Facts about the running system that placeholders can show.

/// Values substituted into the `{{...}}` placeholders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemInfo {
    pub os_name: String,
    pub kernel: String,
    pub desktop: String,
    pub session_type: String,
}

const OS_RELEASE_PATHS: [&str; 2] = ["/etc/os-release", "/usr/lib/os-release"];

impl SystemInfo {
    /// Read the facts from the running system. Called once at startup.
    #[must_use]
    pub fn detect() -> Self {
        let os_release = OS_RELEASE_PATHS
            .iter()
            .find_map(|path| std::fs::read_to_string(path).ok());
        let kernel = rustix::system::uname()
            .release()
            .to_str()
            .ok()
            .map(str::to_owned);

        Self::from_sources(
            os_release.as_deref(),
            kernel,
            std::env::var("XDG_CURRENT_DESKTOP").ok(),
            std::env::var("XDG_SESSION_TYPE").ok(),
        )
    }

    /// Build from raw sources; missing or empty values get the defaults.
    #[must_use]
    pub fn from_sources(
        os_release: Option<&str>,
        kernel: Option<String>,
        desktop: Option<String>,
        session_type: Option<String>,
    ) -> Self {
        let or = |value: Option<String>, default: &str| {
            value
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| default.to_owned())
        };
        Self {
            os_name: or(os_release.and_then(parse_os_name), "Linux"),
            kernel: or(kernel, "Unknown kernel"),
            desktop: or(desktop, "COSMIC"),
            session_type: or(session_type, "Wayland/X11"),
        }
    }
}

/// The distribution's display name from os-release content: `PRETTY_NAME`, else `NAME`.
#[must_use]
pub fn parse_os_name(os_release: &str) -> Option<String> {
    let value_of = |key: &str| {
        os_release.lines().find_map(|line| {
            let (k, v) = line.trim().split_once('=')?;
            if k != key {
                return None;
            }
            let v = v.trim();
            let v = v
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .or_else(|| v.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                .unwrap_or(v);
            (!v.is_empty()).then(|| v.to_owned())
        })
    };
    value_of("PRETTY_NAME").or_else(|| value_of("NAME"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_name_wins_over_name() {
        let text = "NAME=\"Arch\"\nPRETTY_NAME=\"Arch Linux\"\nID=arch\n";
        assert_eq!(parse_os_name(text).as_deref(), Some("Arch Linux"));
    }

    #[test]
    fn unquoted_and_single_quoted_values() {
        assert_eq!(parse_os_name("NAME=Fedora").as_deref(), Some("Fedora"));
        assert_eq!(
            parse_os_name("PRETTY_NAME='Debian GNU/Linux 13'").as_deref(),
            Some("Debian GNU/Linux 13")
        );
    }

    #[test]
    fn empty_pretty_name_falls_back_to_name() {
        assert_eq!(
            parse_os_name("PRETTY_NAME=\"\"\nNAME=\"Pop!_OS\"").as_deref(),
            Some("Pop!_OS")
        );
    }

    #[test]
    fn comments_and_garbage_are_ignored() {
        assert_eq!(
            parse_os_name("# PRETTY_NAME=nope\n\nnot a pair\nNAME=x").as_deref(),
            Some("x")
        );
        assert_eq!(parse_os_name("ID=arch"), None);
        assert_eq!(parse_os_name(""), None);
    }

    #[test]
    fn missing_sources_use_defaults() {
        let info = SystemInfo::from_sources(None, None, None, Some(String::new()));
        assert_eq!(info.os_name, "Linux");
        assert_eq!(info.kernel, "Unknown kernel");
        assert_eq!(info.desktop, "COSMIC");
        assert_eq!(info.session_type, "Wayland/X11");
    }

    #[test]
    fn present_sources_are_used() {
        let info = SystemInfo::from_sources(
            Some("NAME=CachyOS"),
            Some("7.2.11-1-cachyos".into()),
            Some("COSMIC".into()),
            Some("wayland".into()),
        );
        assert_eq!(info.os_name, "CachyOS");
        assert_eq!(info.kernel, "7.2.11-1-cachyos");
        assert_eq!(info.session_type, "wayland");
    }
}
