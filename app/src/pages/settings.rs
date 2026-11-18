//! Settings: a page in the app, like Home. General, the setter's word lists,
//! abbreviations and indicators, and every keyboard shortcut. Opened from the
//! gear menu or Ctrl+,; Esc or "Back" returns.

use crate::bindings::{self, Combo};
use crate::components::button::{Button, ButtonSize, ButtonVariant};
use crate::ds::list_editor::{ListEditor, ListRow};
use crate::ds::segmented::Segmented;
use crate::ds::settings::{
    KeyChips, SettingsGroup, SettingsNav, SettingsNavItem, SettingsPage, SettingsRow,
};
use crate::files::{self, Theme};
use crate::library;
use crate::state::{self, AppState, Shortcut};
use crate::user_lists::{self, WordList};
use dioxus::prelude::*;
use std::time::SystemTime;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    General,
    WordLists,
    Definitions,
    Abbreviations,
    Indicators,
    Shortcuts,
}

impl Section {
    pub const ALL: [Section; 6] = [
        Section::General,
        Section::WordLists,
        Section::Definitions,
        Section::Abbreviations,
        Section::Indicators,
        Section::Shortcuts,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Section::General => "general",
            Section::WordLists => "words",
            Section::Definitions => "definitions",
            Section::Abbreviations => "abbreviations",
            Section::Indicators => "indicators",
            Section::Shortcuts => "shortcuts",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Section::General => "General",
            Section::WordLists => "Word lists",
            Section::Definitions => "Definitions",
            Section::Abbreviations => "Abbreviations",
            Section::Indicators => "Indicators",
            Section::Shortcuts => "Keyboard shortcuts",
        }
    }

    /// Words that find the section from "Find a setting".
    fn words(self) -> &'static str {
        match self {
            Section::General => "general setter name author theme dark light system appearance definition start end puzzles folder documents save updates update automatically version",
            Section::WordLists => "word lists words phrases own dictionary never suggest banned block import export",
            Section::Definitions => "definitions definition meaning means answer phrase synonym own thesaurus",
            Section::Abbreviations => "abbreviations letters stand for meanings",
            Section::Indicators => "indicators anagram reversal container hidden homophone sounds deletion charade spoonerism",
            Section::Shortcuts => "keyboard shortcuts keys ctrl cmd command hotkeys",
        }
    }

    fn from_key(key: &str) -> Option<Section> {
        Section::ALL.into_iter().find(|s| s.key() == key)
    }
}

/// The sections "Find a setting" leaves in the sidebar.
pub fn matching(query: &str) -> Vec<Section> {
    let q = query.trim().to_lowercase();
    Section::ALL
        .into_iter()
        .filter(|s| {
            q.is_empty()
                || s.label().to_lowercase().contains(&q)
                || s.words().split(' ').any(|w| w.starts_with(&q))
        })
        .collect()
}

/// Keys that work in the grid and can't be changed: (action, keys).
pub const FIXED_KEYS: &[(&str, &str)] = &[
    ("Type a letter", "A"),
    ("Move between squares", "← · → · ↑ · ↓"),
    ("Switch across and down", "Space"),
    ("Delete a letter", "Backspace · Delete"),
];

/// One row of the shortcuts list.
#[derive(Clone, PartialEq, Debug)]
pub enum KeyRow {
    /// An action whose keys can be changed.
    Action(Shortcut),
    /// A key that works in the grid: (action, keys).
    Fixed(&'static str, &'static str),
}

