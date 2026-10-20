// SPDX-License-Identifier: GPL-3.0-or-later

//! Layer-surface placement derived from the config.

use activate_linux_config::{Config, Corner};
use cosmic::iced::Limits;
use cosmic::iced::platform_specific::shell::commands::layer_surface::{Anchor, Layer};
use cosmic::iced::runtime::platform_specific::wayland::layer_surface::IcedMargin;

/// The layer-shell namespace the compositor sees.
pub const NAMESPACE: &str = "activate-linux";

/// Upper bound for the surface size. libcosmic's default (1920×1080) would clip
/// long messages or big fonts on large displays.
const MAX_SURFACE_SIZE: f32 = 8192.0;

#[must_use]
pub fn anchor(corner: Corner) -> Anchor {
    let vertical = if corner.is_top() {
        Anchor::TOP
    } else {
        Anchor::BOTTOM
    };
    let horizontal = if corner.is_left() {
        Anchor::LEFT
    } else {
        Anchor::RIGHT
    };
    vertical | horizontal
}

/// Margins on the two anchored edges; the others are ignored by the compositor
/// but are zeroed anyway so the request is unambiguous.
#[must_use]
pub fn margin(config: &Config) -> IcedMargin {
    // sanitized() caps margins at MARGIN_MAX, far below i32::MAX.
    let x = i32::try_from(config.margin_x).unwrap_or(i32::MAX);
    let y = i32::try_from(config.margin_y).unwrap_or(i32::MAX);
    let (top, bottom) = if config.corner.is_top() {
        (y, 0)
    } else {
        (0, y)
    };
    let (left, right) = if config.corner.is_left() {
        (x, 0)
    } else {
        (0, x)
    };
    IcedMargin {
        top,
        right,
        bottom,
        left,
    }
}

#[must_use]
pub fn layer(config: &Config) -> Layer {
    if config.over_windows {
        Layer::Overlay
    } else {
        Layer::Bottom
    }
}

#[must_use]
pub fn size_limits() -> Limits {
    Limits::NONE
        .min_width(1.0)
        .min_height(1.0)
        .max_width(MAX_SURFACE_SIZE)
        .max_height(MAX_SURFACE_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn margins(m: IcedMargin) -> (i32, i32, i32, i32) {
        (m.top, m.right, m.bottom, m.left)
    }

    #[test]
    fn anchors_follow_the_corner() {
        assert_eq!(anchor(Corner::BottomRight), Anchor::BOTTOM | Anchor::RIGHT);
        assert_eq!(anchor(Corner::BottomLeft), Anchor::BOTTOM | Anchor::LEFT);
        assert_eq!(anchor(Corner::TopRight), Anchor::TOP | Anchor::RIGHT);
        assert_eq!(anchor(Corner::TopLeft), Anchor::TOP | Anchor::LEFT);
    }

    #[test]
    fn margins_apply_only_to_anchored_edges() {
        let mut c = Config {
            margin_x: 10,
            margin_y: 20,
            ..Config::default()
        };
        assert_eq!(margins(margin(&c)), (0, 10, 20, 0));
        c.corner = Corner::TopLeft;
        assert_eq!(margins(margin(&c)), (20, 0, 0, 10));
    }

    #[test]
    fn layer_follows_over_windows() {
        let mut c = Config::default();
        assert_eq!(layer(&c), Layer::Bottom);
        c.over_windows = true;
        assert_eq!(layer(&c), Layer::Overlay);
    }

    #[test]
    fn size_limits_allow_large_watermarks() {
        let max = size_limits().max();
        assert!(max.width >= MAX_SURFACE_SIZE && max.height >= MAX_SURFACE_SIZE);
    }
}
