// SPDX-License-Identifier: GPL-3.0-or-later

//! Deciding which surfaces to create and destroy.

use activate_linux_config::Targets;

/// What a surface is for.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Slot {
    Automatic,
    Output(String),
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub destroy: Vec<Slot>,
    pub create: Vec<Slot>,
}

#[must_use]
pub fn wanted(targets: &Targets) -> Vec<Slot> {
    match targets {
        Targets::Automatic => vec![Slot::Automatic],
        Targets::Outputs(names) => names.iter().cloned().map(Slot::Output).collect(),
    }
}

/// Surfaces to destroy (exist but not wanted) and create (wanted but missing).
#[must_use]
pub fn plan(wanted: &[Slot], existing: &[Slot]) -> Plan {
    Plan {
        destroy: existing
            .iter()
            .filter(|s| !wanted.contains(s))
            .cloned()
            .collect(),
        create: wanted
            .iter()
            .filter(|s| !existing.contains(s))
            .cloned()
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out(name: &str) -> Slot {
        Slot::Output(name.into())
    }

    #[test]
    fn wanted_slots() {
        assert_eq!(wanted(&Targets::Automatic), vec![Slot::Automatic]);
        assert_eq!(
            wanted(&Targets::Outputs(vec!["DP-1".into()])),
            vec![out("DP-1")]
        );
    }

    #[test]
    fn first_start_creates_everything() {
        let p = plan(&[out("DP-1"), out("DP-2")], &[]);
        assert_eq!(
            p,
            Plan {
                destroy: vec![],
                create: vec![out("DP-1"), out("DP-2")]
            }
        );
    }

    #[test]
    fn keeps_existing_surfaces() {
        assert_eq!(
            plan(&[Slot::Automatic], &[Slot::Automatic]),
            Plan::default()
        );
    }

    #[test]
    fn switching_from_all_to_one_output() {
        let p = plan(&[out("DP-2")], &[out("DP-1"), out("DP-2")]);
        assert_eq!(
            p,
            Plan {
                destroy: vec![out("DP-1")],
                create: vec![]
            }
        );
    }

    #[test]
    fn closed_automatic_surface_is_recreated() {
        // The compositor closed the only surface when its output was unplugged;
        // the daemon forgets it, so the plan must create a new one.
        assert_eq!(
            plan(&[Slot::Automatic], &[]),
            Plan {
                destroy: vec![],
                create: vec![Slot::Automatic]
            }
        );
    }

    #[test]
    fn unplugged_named_output_falls_back_to_automatic() {
        let p = plan(&[Slot::Automatic], &[out("DP-9")]);
        assert_eq!(
            p,
            Plan {
                destroy: vec![out("DP-9")],
                create: vec![Slot::Automatic]
            }
        );
    }
}