/// The shortcuts whose action or keys match `query`, by where they work.
pub fn key_rows(query: &str) -> Vec<(&'static str, Vec<KeyRow>)> {
    let q = query.trim().to_lowercase();
    let hit = |action: &str, keys: &str| {
        q.is_empty()
            || action.to_lowercase().contains(&q)
            || keys.to_lowercase().contains(&q)
            || crate::keys::label(keys).to_lowercase().contains(&q)
    };
    let mut groups: Vec<(&'static str, Vec<KeyRow>)> = vec![];
    for s in Shortcut::ALL {
        if hit(s.name(), &bindings::keys(s)) {
            match groups.iter_mut().find(|(g, _)| *g == s.group()) {
                Some((_, rows)) => rows.push(KeyRow::Action(s)),
                None => groups.push((s.group(), vec![KeyRow::Action(s)])),
            }
        }
    }
    let fixed: Vec<KeyRow> = FIXED_KEYS
        .iter()
        .filter(|(a, k)| hit(a, k))
        .map(|(a, k)| KeyRow::Fixed(a, k))
        .collect();
    if !fixed.is_empty() {
        groups.push(("Typing in the grid", fixed));
    }
    groups
}

/// What happened to keys pressed for an action.
#[derive(Clone, PartialEq, Debug)]
pub enum Pressed {
    /// Esc: leave it as it was.
    Cancel,
    /// Not usable as a shortcut; why.
    Refused(&'static str),
    /// Another action has them: offer to swap.
    Clash(Combo, Shortcut),
    /// Taken.
    Set(Combo),
}

/// Judge keys pressed for `s`, given the changes so far.
pub fn judge(changes: &bindings::Changes, s: Shortcut, pressed: Combo) -> Pressed {
    if pressed.key == "escape" && !pressed.ctrl && !pressed.alt && !pressed.shift {
        return Pressed::Cancel;
    }
    if let Err(why) = pressed.usable() {
        return Pressed::Refused(why);
    }
    match bindings::clash(changes, s, &pressed) {
        Some(other) => Pressed::Clash(pressed, other),
        None => Pressed::Set(pressed),
    }
}

/// Use and keep new shortcut changes.
fn keep_keys(mut state: AppState, changes: bindings::Changes) {
    bindings::use_changes(&changes);
    state.settings.write().shortcuts = changes;
    files::save_settings(&state.settings.peek());
    document::eval(&state::combos_script());
}

/// "added today", "added 3 days ago"; nothing for words from before dates
/// were kept.
pub fn added(when: Option<SystemTime>, now: SystemTime) -> String {
    match when {
        Some(t) => match library::ago(t, now).as_str() {
            "just now" | "a minute ago" => "added today".to_string(),
            ago if ago.ends_with("minutes ago") || ago.contains("hour") => {
                "added today".to_string()
            }
            ago => format!("added {ago}"),
        },
        None => String::new(),
    }
}

/// The lists changed: load the words again so they count everywhere, and
/// let word banks be built afresh.
pub fn lists_changed(mut state: AppState) {
    *state.build.reloads.write() += 1;
    dioxus::core::spawn_forever(async move {
        let _ = tokio::task::spawn_blocking(state::reload_engine).await;
        state.banks.write().clear();
        *state.build.reloads.write() += 1;
    });
}

fn report(mut state: AppState, result: Result<(), String>) {
    match result {
        Ok(()) => lists_changed(state),
        Err(e) => state.notice.set(Some(e)),
    }
}

/// Where "Back" goes: "Back to Home" or "Back to Prize puzzle 12".
pub fn back_label(state: AppState) -> String {
    if (state.home)() {
        return "Back to Home".to_string();
    }
    let title = state
        .puzzle
        .read()
        .as_ref()
        .map(|p| p.title.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "the puzzle".into());
    format!("Back to {title}")
}

#[component]
pub fn Settings(section: Section) -> Element {
    let mut state = use_context::<AppState>();
    let mut query = use_signal(String::new);
    // Esc goes back, unless it's for a text box, a menu or the palette.
    use_effect(move || {
        spawn(async move {
            let mut esc = document::eval(
                "document.addEventListener('keydown', (e) => {\
                   if (e.key !== 'Escape' || e.defaultPrevented) return;\
                   if (e.target.closest && e.target.closest('input, textarea, select, [role=menu], .palette')) return;\
                   dioxus.send(true);\
                 });\
                 await new Promise(() => {});",
            );
            while esc.recv::<bool>().await.is_ok() {
                state.settings_page.set(None);
            }
        });
    });
    // Counts in the sidebar; re-read when the lists change.
    let _ = (state.build.reloads)();
    let dir = files::config_dir();
    let words = user_lists::read(&dir, WordList::Yours).len()
        + user_lists::read(&dir, WordList::Never).len();
    let abbrevs = user_lists::abbreviations(&dir).len();
    let defs = user_lists::definitions(&dir).len();
    let indicators: usize = user_lists::indicators(&dir).values().map(Vec::len).sum();
    let shown = matching(&query());
    let items: Vec<SettingsNavItem> = shown
        .iter()
        .map(|s| SettingsNavItem {
            key: s.key().into(),
            label: s.label().into(),
            count: match s {
                Section::WordLists => Some(words),
                Section::Definitions => Some(defs),
                Section::Abbreviations => Some(abbrevs),
                Section::Indicators => Some(indicators),
                _ => None,
            }
            .filter(|n| *n > 0),
        })
        .collect();
    let back = back_label(state);
    rsx! {
        div { class: "settings",
            SettingsNav {
                back,
                items,
                current: section.key(),
                query: query(),
                on_query: move |q: String| {
                    // The first match opens as you type.
                    if let Some(first) = matching(&q).first() {
                        state.settings_page.set(Some(*first));
                    }
                    query.set(q);
                },
                on_select: move |k: String| state.settings_page.set(Section::from_key(&k)),
                on_back: move |_| state.settings_page.set(None),
            }
            div { class: "settings__scroll",
                match section {
                    Section::General => rsx! { General {} },
                    Section::WordLists => rsx! { WordLists {} },
                    Section::Definitions => rsx! { Definitions {} },
                    Section::Abbreviations => rsx! { Abbreviations {} },
                    Section::Indicators => rsx! { Indicators {} },
                    Section::Shortcuts => rsx! { Shortcuts {} },
                }
            }
        }
    }
}

#[component]
fn General() -> Element {
    let mut state = use_context::<AppState>();
    let s = state.settings.read().clone();
    let theme = match s.theme {
        Theme::System => 0,
        Theme::Light => 1,
        Theme::Dark => 2,
    };
    let folder = files::library_dir();
    let save = move || files::save_settings(&state.settings.peek());
    rsx! {
        SettingsPage { title: "General",
            SettingsGroup { title: "You",
                SettingsRow { label: "Setter name", detail: "On new puzzles and in exported puzzles",
                    input {
                        class: "settings-input",
                        r#type: "text",
                        "aria-label": "Setter name",
                        value: "{s.setter_name}",
                        oninput: move |e| state.settings.write().setter_name = e.value(),
                        onchange: move |_| save(),
                    }
                }
            }
            SettingsGroup { title: "Appearance",
                SettingsRow { label: "Theme",
                    Segmented {
                        label: "Theme",
                        options: vec!["System".to_string(), "Light".to_string(), "Dark".to_string()],
                        selected: theme,
                        on_change: move |i: usize| {
                            state.settings.write().theme = [Theme::System, Theme::Light, Theme::Dark][i.min(2)];
                            save();
                        },
                    }
                }
            }
            SettingsGroup { title: "Writing clues",
                SettingsRow { label: "Definition goes at", detail: "Where a picked definition is placed in a new clue",
                    Segmented {
                        label: "Definition goes at",
                        options: vec!["Start".to_string(), "End".to_string()],
                        selected: usize::from(s.definition_last),
                        on_change: move |i: usize| {
                            state.settings.write().definition_last = i == 1;
                            save();
                        },
                    }
                }
            }
            SettingsGroup { title: "Updates",
                SettingsRow { label: "Setter {env!(\"CARGO_PKG_VERSION\")}",
                    crate::pages::about::UpdateStatus {}
                }
                SettingsRow { label: "Check for updates",
                    Segmented {
                        label: "Check for updates",
                        options: vec!["Automatically".to_string(), "Only when I ask".to_string()],
                        selected: usize::from(s.manual_update_checks),
                        on_change: move |i: usize| {
                            state.settings.write().manual_update_checks = i == 1;
                            save();
                        },
                    }
                }
                if !s.manual_update_checks {
                    SettingsRow { label: "New versions",
                        Segmented {
                            label: "New versions",
                            options: vec!["Ask first".to_string(), "Install automatically".to_string()],
                            selected: usize::from(s.auto_update),
                            on_change: move |i: usize| {
                                state.settings.write().auto_update = i == 1;
                                save();
                            },
                        }
                    }
                }
            }
            SettingsGroup { title: "Puzzles",
                SettingsRow { label: "Puzzles folder", detail: "{folder.display()}",
                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        onclick: move |_| {
                            if let Err(e) = files::reveal(&files::library_dir()) {
                                state.notice.set(Some(format!("Couldn't show the folder: {e}")));
                            }
                        },
                        "Show"
                    }
                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        onclick: move |_| {
                            spawn(async move {
                                let picked = rfd::AsyncFileDialog::new()
                                    .set_directory(files::library_dir())
                                    .pick_folder()
                                    .await;
                                if let Some(dir) = picked {
                                    state.settings.write().puzzles_folder = Some(dir.path().to_path_buf());
                                    files::save_settings(&state.settings.peek());
                                }
                            });
                        },
                        "Change…"
                    }
                }
            }
        }
    }
}

