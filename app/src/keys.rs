//! Keyboard shortcuts as people on each platform expect to read them:
//! "⌘⇧S" on a Mac, "Ctrl+Shift+S" elsewhere. Shortcuts are written once,
//! the Windows way ("Ctrl+Shift+S"), and shown with [`label`].

/// Is this a Mac? (Where ⌘ stands in for Ctrl.)
pub const MAC: bool = cfg!(target_os = "macos");

/// A shortcut written as "Ctrl+Shift+S", for this platform.
pub fn label(keys: &str) -> String {
    label_for(keys, MAC)
}

/// A shortcut for a given platform: on a Mac, symbols run together (⌘⇧S,
/// ⌥←); elsewhere the names stay, joined with "+".
pub fn label_for(keys: &str, mac: bool) -> String {
    if !mac {
        return keys.to_string();
    }
    keys.split('+')
        .map(|part| match part {
            "Ctrl" => "⌘".to_string(),
            "Shift" => "⇧".to_string(),
            "Alt" => "⌥".to_string(),
            "Space" => "Space".to_string(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .concat()
}

/// A shortcut's keys one by one, for drawing as chips: [Ctrl] [Shift] [S],
/// or [⌘] [⇧] [S] on a Mac.
pub fn parts(keys: &str) -> Vec<String> {
    parts_for(keys, MAC)
}

pub fn parts_for(keys: &str, mac: bool) -> Vec<String> {
    keys.split('+')
        .filter(|k| !k.is_empty())
        .map(|k| label_for(k, mac))
        .collect()
}

/// A tooltip naming an action and its shortcut: "Undo (⌘Z)".
pub fn tip(action: &str, keys: &str) -> String {
    format!("{action} ({})", label(keys))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_and_linux_keep_the_names() {
        assert_eq!(label_for("Ctrl+Shift+S", false), "Ctrl+Shift+S");
        assert_eq!(label_for("Alt+←", false), "Alt+←");
    }

    #[test]
    fn a_mac_shows_the_symbols() {
        assert_eq!(label_for("Ctrl+Shift+S", true), "⌘⇧S");
        assert_eq!(label_for("Ctrl+Z", true), "⌘Z");
        assert_eq!(label_for("Alt+←", true), "⌥←");
        assert_eq!(label_for("Ctrl+Space", true), "⌘Space");
    }

    #[test]
    fn keys_split_into_chips_per_platform() {
        assert_eq!(parts_for("Ctrl+Shift+S", false), ["Ctrl", "Shift", "S"]);
        assert_eq!(parts_for("Ctrl+Shift+S", true), ["⌘", "⇧", "S"]);
        assert_eq!(parts_for("Ctrl+,", false), ["Ctrl", ","]);
    }
}
