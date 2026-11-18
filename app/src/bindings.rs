//! Which keys run which action. Each action has a default; the setter can
//! change any of them in Settings, and the changes are kept in settings.json.
//! The page's key listener, the menus, tooltips and the palette all read the
//! current bindings from here.

use crate::state::Shortcut;
use std::collections::BTreeMap;
use std::sync::{PoisonError, RwLock};

/// A key with the modifiers held: what a keydown event reports.
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct Combo {
    /// `KeyboardEvent.key`, lowercase: "k", ",", "arrowleft", " ".
    pub key: String,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

/// How a key is written: "K", ",", "←", "Space", "F5".
fn key_name(key: &str) -> String {
    match key {
        " " => "Space".into(),
        "arrowleft" => "←".into(),
        "arrowright" => "→".into(),
        "arrowup" => "↑".into(),
        "arrowdown" => "↓".into(),
        "enter" => "Enter".into(),
        "tab" => "Tab".into(),
        "backspace" => "Backspace".into(),
        "delete" => "Delete".into(),
        "home" => "Home".into(),
        "end" => "End".into(),
        "pageup" => "PageUp".into(),
        "pagedown" => "PageDown".into(),
        k if k.len() > 1 && k.starts_with('f') && k[1..].parse::<u8>().is_ok() => k.to_uppercase(),
        k => k.to_uppercase(),
    }
}

/// The reverse of [`key_name`].
fn key_of(name: &str) -> String {
    match name {
        "Space" => " ".into(),
        "←" => "arrowleft".into(),
        "→" => "arrowright".into(),
        "↑" => "arrowup".into(),
        "↓" => "arrowdown".into(),
        n => n.to_lowercase(),
    }
}

impl Combo {
    /// From the Windows way of writing it: "Ctrl+Shift+S", "Alt+←", "Ctrl+,".
    pub fn parse(text: &str) -> Option<Combo> {
        let mut c = Combo {
            key: String::new(),
            ctrl: false,
            alt: false,
            shift: false,
        };
        // "Ctrl++" isn't used; split on '+' but keep a trailing "+" key.
        for part in text.split('+').filter(|p| !p.is_empty()) {
            match part {
                "Ctrl" => c.ctrl = true,
                "Alt" => c.alt = true,
                "Shift" => c.shift = true,
                k => c.key = key_of(k),
            }
        }
        (!c.key.is_empty()).then_some(c)
    }

    /// Written the Windows way: "Ctrl+Shift+S". [`crate::keys::label`] turns
    /// it into a Mac's ⌘⇧S.
    pub fn text(&self) -> String {
        let mut parts: Vec<String> = vec![];
        if self.ctrl {
            parts.push("Ctrl".into());
        }
        if self.alt {
            parts.push("Alt".into());
        }
        if self.shift {
            parts.push("Shift".into());
        }
        parts.push(key_name(&self.key));
        parts.join("+")
    }

    /// Fit to be a shortcut? It needs Ctrl or Alt, so plain typing still
    /// types, and a real key, not a modifier alone.
    pub fn usable(&self) -> Result<(), &'static str> {
        if matches!(
            self.key.as_str(),
            "control" | "alt" | "shift" | "meta" | "altgraph" | "capslock" | ""
        ) {
            return Err("Press a key with Ctrl or Alt");
        }
        if !self.ctrl && !self.alt {
            return Err("Hold Ctrl or Alt with the key");
        }
        Ok(())
    }
}

impl Shortcut {
    pub const ALL: [Shortcut; 13] = [
        Shortcut::Palette,
        Shortcut::New,
        Shortcut::Open,
        Shortcut::Save,
        Shortcut::SaveAs,
        Shortcut::Export,
        Shortcut::Undo,
        Shortcut::Redo,
        Shortcut::PreviousAnswer,
        Shortcut::NextAnswer,
        Shortcut::Settings,
        Shortcut::Words,
        Shortcut::NextUnclued,
    ];