#[component]
fn WordLists() -> Element {
    let mut state = use_context::<AppState>();
    let mut tab = use_signal(|| 0usize);
    let _ = (state.build.reloads)();
    let dir = files::config_dir();
    let lists = [WordList::Yours, WordList::Never];
    let list = lists[tab().min(1)];
    let entries = user_lists::read(&dir, list);
    let counts = [
        user_lists::read(&dir, WordList::Yours).len(),
        user_lists::read(&dir, WordList::Never).len(),
    ];
    let now = SystemTime::now();
    let rows: Vec<ListRow> = entries
        .iter()
        .map(|e| ListRow {
            text: e.text.clone(),
            detail: user_lists::enumeration(&e.text),
            when: added(e.added, now),
        })
        .collect();
    let exported = user_lists::export(&entries);
    rsx! {
        SettingsPage { title: "Word lists",
            ListEditor {
                rows,
                tabs: vec![("Your words".to_string(), counts[0]), ("Never suggest".to_string(), counts[1])],
                tab: tab(),
                on_tab: move |i| tab.set(i),
                placeholder: "Add a word or phrase",
                on_add: move |(text, _): (String, String)| {
                    let done = user_lists::add(&files::config_dir(), list, &[text], SystemTime::now()).map(|_| ());
                    report(state, done);
                },
                on_remove: move |text: String| report(state, user_lists::remove(&files::config_dir(), list, &text)),
                on_import: move |_| {
                    spawn(async move {
                        let Some(file) = rfd::AsyncFileDialog::new().add_filter("Text", &["txt"]).pick_file().await else {
                            return;
                        };
                        let text = String::from_utf8_lossy(&file.read().await).into_owned();
                        let done = user_lists::add(&files::config_dir(), list, &user_lists::lines_of(&text), SystemTime::now());
                        match done {
                            Ok(n) => {
                                lists_changed(state);
                                state.notice.set(Some(format!("Added {n} words.")));
                            }
                            Err(e) => state.notice.set(Some(e)),
                        }
                    });
                },
                on_export: move |_| {
                    let text = exported.clone();
                    spawn(async move {
                        let name = if list == WordList::Yours { "My words.txt" } else { "Never suggest.txt" };
                        let Some(file) = rfd::AsyncFileDialog::new().set_file_name(name).save_file().await else {
                            return;
                        };
                        if let Err(e) = file.write(text.as_bytes()).await {
                            state.notice.set(Some(format!("Couldn't export the list: {e}")));
                        }
                    });
                },
            }
        }
    }
}

