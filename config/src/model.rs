// SPDX-License-Identifier: GPL-3.0-or-later

//! The persisted settings and the value types they are built from.

use cosmic_config::{CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};
use serde::{Deserialize, Serialize};

/// Smallest and largest main-line font size, in points.
pub const FONT_SIZE_MIN: u16 = 6;
pub const FONT_SIZE_MAX: u16 = 200;

/// Largest accepted distance from the anchored screen edges, in logical pixels.
pub const MARGIN_MAX: u32 = 2000;

/// Size of the secondary line relative to the main one.
const SECONDARY_FONT_SCALE: f32 = 0.6;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

impl FontStyle {
    pub const ALL: [Self; 3] = [Self::Normal, Self::Italic, Self::Oblique];
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Corner {
    #[default]
    BottomRight,
    BottomLeft,
    TopRight,
    TopLeft,
}

impl Corner {
    pub const ALL: [Self; 4] = [
        Self::BottomRight,
        Self::BottomLeft,
        Self::TopRight,
        Self::TopLeft,
    ];

    #[must_use]
    pub fn is_top(self) -> bool {
        matches!(self, Self::TopRight | Self::TopLeft)
    }

    #[must_use]
    pub fn is_left(self) -> bool {
        matches!(self, Self::BottomLeft | Self::TopLeft)
    }
}

/// Which outputs get a watermark. Outputs are named by connector (e.g. `DP-1`),
/// because Wayland has no primary display and indexes are not stable.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Monitors {
    #[default]
    Automatic,
    All,
    Output(String),
}

/// A colour with straight alpha, every channel in `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Debug, Clone, PartialEq, CosmicConfigEntry)]
#[version = 1]
// Each bool is an independent user toggle stored as its own config key.
#[allow(clippy::struct_excessive_bools)]
pub struct Config {
    pub main_message: String,
    pub secondary_message: String,
    pub font_family: String,
    pub font_style: FontStyle,
    /// Main line size in points.
    pub font_size: u16,
    pub font_color: Rgba,
    pub text_shadow: bool,
    pub background_plate: bool,
    pub corner: Corner,
    /// Logical pixels from the anchored horizontal edge.
    pub margin_x: u32,
    /// Logical pixels from the anchored vertical edge.
    pub margin_y: u32,
    pub over_windows: bool,
    pub show_on_lockscreen: bool,
    pub monitors: Monitors,
}

impl Rgba {
    #[must_use]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Clamp every channel to `0.0..=1.0`, replacing NaN with the matching
    /// channel of `fallback`.
    #[must_use]
    fn clamped_or(self, fallback: Self) -> Self {
        let fix = |value: f32, fallback: f32| {
            if value.is_nan() {
                fallback
            } else {
                value.clamp(0.0, 1.0)
            }
        };
        Self {
            r: fix(self.r, fallback.r),
            g: fix(self.g, fallback.g),
            b: fix(self.b, fallback.b),
            a: fix(self.a, fallback.a),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            main_message: "Activate COSMIC".into(),
            secondary_message: "Go to Settings to activate COSMIC.".into(),
            font_family: "Sans".into(),
            font_style: FontStyle::Normal,
            font_size: 24,
            font_color: Rgba::new(1.0, 1.0, 1.0, 0.5),
            text_shadow: false,
            background_plate: false,
            corner: Corner::BottomRight,
            margin_x: 50,
            margin_y: 50,
            over_windows: false,
            show_on_lockscreen: false,
            monitors: Monitors::Automatic,
        }
    }
}

impl Config {
    /// Size of the secondary line in points.
    #[must_use]
    // font_size is at most u16::MAX, so 60 % of it always fits back into a u16.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn secondary_font_size(&self) -> u16 {
        (f32::from(self.font_size) * SECONDARY_FONT_SCALE).round() as u16
    }

    /// Clamp values a hand-edited config file could have pushed out of range.
    #[must_use]
    pub fn sanitized(mut self) -> Self {
        self.font_size = self.font_size.clamp(FONT_SIZE_MIN, FONT_SIZE_MAX);
        self.margin_x = self.margin_x.min(MARGIN_MAX);
        self.margin_y = self.margin_y.min(MARGIN_MAX);
        self.font_color = self.font_color.clamped_or(Self::default().font_color);
        self
    }
}

