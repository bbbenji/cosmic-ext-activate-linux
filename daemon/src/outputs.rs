// SPDX-License-Identifier: GPL-3.0-or-later

//! Connected outputs by connector name. Generic over the key so it can be
//! tested without a Wayland connection; the daemon uses `WlOutput`.

#[derive(Debug, Clone)]
pub struct Outputs<K> {
    entries: Vec<(K, String)>,
}

impl<K: PartialEq> Outputs<K> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Record or rename an output. Returns whether the set of names changed.
    pub fn upsert(&mut self, key: K, name: String) -> bool {
        if let Some(entry) = self.entries.iter_mut().find(|(k, _)| *k == key) {
            if entry.1 == name {
                return false;
            }
            entry.1 = name;
        } else {
            self.entries.push((key, name));
        }
        true
    }

    /// Forget an output. Returns whether it was known.
    pub fn remove(&mut self, key: &K) -> bool {
        let before = self.entries.len();
        self.entries.retain(|(k, _)| k != key);
        self.entries.len() != before
    }

    #[must_use]
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.entries.iter().map(|(_, n)| n.clone()).collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    #[must_use]
    pub fn key_for(&self, name: &str) -> Option<&K> {
        self.entries.iter().find(|(_, n)| n == name).map(|(k, _)| k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_names_sorted() {
        let mut o = Outputs::new();
        assert!(o.upsert(2, "HDMI-A-1".into()));
        assert!(o.upsert(1, "DP-1".into()));
        assert_eq!(o.names(), vec!["DP-1".to_owned(), "HDMI-A-1".to_owned()]);
        assert_eq!(o.key_for("HDMI-A-1"), Some(&2));
        assert_eq!(o.key_for("DP-9"), None);
    }

    #[test]
    fn upsert_reports_only_real_changes() {
        let mut o = Outputs::new();
        assert!(o.upsert(1, "DP-1".into()));
        assert!(!o.upsert(1, "DP-1".into()));
        assert!(o.upsert(1, "DP-2".into())); // renamed via InfoUpdate
        assert_eq!(o.names(), vec!["DP-2".to_owned()]);
    }

    #[test]
    fn remove() {
        let mut o = Outputs::new();
        o.upsert(1, "DP-1".into());
        assert!(o.remove(&1));
        assert!(!o.remove(&1));
        assert_eq!(o.names(), Vec::<String>::new());
    }
}
