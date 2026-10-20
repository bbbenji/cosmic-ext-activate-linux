// SPDX-License-Identifier: GPL-3.0-or-later

//! Expansion of the `{{...}}` placeholders documented in the README.

use crate::SystemInfo;

/// Replace `{{OS}}`, `{{KERNEL}}`, `{{DE}}`, `{{WAYLAND_X11}}`, `{{WAYLAND}}` and
/// `{{X11}}` (any case). Anything else between braces is left as written.
#[must_use]
pub fn expand_placeholders(text: &str, info: &SystemInfo) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];

        let value = after
            .find("}}")
            .and_then(|end| Some((value_for(&after[..end], info)?, end)));

        if let Some((value, end)) = value {
            out.push_str(&value);
            rest = &after[end + 2..];
        } else {
            // Not a known token: keep the braces and rescan what follows them,
            // so "{{ {{OS}}" still expands the second token.
            out.push_str("{{");
            rest = after;
        }
    }

    out.push_str(rest);
    out
}

fn value_for(name: &str, info: &SystemInfo) -> Option<String> {
    let session = info.session_type.to_ascii_lowercase();
    let value = match name.to_ascii_uppercase().as_str() {
        "OS" => info.os_name.clone(),
        "KERNEL" => info.kernel.clone(),
        "DE" => info.desktop.clone(),
        "WAYLAND_X11" => info.session_type.clone(),
        "WAYLAND" if session == "wayland" => "Wayland".into(),
        "X11" if session == "x11" => "X11".into(),
        "WAYLAND" | "X11" => String::new(),
        _ => return None,
    };
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(session: &str) -> SystemInfo {
        SystemInfo {
            os_name: "Arch Linux".into(),
            kernel: "6.18.1".into(),
            desktop: "COSMIC".into(),
            session_type: session.into(),
        }
    }

    #[test]
    fn every_token() {
        let i = info("wayland");
        assert_eq!(
            expand_placeholders("Activate {{OS}}", &i),
            "Activate Arch Linux"
        );
        assert_eq!(expand_placeholders("{{KERNEL}}", &i), "6.18.1");
        assert_eq!(expand_placeholders("{{DE}}", &i), "COSMIC");
        assert_eq!(expand_placeholders("{{WAYLAND_X11}}", &i), "wayland");
        assert_eq!(expand_placeholders("{{WAYLAND}}", &i), "Wayland");
        assert_eq!(expand_placeholders("{{X11}}", &i), "");
    }

    #[test]
    fn x11_session() {
        let i = info("x11");
        assert_eq!(expand_placeholders("{{X11}}", &i), "X11");
        assert_eq!(expand_placeholders("{{WAYLAND}}", &i), "");
    }

    #[test]
    fn tokens_are_case_insensitive() {
        assert_eq!(
            expand_placeholders("{{os}} {{Kernel}} {{wayland_x11}}", &info("wayland")),
            "Arch Linux 6.18.1 wayland"
        );
    }

    #[test]
    fn wayland_x11_is_not_read_as_wayland() {
        assert_eq!(
            expand_placeholders("[{{WAYLAND_X11}}][{{WAYLAND}}]", &info("wayland")),
            "[wayland][Wayland]"
        );
    }

    #[test]
    fn unknown_and_malformed_tokens_are_kept() {
        let i = info("wayland");
        assert_eq!(
            expand_placeholders("{{FOO}} {{OS}}", &i),
            "{{FOO}} Arch Linux"
        );
        assert_eq!(expand_placeholders("{{OS", &i), "{{OS");
        assert_eq!(expand_placeholders("{{ {{OS}}", &i), "{{ Arch Linux");
        assert_eq!(expand_placeholders("}}{{", &i), "}}{{");
    }

    #[test]
    fn plain_and_empty_text() {
        let i = info("wayland");
        assert_eq!(expand_placeholders("", &i), "");
        assert_eq!(expand_placeholders("Zażółć ✨", &i), "Zażółć ✨");
    }
}
