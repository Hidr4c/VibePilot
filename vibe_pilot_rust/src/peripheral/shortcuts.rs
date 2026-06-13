//! Common keyboard shortcut definitions.
//!
//! Provides a lookup table mapping shortcut names to `KeyCombination` structs
//! for use by the orchestrator when the LLM returns a named shortcut action.

use std::collections::HashMap;
use rdev::Key;

/// A key combination representing a keyboard shortcut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCombination {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub win: bool,
    pub key: Option<Key>,
}

impl Default for KeyCombination {
    fn default() -> Self {
        Self {
            ctrl: false,
            shift: false,
            alt: false,
            win: false,
            key: None,
        }
    }
}

/// Returns a static map of common shortcut names to their key combinations.
pub fn shortcut_table() -> &'static HashMap<&'static str, KeyCombination> {
    lazy_static::lazy_static! {
        pub static ref SHORTCUTS: HashMap<&'static str, KeyCombination> = {
            let mut m = HashMap::new();
            // File operations
            m.insert("new", KeyCombination { ctrl: true, key: Some(Key::KeyN), ..Default::default() });
            m.insert("open", KeyCombination { ctrl: true, key: Some(Key::KeyO), ..Default::default() });
            m.insert("save", KeyCombination { ctrl: true, key: Some(Key::KeyS), ..Default::default() });
            m.insert("save_as", KeyCombination { ctrl: true, shift: true, key: Some(Key::KeyS), ..Default::default() });
            m.insert("close", KeyCombination { ctrl: true, key: Some(Key::KeyW), ..Default::default() });
            // Tab operations
            m.insert("new_tab", KeyCombination { ctrl: true, key: Some(Key::KeyT), ..Default::default() });
            m.insert("close_tab", KeyCombination { ctrl: true, key: Some(Key::KeyW), ..Default::default() });
            m.insert("reopen_tab", KeyCombination { ctrl: true, shift: true, key: Some(Key::KeyT), ..Default::default() });
            m.insert("next_tab", KeyCombination { ctrl: true, key: Some(Key::Tab), ..Default::default() });
            m.insert("prev_tab", KeyCombination { ctrl: true, shift: true, key: Some(Key::Tab), ..Default::default() });
            // Edit operations
            m.insert("undo", KeyCombination { ctrl: true, key: Some(Key::KeyZ), ..Default::default() });
            m.insert("redo", KeyCombination { ctrl: true, shift: true, key: Some(Key::KeyZ), ..Default::default() });
            m.insert("cut", KeyCombination { ctrl: true, key: Some(Key::KeyX), ..Default::default() });
            m.insert("copy", KeyCombination { ctrl: true, key: Some(Key::KeyC), ..Default::default() });
            m.insert("paste", KeyCombination { ctrl: true, key: Some(Key::KeyV), ..Default::default() });
            m.insert("select_all", KeyCombination { ctrl: true, key: Some(Key::KeyA), ..Default::default() });
            m.insert("find", KeyCombination { ctrl: true, key: Some(Key::KeyF), ..Default::default() });
            m.insert("find_replace", KeyCombination { ctrl: true, shift: true, key: Some(Key::KeyH), ..Default::default() });
            // Navigation
            m.insert("home", KeyCombination { key: Some(Key::Home), ..Default::default() });
            m.insert("end", KeyCombination { key: Some(Key::End), ..Default::default() });
            m.insert("page_up", KeyCombination { key: Some(Key::PageUp), ..Default::default() });
            m.insert("page_down", KeyCombination { key: Some(Key::PageDown), ..Default::default() });
            m.insert("escape", KeyCombination { key: Some(Key::Escape), ..Default::default() });
            m.insert("enter", KeyCombination { key: Some(Key::Return), ..Default::default() });
            m.insert("tab", KeyCombination { key: Some(Key::Tab), ..Default::default() });
            m.insert("backspace", KeyCombination { key: Some(Key::Backspace), ..Default::default() });
            m.insert("delete", KeyCombination { key: Some(Key::Delete), ..Default::default() });
            // Window management
            m.insert("show_desktop", KeyCombination { win: true, key: Some(Key::Minus), ..Default::default() });
            m.insert("task_view", KeyCombination { win: true, key: Some(Key::Tab), ..Default::default() });
            // Browser-like
            m.insert("refresh", KeyCombination { ctrl: true, key: Some(Key::KeyR), ..Default::default() });
            m.insert("open_new_window", KeyCombination { ctrl: true, shift: true, key: Some(Key::KeyN), ..Default::default() });
            m.insert("print", KeyCombination { ctrl: true, key: Some(Key::KeyP), ..Default::default() });
            // Developer
            m.insert("toggle_devtools", KeyCombination { ctrl: true, shift: true, key: Some(Key::KeyC), ..Default::default() });
            m.insert("command_palette", KeyCombination { ctrl: true, shift: true, key: Some(Key::KeyP), ..Default::default() });
            m.insert("terminal", KeyCombination { ctrl: true, shift: true, key: Some(Key::KeyT), ..Default::default() });
            m
        };
    }
    &SHORTCUTS
}