/// The setter's own definitions: shown first in an answer's Definition
/// column, and the other way round for the definition itself.
#[component]
fn Definitions() -> Element {
    let state = use_context::<AppState>();
    let _ = (state.build.reloads)();
    let rows: Vec<ListRow> = user_lists::definitions(&files::config_dir())
        .into_iter()
        .map(|(answer, defs)| ListRow {
            text: answer.to_uppercase(),
            detail: defs.join(", "),
            when: String::new(),
        })
        .collect();
    rsx! {
        SettingsPage { title: "Definitions",
            ListEditor {
                rows,
                placeholder: "Answer",
                second_placeholder: "What defines it, with commas".to_string(),
                on_add: move |(answer, defs): (String, String)| {
                    report(state, user_lists::set_definitions(&files::config_dir(), &answer, &defs));
                },
                on_remove: move |answer: String| {
                    report(state, user_lists::set_definitions(&files::config_dir(), &answer, ""));
                },
            }
        }
    }
}

#[component]
fn Abbreviations() -> Element {
    let state = use_context::<AppState>();
    let _ = (state.build.reloads)();
    let rows: Vec<ListRow> = user_lists::abbreviations(&files::config_dir())
        .into_iter()
        .map(|(letters, meanings)| ListRow {
            text: letters.to_uppercase(),
            detail: meanings.join(", "),
            when: String::new(),
        })
        .collect();
    rsx! {
        SettingsPage { title: "Abbreviations",
            ListEditor {
                rows,
                placeholder: "Letters",
                second_placeholder: "What they stand for, with commas".to_string(),
                on_add: move |(letters, meanings): (String, String)| {
                    report(state, user_lists::set_abbreviation(&files::config_dir(), &letters, &meanings));
                },
                on_remove: move |letters: String| {
                    report(state, user_lists::set_abbreviation(&files::config_dir(), &letters, ""));
                },
            }
        }
    }
}