    /// Kept in settings.json under this name.
    pub fn id(self) -> &'static str {
        match self {
            Shortcut::PreviousAnswer => "previous-answer",
            Shortcut::NextAnswer => "next-answer",
            Shortcut::Save => "save",
            Shortcut::SaveAs => "save-as",
            Shortcut::New => "new",
            Shortcut::Open => "open",
            Shortcut::Export => "export",
            Shortcut::Undo => "undo",
            Shortcut::Redo => "redo",
            Shortcut::Palette => "search",
            Shortcut::Settings => "settings",
            Shortcut::NextUnclued => "next-unclued",
            Shortcut::Words => "words-that-fit",
        }
    }

    /// What it does, as Settings lists it.
    pub fn name(self) -> &'static str {
        match self {
            Shortcut::PreviousAnswer => "Previous answer",
            Shortcut::NextAnswer => "Next answer",
            Shortcut::Save => "Save now",
            Shortcut::SaveAs => "Save as",
            Shortcut::New => "New puzzle",
            Shortcut::Open => "Open",
            Shortcut::Export => "Export web page",
            Shortcut::Undo => "Undo",
            Shortcut::Redo => "Redo",
            Shortcut::Palette => "Search commands, answers and puzzles",
            Shortcut::Settings => "Settings",
            Shortcut::NextUnclued => "Next answer without a clue",
            Shortcut::Words => "Words that fit",
        }
    }

    /// Where it works, for grouping in Settings.
    pub fn group(self) -> &'static str {
        match self {
            Shortcut::Words => "Build grid",
            Shortcut::NextUnclued => "Write clues",
            _ => "Anywhere",
        }
    }

    pub fn default_keys(self) -> &'static str {
        match self {
            Shortcut::PreviousAnswer => "Alt+←",
            Shortcut::NextAnswer => "Alt+→",
            Shortcut::Save => "Ctrl+S",
            Shortcut::SaveAs => "Ctrl+Shift+S",
            Shortcut::New => "Ctrl+N",
            Shortcut::Open => "Ctrl+O",
            Shortcut::Export => "Ctrl+E",
            Shortcut::Undo => "Ctrl+Z",
            Shortcut::Redo => "Ctrl+Shift+Z",
            Shortcut::Palette => "Ctrl+K",
            Shortcut::Settings => "Ctrl+,",
            Shortcut::NextUnclued => "Ctrl+J",
            Shortcut::Words => "Ctrl+Space",
        }
    }

    fn from_id(id: &str) -> Option<Shortcut> {
        Shortcut::ALL.into_iter().find(|s| s.id() == id)
    }
}

/// The setter's changes: action id → keys ("Ctrl+Shift+K").
pub type Changes = BTreeMap<String, String>;

static CHANGES: RwLock<Changes> = RwLock::new(BTreeMap::new());

/// Use these changes from now on (at start-up and whenever they change).
pub fn use_changes(changes: &Changes) {
    *CHANGES.write().unwrap_or_else(PoisonError::into_inner) = changes.clone();
}

/// The keys for an action, given the setter's changes.
pub fn combo_in(changes: &Changes, s: Shortcut) -> Combo {
    changes
        .get(s.id())
        .and_then(|t| Combo::parse(t))
        .unwrap_or_else(|| Combo::parse(s.default_keys()).expect("defaults parse"))
}

/// The keys for an action now.
pub fn combo(s: Shortcut) -> Combo {
    combo_in(&CHANGES.read().unwrap_or_else(PoisonError::into_inner), s)
}

/// The keys for an action now, written the Windows way ("Ctrl+K").
pub fn keys(s: Shortcut) -> String {
    combo(s).text()
}

/// The keys for an action, as this platform shows them ("Ctrl+K", "⌘K").
pub fn label(s: Shortcut) -> String {
    crate::keys::label(&keys(s))
}

/// A tooltip: "Undo (Ctrl+Z)".
pub fn tip(action: &str, s: Shortcut) -> String {
    crate::keys::tip(action, &keys(s))
}

/// Which action a keydown runs, if any.
pub fn action_for(pressed: &Combo) -> Option<Shortcut> {
    Shortcut::ALL.into_iter().find(|s| combo(*s) == *pressed)
}

/// Which other action already uses these keys, given the changes.
pub fn clash(changes: &Changes, s: Shortcut, keys: &Combo) -> Option<Shortcut> {
    Shortcut::ALL
        .into_iter()
        .find(|other| *other != s && combo_in(changes, *other) == *keys)
}