/// Resolves a shortcut name to its `KeyCombination`.
///
/// Returns `None` if the name is not found in the shortcut table.
pub fn resolve_shortcut(name: &str) -> Option<KeyCombination> {
    shortcut_table().get(name).cloned()
}

/// Returns the list of all registered shortcut names.
pub fn shortcut_names() -> Vec<&'static str> {
    shortcut_table().keys().copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shortcut_table_not_empty() {
        assert!(!shortcut_table().is_empty());
    }

    #[test]
    fn test_resolve_shortcut_copy() {
        let combo = resolve_shortcut("copy").expect("copy shortcut should exist");
        assert!(combo.ctrl);
        assert_eq!(combo.key, Some(Key::KeyC));
        assert!(!combo.shift);
        assert!(!combo.alt);
        assert!(!combo.win);
    }

    #[test]
    fn test_resolve_shortcut_paste() {
        let combo = resolve_shortcut("paste").expect("paste shortcut should exist");
        assert!(combo.ctrl);
        assert_eq!(combo.key, Some(Key::KeyV));
    }

    #[test]
    fn test_resolve_shortcut_undo() {
        let combo = resolve_shortcut("undo").expect("undo shortcut should exist");
        assert!(combo.ctrl);
        assert_eq!(combo.key, Some(Key::KeyZ));
    }

    #[test]
    fn test_resolve_shortcut_redo() {
        let combo = resolve_shortcut("redo").expect("redo shortcut should exist");
        assert!(combo.ctrl);
        assert!(combo.shift);
        assert_eq!(combo.key, Some(Key::KeyZ));
    }

    #[test]
    fn test_resolve_shortcut_new_tab() {
        let combo = resolve_shortcut("new_tab").expect("new_tab shortcut should exist");
        assert!(combo.ctrl);
        assert_eq!(combo.key, Some(Key::KeyT));
    }

    #[test]
    fn test_resolve_shortcut_unknown_returns_none() {
        assert!(resolve_shortcut("nonexistent_shortcut_xyz").is_none());
    }

    #[test]
    fn test_shortcut_names_contains_common() {
        let names = shortcut_names();
        assert!(names.contains(&"copy"));
        assert!(names.contains(&"paste"));
        assert!(names.contains(&"undo"));
        assert!(names.contains(&"save"));
        assert!(names.contains(&"find"));
    }

    #[test]
    fn test_key_combination_default() {
        let combo = KeyCombination::default();
        assert!(!combo.ctrl);
        assert!(!combo.shift);
        assert!(!combo.alt);
        assert!(!combo.win);
        assert!(combo.key.is_none());
    }

    #[test]
    fn test_shortcut_table_equality() {
        let copy1 = resolve_shortcut("copy").unwrap();
        let copy2 = resolve_shortcut("copy").unwrap();
        assert_eq!(copy1, copy2);
    }

    #[test]
    fn test_shortcut_table_all_resolvable() {
        let names = shortcut_names();
        for name in &names {
            assert!(
                resolve_shortcut(name).is_some(),
                "Shortcut '{}' should be resolvable",
                name
            );
        }
    }
}