/// The kinds of indicator, as the built-in lists name them.
fn indicator_kinds() -> Vec<(String, String)> {
    xw_engine::lists::Indicators::bundled()
        .lists
        .into_iter()
        .map(|(key, list)| (key, list.label))
        .collect()
}

#[component]
fn Indicators() -> Element {
    let state = use_context::<AppState>();
    let kinds = use_hook(indicator_kinds);
    let mut kind = use_signal(|| kinds.first().map(|(k, _)| k.clone()).unwrap_or_default());
    let _ = (state.build.reloads)();
    let mine = user_lists::indicators(&files::config_dir());
    let current = kind();
    let rows: Vec<ListRow> = mine
        .get(&current)
        .into_iter()
        .flatten()
        .map(|w| ListRow {
            text: w.clone(),
            detail: String::new(),
            when: String::new(),
        })
        .collect();
    rsx! {
        SettingsPage { title: "Indicators",
            SettingsGroup {
                SettingsRow { label: "Kind of wordplay",
                    select {
                        class: "settings-input",
                        "aria-label": "Kind of wordplay",
                        onchange: move |e| kind.set(e.value()),
                        for (key , label) in kinds.iter() {
                            option {
                                key: "{key}",
                                value: "{key}",
                                selected: *key == current,
                                {
                                    let n = mine.get(key).map_or(0, Vec::len);
                                    if n > 0 { format!("{label} ({n})") } else { label.clone() }
                                }
                            }
                        }
                    }
                }
            }
            ListEditor {
                rows,
                placeholder: "Add an indicator word or phrase",
                on_add: move |(word, _): (String, String)| {
                    report(state, user_lists::add_indicator(&files::config_dir(), &kind.peek(), &word));
                },
                on_remove: move |word: String| {
                    report(state, user_lists::remove_indicator(&files::config_dir(), &kind.peek(), &word));
                },
            }
        }
    }
}