/// Give `s` these keys. Keys back to the default aren't kept as a change.
pub fn set(changes: &mut Changes, s: Shortcut, keys: &Combo) {
    if Combo::parse(s.default_keys()).as_ref() == Some(keys) {
        changes.remove(s.id());
    } else {
        changes.insert(s.id().to_string(), keys.text());
    }
}

/// Swap: `s` takes `keys`, and whatever had them takes `s`'s old keys.
pub fn swap(changes: &mut Changes, s: Shortcut, keys: &Combo) {
    let old = combo_in(changes, s);
    if let Some(other) = clash(changes, s, keys) {
        set(changes, other, &old);
    }
    set(changes, s, keys);
}

/// Drop changes for actions that no longer exist.
pub fn tidy(changes: &mut Changes) {
    changes.retain(|id, keys| Shortcut::from_id(id).is_some() && Combo::parse(keys).is_some());
}

/// The script's table of combos: [[key, alt, ctrl, shift, works while typing]].
pub fn script_table() -> String {
    let rows: Vec<String> = Shortcut::ALL
        .into_iter()
        .map(|s| {
            let c = combo(s);
            let key = serde_json::to_string(&c.key).unwrap_or_default();
            format!(
                "[{key},{},{},{},{}]",
                c.alt,
                c.ctrl,
                c.shift,
                s.while_typing()
            )
        })
        .collect();
    format!("[{}]", rows.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_read_and_written_the_windows_way() {
        for text in [
            "Ctrl+Shift+S",
            "Alt+←",
            "Ctrl+,",
            "Ctrl+Space",
            "Ctrl+Alt+F5",
        ] {
            assert_eq!(Combo::parse(text).unwrap().text(), text);
        }
        let c = Combo::parse("Ctrl+Space").unwrap();
        assert_eq!(c.key, " ");
        assert!(Combo::parse("Ctrl").is_none());
    }

    #[test]
    fn every_default_is_different() {
        let mut seen = std::collections::HashSet::new();
        for s in Shortcut::ALL {
            assert!(
                seen.insert(combo_in(&Changes::new(), s)),
                "{} clashes",
                s.name()
            );
        }
    }

    #[test]
    fn plain_keys_and_lone_modifiers_are_refused() {
        let k = |t: &str| Combo::parse(t).unwrap();
        assert!(k("K").usable().is_err(), "a plain letter would stop typing");
        assert!(k("Shift+K").usable().is_err());
        assert!(k("Ctrl+K").usable().is_ok());
        assert!(k("Alt+P").usable().is_ok());
        let lone = Combo {
            key: "control".into(),
            ctrl: true,
            alt: false,
            shift: false,
        };
        assert!(lone.usable().is_err());
    }

    #[test]
    fn a_change_is_kept_and_back_to_default_is_forgotten() {
        let mut ch = Changes::new();
        set(&mut ch, Shortcut::Palette, &Combo::parse("Ctrl+P").unwrap());
        assert_eq!(ch.get("search").map(String::as_str), Some("Ctrl+P"));
        assert_eq!(combo_in(&ch, Shortcut::Palette).text(), "Ctrl+P");
        set(&mut ch, Shortcut::Palette, &Combo::parse("Ctrl+K").unwrap());
        assert!(ch.is_empty());
    }

    #[test]
    fn clashes_are_found_and_swapped() {
        let mut ch = Changes::new();
        let ctrl_n = Combo::parse("Ctrl+N").unwrap();
        assert_eq!(clash(&ch, Shortcut::Palette, &ctrl_n), Some(Shortcut::New));
        swap(&mut ch, Shortcut::Palette, &ctrl_n);
        assert_eq!(combo_in(&ch, Shortcut::Palette).text(), "Ctrl+N");
        assert_eq!(combo_in(&ch, Shortcut::New).text(), "Ctrl+K");
        assert_eq!(clash(&ch, Shortcut::Palette, &ctrl_n), None);
    }

    #[test]
    fn unknown_or_broken_changes_are_dropped() {
        let mut ch = Changes::new();
        ch.insert("gone".into(), "Ctrl+Q".into());
        ch.insert("search".into(), "".into());
        ch.insert("new".into(), "Ctrl+M".into());
        tidy(&mut ch);
        assert_eq!(ch.len(), 1);
    }
}
