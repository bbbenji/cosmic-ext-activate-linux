// SPDX-License-Identifier: GPL-3.0-or-later

//! Turning the monitor preference into concrete surfaces.

use crate::Monitors;

/// Where watermark surfaces should exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Targets {
    /// One surface; the compositor puts it on the active output.
    Automatic,
    /// One surface per named output.
    Outputs(Vec<String>),
}

/// Resolve the preference against the outputs that currently exist.
#[must_use]
pub fn resolve_targets(pref: &Monitors, available: &[String]) -> Targets {
    match pref {
        Monitors::All if !available.is_empty() => Targets::Outputs(available.to_vec()),
        Monitors::Output(name) if available.contains(name) => Targets::Outputs(vec![name.clone()]),
        // A missing output or an empty list falls back to Automatic, mirroring
        // the GNOME version's index fallback.
        Monitors::Automatic | Monitors::All | Monitors::Output(_) => Targets::Automatic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn automatic_ignores_outputs() {
        assert_eq!(
            resolve_targets(&Monitors::Automatic, &names(&["DP-1"])),
            Targets::Automatic
        );
    }

    #[test]
    fn all_lists_every_output() {
        let available = names(&["DP-1", "HDMI-A-1"]);
        assert_eq!(
            resolve_targets(&Monitors::All, &available),
            Targets::Outputs(available.clone())
        );
    }

    #[test]
    fn all_without_outputs_falls_back() {
        assert_eq!(resolve_targets(&Monitors::All, &[]), Targets::Automatic);
    }

    #[test]
    fn named_output_present() {
        let pref = Monitors::Output("HDMI-A-1".into());
        assert_eq!(
            resolve_targets(&pref, &names(&["DP-1", "HDMI-A-1"])),
            Targets::Outputs(names(&["HDMI-A-1"]))
        );
    }

    #[test]
    fn named_output_missing_falls_back() {
        let pref = Monitors::Output("DP-9".into());
        assert_eq!(
            resolve_targets(&pref, &names(&["DP-1"])),
            Targets::Automatic
        );
        assert_eq!(resolve_targets(&pref, &[]), Targets::Automatic);
    }
}