/// Convert points to logical pixels at the 96 DPI reference density.
#[must_use]
pub fn points_to_pixels(pt: u16) -> f32 {
    f32::from(pt) * 4.0 / 3.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_spec() {
        let c = Config::default();
        assert_eq!(c.main_message, "Activate COSMIC");
        assert_eq!(c.secondary_message, "Go to Settings to activate COSMIC.");
        assert_eq!(c.font_family, "Sans");
        assert_eq!(c.font_style, FontStyle::Normal);
        assert_eq!(c.font_size, 24);
        assert_eq!(
            c.font_color,
            Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.5
            }
        );
        assert!(!c.text_shadow);
        assert!(!c.background_plate);
        assert_eq!(c.corner, Corner::BottomRight);
        assert_eq!((c.margin_x, c.margin_y), (50, 50));
        assert!(!c.over_windows);
        assert!(!c.show_on_lockscreen);
        assert_eq!(c.monitors, Monitors::Automatic);
    }

    #[test]
    fn secondary_size_is_sixty_percent_rounded() {
        let mut c = Config::default();
        assert_eq!(c.secondary_font_size(), 14); // 14.4
        c.font_size = 25;
        assert_eq!(c.secondary_font_size(), 15);
        c.font_size = FONT_SIZE_MIN;
        assert_eq!(c.secondary_font_size(), 4); // 3.6
    }

    #[test]
    fn points_convert_at_96_dpi() {
        assert!((points_to_pixels(24) - 32.0).abs() < f32::EPSILON);
        assert!((points_to_pixels(18) - 24.0).abs() < f32::EPSILON);
    }

    #[test]
    fn corner_edges() {
        assert!(Corner::TopLeft.is_top() && Corner::TopLeft.is_left());
        assert!(Corner::TopRight.is_top() && !Corner::TopRight.is_left());
        assert!(!Corner::BottomLeft.is_top() && Corner::BottomLeft.is_left());
        assert!(!Corner::BottomRight.is_top() && !Corner::BottomRight.is_left());
    }

    #[test]
    fn sanitized_clamps_hand_edited_values() {
        let c = Config {
            font_size: 0,
            margin_x: u32::MAX,
            margin_y: 4_000_000_000,
            font_color: Rgba {
                r: f32::NAN,
                g: -1.0,
                b: 2.0,
                a: 5.0,
            },
            ..Config::default()
        }
        .sanitized();
        assert_eq!(c.font_size, FONT_SIZE_MIN);
        assert_eq!((c.margin_x, c.margin_y), (MARGIN_MAX, MARGIN_MAX));
        assert_eq!(
            c.font_color,
            Rgba {
                r: 1.0,
                g: 0.0,
                b: 1.0,
                a: 1.0
            }
        );

        let big = Config {
            font_size: 60_000,
            ..Config::default()
        }
        .sanitized();
        assert_eq!(big.font_size, FONT_SIZE_MAX);
    }

    #[test]
    fn sanitized_keeps_valid_values() {
        assert_eq!(Config::default().sanitized(), Config::default());
    }

    #[test]
    fn values_round_trip_through_ron() {
        for corner in Corner::ALL {
            let s = ron::to_string(&corner).unwrap();
            assert_eq!(ron::from_str::<Corner>(&s).unwrap(), corner);
        }
        for style in FontStyle::ALL {
            let s = ron::to_string(&style).unwrap();
            assert_eq!(ron::from_str::<FontStyle>(&s).unwrap(), style);
        }
        for monitors in [
            Monitors::Automatic,
            Monitors::All,
            Monitors::Output("DP-1".into()),
        ] {
            let s = ron::to_string(&monitors).unwrap();
            assert_eq!(ron::from_str::<Monitors>(&s).unwrap(), monitors);
        }
        let color = Rgba {
            r: 0.25,
            g: 0.5,
            b: 0.75,
            a: 1.0,
        };
        let s = ron::to_string(&color).unwrap();
        assert_eq!(ron::from_str::<Rgba>(&s).unwrap(), color);
    }
}