#[component]
fn Shortcuts() -> Element {
    let state = use_context::<AppState>();
    let mut query = use_signal(String::new);
    // The action whose new keys are being recorded, and what came of it.
    let mut recording = use_signal(|| None::<Shortcut>);
    let mut outcome = use_signal(|| None::<(Shortcut, Pressed)>);
    let changes = state.settings.read().shortcuts.clone();
    let groups = key_rows(&query());
    let mut record = move |s: Shortcut| {
        outcome.set(None);
        recording.set(Some(s));
        spawn(async move {
            let mut keys = document::eval(&format!(
                "window.__setterCapturing = true;\
                 const h = (e) => {{\
                   if (['Control', 'Shift', 'Alt', 'Meta', 'AltGraph'].includes(e.key)) return;\
                   e.preventDefault(); e.stopPropagation();\
                   document.removeEventListener('keydown', h, true);\
                   window.__setterCapturing = false;\
                   dioxus.send({});\
                 }};\
                 document.addEventListener('keydown', h, true);\
                 await new Promise(() => {{}});",
                crate::bindings::read_key_js()
            ));
            if let Ok((key, alt, ctrl, shift)) = keys.recv::<(String, bool, bool, bool)>().await {
                recording.set(None);
                let pressed = Combo {
                    key,
                    ctrl,
                    alt,
                    shift,
                };
                let mut changes = state.settings.peek().shortcuts.clone();
                match judge(&changes, s, pressed) {
                    Pressed::Cancel => {}
                    Pressed::Set(c) => {
                        bindings::set(&mut changes, s, &c);
                        keep_keys(state, changes);
                    }
                    other => outcome.set(Some((s, other))),
                }
            }
        });
    };
    rsx! {
        SettingsPage { title: "Keyboard shortcuts",
            div { class: "settings-keys-bar",
                input {
                    class: "settings-input settings-input--full",
                    r#type: "search",
                    placeholder: "Filter by action or key",
                    "aria-label": "Filter by action or key",
                    value: "{query}",
                    oninput: move |e| query.set(e.value()),
                }
                if !changes.is_empty() {
                    button {
                        class: "list-editor__link",
                        r#type: "button",
                        onclick: move |_| keep_keys(state, bindings::Changes::new()),
                        "Reset all"
                    }
                }
            }
            if groups.is_empty() {
                p { class: "list-editor__empty", "No matches" }
            }
            for (group , rows) in groups {
                SettingsGroup { key: "{group}", title: group,
                    for row in rows {
                        match row {
                            KeyRow::Fixed(action, keys) => rsx! {
                                SettingsRow { key: "{action}", label: action, KeyChips { keys } }
                            },
                            KeyRow::Action(s) => {
                                let changed = changes.contains_key(s.id());
                                let detail = match outcome().filter(|(o, _)| *o == s) {
                                    Some((_, Pressed::Refused(why))) => why.to_string(),
                                    Some((_, Pressed::Clash(c, other))) => {
                                        format!("{} is already used by {}", crate::keys::label(&c.text()), other.name())
                                    }
                                    _ => String::new(),
                                };
                                let clash = match outcome() {
                                    Some((o, Pressed::Clash(c, _))) if o == s => Some(c),
                                    _ => None,
                                };
                                rsx! {
                                    SettingsRow { key: "{s.id()}", label: s.name(), detail,
                                        if let Some(c) = clash {
                                            button {
                                                class: "list-editor__link",
                                                r#type: "button",
                                                onclick: move |_| {
                                                    let mut changes = state.settings.peek().shortcuts.clone();
                                                    bindings::swap(&mut changes, s, &c);
                                                    keep_keys(state, changes);
                                                    outcome.set(None);
                                                },
                                                "Swap"
                                            }
                                            button {
                                                class: "list-editor__link",
                                                r#type: "button",
                                                onclick: move |_| outcome.set(None),
                                                "Cancel"
                                            }
                                        } else if recording() == Some(s) {
                                            span { class: "key-record", "Press the new keys" }
                                            button {
                                                class: "list-editor__link",
                                                r#type: "button",
                                                // The recorder takes Esc as "leave it".
                                                onclick: move |_| {
                                                    document::eval(
                                                        "document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))",
                                                    );
                                                },
                                                "Cancel"
                                            }
                                        } else {
                                            if changed {
                                                button {
                                                    class: "list-editor__link",
                                                    r#type: "button",
                                                    onclick: move |_| {
                                                        let mut changes = state.settings.peek().shortcuts.clone();
                                                        changes.remove(s.id());
                                                        keep_keys(state, changes);
                                                    },
                                                    "Reset"
                                                }
                                            }
                                            button {
                                                class: "key-edit",
                                                r#type: "button",
                                                title: "Change",
                                                "aria-label": "Change the keys for {s.name()}",
                                                onclick: move |_| record(s),
                                                KeyChips { keys: bindings::keys(s) }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn find_a_setting_narrows_the_sections() {
        assert_eq!(matching(""), Section::ALL);
        assert_eq!(matching("dark"), [Section::General]);
        assert_eq!(matching("never"), [Section::WordLists]);
        assert_eq!(matching("anagram"), [Section::Indicators]);
        assert_eq!(matching("keys"), [Section::Shortcuts]);
        assert!(matching("zzz").is_empty());
    }

    #[test]
    fn shortcuts_filter_by_action_or_key() {
        let undo = key_rows("undo");
        assert_eq!(undo, [("Anywhere", vec![KeyRow::Action(Shortcut::Undo)])]);
        let ctrl_j = key_rows("ctrl+j");
        assert_eq!(
            ctrl_j,
            [("Write clues", vec![KeyRow::Action(Shortcut::NextUnclued)])]
        );
        let all = key_rows("");
        let actions = all
            .iter()
            .flat_map(|(_, r)| r)
            .filter(|r| matches!(r, KeyRow::Action(_)))
            .count();
        assert_eq!(actions, Shortcut::ALL.len(), "every action can be changed");
        assert_eq!(all.last().unwrap().0, "Typing in the grid");
    }

    #[test]
    fn pressed_keys_are_judged() {
        let ch = bindings::Changes::new();
        let k = |t: &str| Combo::parse(t).unwrap();
        let esc = Combo {
            key: "escape".into(),
            ctrl: false,
            alt: false,
            shift: false,
        };
        assert_eq!(judge(&ch, Shortcut::Palette, esc), Pressed::Cancel);
        assert!(matches!(
            judge(&ch, Shortcut::Palette, k("P")),
            Pressed::Refused(_)
        ));
        assert_eq!(
            judge(&ch, Shortcut::Palette, k("Ctrl+N")),
            Pressed::Clash(k("Ctrl+N"), Shortcut::New)
        );
        assert_eq!(
            judge(&ch, Shortcut::Palette, k("Ctrl+P")),
            Pressed::Set(k("Ctrl+P"))
        );
        assert_eq!(
            judge(&ch, Shortcut::Palette, k("Ctrl+K")),
            Pressed::Set(k("Ctrl+K")),
            "its own keys are fine"
        );
    }

    #[test]
    fn when_a_word_was_added() {
        let now = SystemTime::now();
        assert_eq!(
            added(Some(now - Duration::from_secs(30)), now),
            "added today"
        );
        assert_eq!(
            added(Some(now - Duration::from_secs(3 * 3600)), now),
            "added today"
        );
        assert_eq!(
            added(Some(now - Duration::from_secs(3 * 86400)), now),
            "added 3 days ago"
        );
        assert_eq!(added(None, now), "");
    }
}
