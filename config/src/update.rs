// SPDX-License-Identifier: GPL-3.0-or-later

//! How much work a config change needs from the daemon.

use crate::Config;

/// Ordered so that a burst of changes can be merged with `max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Update {
    None,
    /// Re-render the existing surfaces.
    Content,
    /// Send anchor/margin/layer/lock requests to the existing surfaces.
    Surface,
    /// Recompute which surfaces exist.
    Rebuild,
}

/// The cheapest update that brings surfaces built for `old` in line with `new`.
#[must_use]
pub fn classify(old: &Config, new: &Config) -> Update {
    // Destructured so that adding a field without classifying it fails to compile.
    let Config {
        main_message,
        secondary_message,
        font_family,
        font_style,
        font_size,
        font_color,
        text_shadow,
        background_plate,
        corner,
        margin_x,
        margin_y,
        over_windows,
        show_on_lockscreen,
        monitors,
    } = new;

    if *monitors != old.monitors {
        Update::Rebuild
    } else if *corner != old.corner
        || *margin_x != old.margin_x
        || *margin_y != old.margin_y
        || *over_windows != old.over_windows
        || *show_on_lockscreen != old.show_on_lockscreen
    {
        Update::Surface
    } else if *main_message != old.main_message
        || *secondary_message != old.secondary_message
        || *font_family != old.font_family
        || *font_style != old.font_style
        || *font_size != old.font_size
        || *font_color != old.font_color
        || *text_shadow != old.text_shadow
        || *background_plate != old.background_plate
    {
        Update::Content
    } else {
        Update::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Corner, FontStyle, Monitors, Rgba};

    fn tier(change: impl FnOnce(&mut Config)) -> Update {
        let old = Config::default();
        let mut new = old.clone();
        change(&mut new);
        classify(&old, &new)
    }

    #[test]
    fn unchanged_is_none() {
        assert_eq!(tier(|_| {}), Update::None);
    }

    #[test]
    fn content_fields() {
        assert_eq!(tier(|c| c.main_message = "x".into()), Update::Content);
        assert_eq!(tier(|c| c.secondary_message = "x".into()), Update::Content);
        assert_eq!(tier(|c| c.font_family = "Serif".into()), Update::Content);
        assert_eq!(tier(|c| c.font_style = FontStyle::Italic), Update::Content);
        assert_eq!(tier(|c| c.font_size = 30), Update::Content);
        assert_eq!(
            tier(|c| c.font_color = Rgba::new(0.0, 0.0, 0.0, 1.0)),
            Update::Content
        );
        assert_eq!(tier(|c| c.text_shadow = true), Update::Content);
        assert_eq!(tier(|c| c.background_plate = true), Update::Content);
    }

    #[test]
    fn surface_fields() {
        assert_eq!(tier(|c| c.corner = Corner::TopLeft), Update::Surface);
        assert_eq!(tier(|c| c.margin_x = 10), Update::Surface);
        assert_eq!(tier(|c| c.margin_y = 10), Update::Surface);
        assert_eq!(tier(|c| c.over_windows = true), Update::Surface);
        assert_eq!(tier(|c| c.show_on_lockscreen = true), Update::Surface);
    }

    #[test]
    fn rebuild_fields() {
        assert_eq!(tier(|c| c.monitors = Monitors::All), Update::Rebuild);
    }

    #[test]
    fn several_changes_take_the_highest_tier() {
        assert_eq!(
            tier(|c| {
                c.main_message = "x".into();
                c.corner = Corner::TopLeft;
                c.monitors = Monitors::All;
            }),
            Update::Rebuild
        );
        assert_eq!(
            tier(|c| {
                c.font_size = 40;
                c.margin_x = 0;
            }),
            Update::Surface
        );
    }
}
