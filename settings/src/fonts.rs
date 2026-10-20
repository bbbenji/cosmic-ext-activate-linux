// SPDX-License-Identifier: GPL-3.0-or-later

//! The font family list for the font dropdown.

/// Every installed family name, sorted, without duplicates.
#[must_use]
pub fn system_families() -> Vec<String> {
    let mut font_system = cosmic::iced::advanced::graphics::text::font_system()
        .write()
        .expect("font system lock poisoned");
    let mut families: Vec<String> = font_system
        .raw()
        .db()
        .faces()
        .filter_map(|face| face.families.first().map(|(name, _)| name.clone()))
        .collect();
    families.sort_unstable_by_key(|name| name.to_lowercase());
    families.dedup();
    families
}

/// Families containing `query` (case-insensitive), always including `current`.
#[must_use]
pub fn filter(all: &[String], query: &str, current: &str) -> Vec<String> {
    let query = query.trim().to_lowercase();
    let mut list: Vec<String> = all
        .iter()
        .filter(|name| {
            query.is_empty() || name.to_lowercase().contains(&query) || name.as_str() == current
        })
        .cloned()
        .collect();
    if !list.iter().any(|name| name == current) {
        list.push(current.to_owned());
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fams(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn filter_is_case_insensitive_and_keeps_current() {
        let all = fams(&["Fira Sans", "Noto Serif", "Open Sans"]);
        assert_eq!(
            filter(&all, "sans", "Noto Serif"),
            fams(&["Fira Sans", "Noto Serif", "Open Sans"])
        );
        assert_eq!(
            filter(&all, "FIRA", "Open Sans"),
            fams(&["Fira Sans", "Open Sans"])
        );
    }

    #[test]
    fn empty_filter_returns_everything() {
        let all = fams(&["A", "B"]);
        assert_eq!(filter(&all, "  ", "A"), all);
    }

    #[test]
    fn current_family_is_listed_even_if_not_installed() {
        assert_eq!(
            filter(&fams(&["Fira Sans"]), "", "Sans"),
            fams(&["Fira Sans", "Sans"])
        );
    }
}
