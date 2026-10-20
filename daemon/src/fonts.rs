// SPDX-License-Identifier: GPL-3.0-or-later

//! Mapping the configured family/style to an iced `Font`.

use std::collections::HashMap;

use activate_linux_config::{Config, FontStyle};
use cosmic::iced::Font;
use cosmic::iced::font::{Family, Style};

/// iced wants `&'static str` family names; this leaks each distinct name once,
/// so memory is bounded by the number of different families ever configured.
#[derive(Debug, Default)]
pub struct FontFamilies {
    names: HashMap<String, &'static str>,
}

impl FontFamilies {
    pub fn font(&mut self, config: &Config) -> Font {
        let family = match config.font_family.trim() {
            "" | "Sans" | "sans" | "sans-serif" => Family::SansSerif,
            "Serif" | "serif" => Family::Serif,
            "Monospace" | "monospace" => Family::Monospace,
            name => Family::Name(self.intern(name)),
        };
        let style = match config.font_style {
            FontStyle::Normal => Style::Normal,
            FontStyle::Italic => Style::Italic,
            FontStyle::Oblique => Style::Oblique,
        };
        Font {
            family,
            style,
            ..Font::DEFAULT
        }
    }

    fn intern(&mut self, name: &str) -> &'static str {
        if let Some(interned) = self.names.get(name) {
            return interned;
        }
        let interned: &'static str = Box::leak(name.to_owned().into_boxed_str());
        self.names.insert(name.to_owned(), interned);
        interned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_family(name: &str) -> Config {
        Config {
            font_family: name.into(),
            ..Config::default()
        }
    }

    #[test]
    fn generic_families() {
        let mut f = FontFamilies::default();
        assert_eq!(f.font(&with_family("Sans")).family, Family::SansSerif);
        assert_eq!(f.font(&with_family("sans-serif")).family, Family::SansSerif);
        assert_eq!(f.font(&with_family("")).family, Family::SansSerif);
        assert_eq!(f.font(&with_family("Serif")).family, Family::Serif);
        assert_eq!(f.font(&with_family("monospace")).family, Family::Monospace);
    }

    #[test]
    fn named_families_are_interned_once() {
        let mut f = FontFamilies::default();
        let a = f.font(&with_family("Fira Sans")).family;
        let b = f.font(&with_family("Fira Sans")).family;
        let (Family::Name(a), Family::Name(b)) = (a, b) else {
            panic!("expected named family")
        };
        assert_eq!(a, "Fira Sans");
        assert!(std::ptr::eq(a, b));
    }

    #[test]
    fn styles() {
        let mut f = FontFamilies::default();
        let mut c = Config::default();
        assert_eq!(f.font(&c).style, Style::Normal);
        c.font_style = FontStyle::Italic;
        assert_eq!(f.font(&c).style, Style::Italic);
        c.font_style = FontStyle::Oblique;
        assert_eq!(f.font(&c).style, Style::Oblique);
    }
}
