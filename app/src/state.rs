//! Shared app state, and the pure logic behind the pages.

use crate::answers::{self, Answer};
use crate::files::{self, EngineData, Settings, Theme};
use dioxus::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::LazyLock;
use xw_engine::bank::{self, Column, DeviceKind, DeviceOption, DeviceView, Tile, Tone};
use xw_engine::puzzle::Puzzle;

/// Lexicon, lists and thesaurus, loaded once. Loading takes a moment, so the
/// first touch must happen off the UI thread.
pub static DATA: LazyLock<EngineData> = LazyLock::new(|| {
    // Tests must not read or write the real config folder.
    if cfg!(test) {
        files::load_engine_data_from(&std::env::temp_dir().join("setter-test-data"))
    } else {
        files::load_engine_data()
    }
});

/// Every device's material for one answer pattern. Slow enough to keep off
/// the UI thread.
pub fn build_bank(pattern: &str) -> Vec<DeviceView> {
    let data = &*DATA;
    bank::build(
        xw_engine::Ctx::new(&data.lex, &data.abbr),
        &data.ind,
        &data.th,
        pattern,
    )
}

// ---------- modes ----------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    BuildGrid,
    WriteClues,
}

impl Mode {
    pub const ALL: [Mode; 2] = [Mode::BuildGrid, Mode::WriteClues];

    pub fn label(self) -> &'static str {
        match self {
            Mode::BuildGrid => "Build grid",
            Mode::WriteClues => "Write clues",
        }
    }

    pub fn index(self) -> usize {
        Mode::ALL.iter().position(|m| *m == self).unwrap_or(0)
    }

    pub fn from_index(i: usize) -> Mode {
        Mode::ALL.get(i).copied().unwrap_or(Mode::WriteClues)
    }
}

// ---------- which answer, device and option are on screen ----------

/// The answer on screen, and the device and option chosen for it.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Selection {
    /// Light id of the answer.
    pub light: String,
    /// `None` until the user picks one: the first device with anything to offer.
    pub device: Option<DeviceKind>,
    pub option: usize,
}

impl Selection {
    /// A fresh look at one answer: default device, first option.
    pub fn at(light: impl Into<String>) -> Selection {
        Selection {
            light: light.into(),
            device: None,
            option: 0,
        }
    }

    /// The answer `delta` places along, wrapping. Device and option reset.
    pub fn stepped(&self, list: &[Answer], delta: i32) -> Selection {
        match answers::step(list, &self.light, delta) {
            Some(light) if light != self.light => Selection::at(light),
            _ => self.clone(),
        }
    }

    /// Another device for the same answer, back on its first option.
    pub fn with_device(&self, device: DeviceKind) -> Selection {
        Selection {
            light: self.light.clone(),
            device: Some(device),
            option: 0,
        }
    }

    /// Another option of the device on screen (`shown`, which may be the default).
    pub fn with_option(&self, shown: DeviceKind, option: usize) -> Selection {
        Selection {
            light: self.light.clone(),
            device: Some(shown),
            option,
        }
    }
}

/// The device shown before the user picks one: the first with a count.
pub fn default_device(devices: &[DeviceView]) -> DeviceKind {
    devices
        .iter()
        .find(|d| d.count > 0)
        .or(devices.first())
        .map(|d| d.kind)
        .unwrap_or(DeviceKind::ALL[0])
}

/// The device and option a selection lands on in this word bank. The option
/// index is clamped to what the device has.
pub fn resolve(devices: &[DeviceView], selection: &Selection) -> (DeviceKind, usize) {
    let kind = selection.device.unwrap_or_else(|| default_device(devices));
    let options = devices
        .iter()
        .find(|d| d.kind == kind)
        .map_or(0, |d| d.options.len());
    (kind, selection.option.min(options.saturating_sub(1)))
}

pub fn device_named(name: &str) -> Option<DeviceKind> {
    DeviceKind::ALL.iter().copied().find(|k| k.name() == name)
}

// ---------- picks ----------

/// (light id, device, option index, column index)
pub type PickKey = (String, DeviceKind, usize, usize);
/// The word picked in each word-bank column, if any.
pub type Picks = HashMap<PickKey, String>;

pub fn pick_key(light: &str, device: DeviceKind, option: usize, column: usize) -> PickKey {
    (light.to_string(), device, option, column)
}

/// Pick `item` in a column; picking the same item again un-picks it.
pub fn toggle_pick(picks: &mut Picks, key: PickKey, item: &str) {
    if picks.get(&key).is_some_and(|p| p == item) {
        picks.remove(&key);
    } else {
        picks.insert(key, item.to_string());
    }
}

fn is_part(tone: Tone) -> bool {
    !matches!(tone, Tone::Definition | Tone::Indicator)
}

/// The picks for one option, as the clue needs them.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Marks {
    /// Every pick with its column's tone, in column order: what the clue colours.
    pub all: Vec<(String, Tone)>,
    /// The wordplay parts' picks, once every part column has one. Empty until
    /// then, so a half-chosen clue never claims to add up.
    pub parts: Vec<(String, Tone)>,
    /// The Definition column's pick.
    pub definition: Option<String>,
}

pub fn marks(
    picks: &Picks,
    light: &str,
    device: DeviceKind,
    option: usize,
    columns: &[Column],
) -> Marks {
    let picked = |i: usize| picks.get(&pick_key(light, device, option, i));
    let all: Vec<(String, Tone)> = columns
        .iter()
        .enumerate()
        .filter_map(|(i, c)| picked(i).map(|p| (p.clone(), c.tone)))
        .collect();
    let part_columns = columns.iter().filter(|c| is_part(c.tone)).count();
    let mut parts: Vec<(String, Tone)> = all.iter().filter(|(_, t)| is_part(*t)).cloned().collect();
    if parts.len() < part_columns {
        parts.clear();
    }
    let definition = all
        .iter()
        .find(|(_, t)| *t == Tone::Definition)
        .map(|(p, _)| p.clone());
    Marks {
        all,
        parts,
        definition,
    }
}

// ---------- the answer as text and tiles ----------

/// The answer in capitals for the navigator, blanks as a middle dot.
pub fn answer_letters(pattern: &str) -> String {
    pattern
        .chars()
        .map(|c| {
            if c == '?' {
                '·'
            } else {
                c.to_ascii_uppercase()
            }
        })
        .collect()
}

/// The answer as uncoloured tiles (blank squares stay blank).
pub fn plain_tiles(pattern: &str) -> Vec<Tile> {
    vec![Tile {
        letters: pattern.to_uppercase(),
        tone: None,
        struck: false,
    }]
}

// ---------- theme ----------

pub fn next_theme(theme: Theme) -> Theme {
    match theme {
        Theme::System => Theme::Light,
        Theme::Light => Theme::Dark,
        Theme::Dark => Theme::System,
    }
}

pub fn theme_label(theme: Theme) -> &'static str {
    match theme {
        Theme::System => "Theme: system",
        Theme::Light => "Theme: light",
        Theme::Dark => "Theme: dark",
    }
}

/// JavaScript that applies a theme to the page.
pub fn theme_script(theme: Theme) -> &'static str {
    match theme {
        Theme::System => "delete document.documentElement.dataset.theme",
        Theme::Light => "document.documentElement.dataset.theme = 'light'",
        Theme::Dark => "document.documentElement.dataset.theme = 'dark'",
    }
}

// ---------- keyboard shortcuts ----------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shortcut {
    PreviousAnswer,
    NextAnswer,
    Save,
}

/// (key, Alt held, Ctrl held, action). The page's key listener and
/// `shortcut` both read this table.
pub const SHORTCUTS: [(&str, bool, bool, Shortcut); 3] = [
    ("arrowleft", true, false, Shortcut::PreviousAnswer),
    ("arrowright", true, false, Shortcut::NextAnswer),
    ("s", false, true, Shortcut::Save),
];

pub fn shortcut(key: &str, alt: bool, ctrl: bool) -> Option<Shortcut> {
    let key = key.to_lowercase();
    SHORTCUTS
        .iter()
        .find(|(k, a, c, _)| *k == key && *a == alt && *c == ctrl)
        .map(|s| s.3)
}

/// JavaScript that listens for the shortcut keys anywhere in the window,
/// stops the webview's own handling of them, and sends `[key, alt, ctrl]`.
pub fn shortcut_script() -> String {
    let combos: Vec<String> = SHORTCUTS
        .iter()
        .map(|(k, a, c, _)| format!("[\"{k}\",{a},{c}]"))
        .collect();
    format!(
        "const combos = [{}];\
         document.addEventListener('keydown', (e) => {{\
           const key = e.key.toLowerCase(), alt = e.altKey, ctrl = e.ctrlKey || e.metaKey;\
           if (combos.some(([k, a, c]) => k === key && a === alt && c === ctrl)) {{\
             e.preventDefault();\
             dioxus.send([key, alt, ctrl]);\
           }}\
         }});\
         await new Promise(() => {{}});",
        combos.join(",")
    )
}

// ---------- replacing the puzzle ----------

/// Would replacing the puzzle throw away clues that aren't saved?
pub fn loses_work(dirty: bool, puzzle: Option<&Puzzle>) -> bool {
    dirty && puzzle.is_some_and(|p| answers::clued_count(p).0 > 0)
}

// ---------- app state ----------

/// A word bank per answer pattern, kept so going back to an answer is instant.
pub type Banks = HashMap<String, Rc<Vec<DeviceView>>>;

/// What the word bank has for the selection on screen.
pub struct Shown {
    pub devices: Rc<Vec<DeviceView>>,
    pub device: DeviceKind,
    pub option: usize,
}

impl Shown {
    pub fn view(&self) -> Option<&DeviceView> {
        self.devices.iter().find(|d| d.kind == self.device)
    }

    pub fn option(&self) -> Option<&DeviceOption> {
        self.view().and_then(|d| d.options.get(self.option))
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct AppState {
    /// `None` while the first puzzle is being made.
    pub puzzle: Signal<Option<Puzzle>>,
    /// Where the puzzle is saved, if it has been.
    pub path: Signal<Option<PathBuf>>,
    /// Edited since it was last saved or opened.
    pub dirty: Signal<bool>,
    pub mode: Signal<Mode>,
    pub selection: Signal<Selection>,
    pub picks: Signal<Picks>,
    pub banks: Signal<Banks>,
    pub settings: Signal<Settings>,
    /// One line shown under the top bar until dismissed.
    pub notice: Signal<Option<String>>,
    /// A sample grid is being filled.
    pub filling: Signal<bool>,
    /// There is a puzzle to show, and no new one on its way.
    pub ready: Memo<bool>,
    /// The answer on screen.
    pub answer: Memo<Option<Answer>>,
    /// (answers with a clue, all answers)
    pub clued: Memo<(usize, usize)>,
}

/// Create the app state and share it with every component below.
pub fn use_app_state(settings: impl FnOnce() -> Settings) -> AppState {
    let puzzle = use_signal(|| None::<Puzzle>);
    let selection = use_signal(Selection::default);
    let answers = use_memo(move || {
        puzzle
            .read()
            .as_ref()
            .map(answers::answers)
            .unwrap_or_default()
    });
    let answer = use_memo(move || {
        let light = selection.read().light.clone();
        answers.read().iter().find(|a| a.light_id == light).cloned()
    });
    let clued = use_memo(move || {
        puzzle
            .read()
            .as_ref()
            .map(answers::clued_count)
            .unwrap_or_default()
    });
    let filling = use_signal(|| false);
    let ready = use_memo(move || puzzle.read().is_some() && !filling());
    let state = AppState {
        puzzle,
        path: use_signal(|| None),
        dirty: use_signal(|| false),
        mode: use_signal(|| Mode::WriteClues),
        selection,
        picks: use_signal(Picks::new),
        banks: use_signal(Banks::new),
        settings: use_signal(settings),
        notice: use_signal(|| None),
        filling,
        ready,
        answer,
        clued,
    };
    use_context_provider(|| state);
    state
}

impl AppState {
    /// Show a puzzle, starting at its first answer. `saved` says whether it
    /// matches the file at `path`.
    pub fn show(mut self, puzzle: Puzzle, path: Option<PathBuf>, saved: bool) {
        let first = answers::answers(&puzzle)
            .first()
            .map(|a| a.light_id.clone())
            .unwrap_or_default();
        self.picks.write().clear();
        self.selection.set(Selection::at(first));
        self.puzzle.set(Some(puzzle));
        self.path.set(path);
        self.dirty.set(!saved);
    }

    pub fn step(mut self, delta: i32) {
        // Straight from the puzzle: a memo that is only peeked can be stale.
        let list = self
            .puzzle
            .peek()
            .as_ref()
            .map(answers::answers)
            .unwrap_or_default();
        let next = self.selection.peek().stepped(&list, delta);
        self.selection.set(next);
    }

    /// The word bank for the answer on screen, once it has been built.
    pub fn shown(self) -> Option<Shown> {
        let pattern = (self.answer)()?.pattern;
        let devices = self.banks.read().get(&pattern).cloned()?;
        let (device, option) = resolve(&devices, &self.selection.read());
        Some(Shown {
            devices,
            device,
            option,
        })
    }

    pub fn select_device(mut self, device: DeviceKind) {
        let next = self.selection.peek().with_device(device);
        self.selection.set(next);
    }

    /// Choose an option of the device on screen.
    pub fn select_option(mut self, shown: DeviceKind, option: usize) {
        let next = self.selection.peek().with_option(shown, option);
        self.selection.set(next);
    }

    pub fn toggle_pick(mut self, key: PickKey, item: &str) {
        toggle_pick(&mut self.picks.write(), key, item);
    }

    pub fn clue_text(self) -> String {
        let light = self.selection.read().light.clone();
        self.puzzle
            .read()
            .as_ref()
            .and_then(|p| p.clues.get(&light))
            .map(|c| c.text.clone())
            .unwrap_or_default()
    }

    pub fn set_clue_text(mut self, text: String) {
        let light = self.selection.peek().light.clone();
        if let Some(p) = self.puzzle.write().as_mut() {
            p.clues.entry(light).or_default().text = text;
        }
        if !*self.dirty.peek() {
            self.dirty.set(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xw_engine::grid::Grid;

    fn puzzle() -> Puzzle {
        let mut p = Puzzle::new(3, 3);
        p.grid = Grid::from_rows(&["cat", "a#o", "bye"]);
        p
    }

    fn device(kind: DeviceKind, count: usize, options: usize) -> DeviceView {
        DeviceView {
            kind,
            count,
            options: (0..options)
                .map(|i| DeviceOption {
                    label: format!("Option {i}"),
                    tiles: vec![],
                    columns: vec![],
                    reads_as: String::new(),
                })
                .collect(),
        }
    }

    fn column(title: &str, tone: Tone) -> Column {
        Column {
            title: title.into(),
            tone,
            wide: false,
            groups: vec![],
            marks: vec![],
        }
    }

    fn charade_columns() -> Vec<Column> {
        vec![
            column("OR", Tone::P1),
            column("CHEST", Tone::P2),
            column("Definition", Tone::Definition),
            column("Joining words", Tone::Indicator),
        ]
    }

    #[test]
    fn default_device_is_the_first_with_a_count() {
        let devices = vec![
            device(DeviceKind::AlternateLetters, 0, 0),
            device(DeviceKind::Anagram, 0, 0),
            device(DeviceKind::Charade, 3, 3),
            device(DeviceKind::Hidden, 9, 1),
        ];
        assert_eq!(default_device(&devices), DeviceKind::Charade);
    }

    #[test]
    fn default_device_falls_back_to_the_first_listed() {
        let devices = vec![
            device(DeviceKind::AlternateLetters, 0, 0),
            device(DeviceKind::Anagram, 0, 0),
        ];
        assert_eq!(default_device(&devices), DeviceKind::AlternateLetters);
        assert_eq!(default_device(&[]), DeviceKind::AlternateLetters);
    }

    #[test]
    fn resolve_uses_the_default_device_and_clamps_the_option() {
        let devices = vec![
            device(DeviceKind::Anagram, 0, 0),
            device(DeviceKind::Charade, 3, 3),
        ];
        assert_eq!(
            resolve(&devices, &Selection::at("0,0,a")),
            (DeviceKind::Charade, 0)
        );
        let chosen = Selection {
            light: "0,0,a".into(),
            device: Some(DeviceKind::Charade),
            option: 7,
        };
        assert_eq!(resolve(&devices, &chosen), (DeviceKind::Charade, 2));
        let empty = Selection::at("0,0,a").with_device(DeviceKind::Anagram);
        assert_eq!(resolve(&devices, &empty), (DeviceKind::Anagram, 0));
    }

    #[test]
    fn stepping_to_another_answer_resets_device_and_option() {
        let list = answers::answers(&puzzle());
        let here = Selection {
            light: "0,0,a".into(),
            device: Some(DeviceKind::Hidden),
            option: 2,
        };
        assert_eq!(here.stepped(&list, 1), Selection::at("2,0,a"));
        assert_eq!(here.stepped(&list, -1), Selection::at("0,2,d"));
        // Nowhere to go: nothing changes.
        assert_eq!(here.stepped(&list[..1], 1), here);
        assert_eq!(here.stepped(&[], 1), here);
    }

    #[test]
    fn choosing_a_device_resets_the_option_but_an_option_keeps_the_device() {
        let here = Selection {
            light: "0,0,a".into(),
            device: Some(DeviceKind::Hidden),
            option: 2,
        };
        let anagram = here.with_device(DeviceKind::Anagram);
        assert_eq!(
            (anagram.device, anagram.option, anagram.light.as_str()),
            (Some(DeviceKind::Anagram), 0, "0,0,a")
        );
        // Choosing an option pins the device that was showing by default.
        let pinned = Selection::at("0,0,a").with_option(DeviceKind::Charade, 1);
        assert_eq!(
            (pinned.device, pinned.option),
            (Some(DeviceKind::Charade), 1)
        );
    }

    #[test]
    fn device_names_round_trip() {
        for kind in DeviceKind::ALL {
            assert_eq!(device_named(kind.name()), Some(kind));
        }
        assert_eq!(device_named("Nonsense"), None);
    }

    #[test]
    fn pick_keys_separate_answer_device_option_and_column() {
        let base = pick_key("0,0,a", DeviceKind::Charade, 0, 0);
        assert_eq!(base, ("0,0,a".to_string(), DeviceKind::Charade, 0, 0));
        for other in [
            pick_key("2,0,a", DeviceKind::Charade, 0, 0),
            pick_key("0,0,a", DeviceKind::Anagram, 0, 0),
            pick_key("0,0,a", DeviceKind::Charade, 1, 0),
            pick_key("0,0,a", DeviceKind::Charade, 0, 1),
        ] {
            assert_ne!(base, other);
        }
    }

    #[test]
    fn picking_again_unpicks_and_another_item_replaces() {
        let mut picks = Picks::new();
        let key = pick_key("0,0,a", DeviceKind::Charade, 0, 1);
        toggle_pick(&mut picks, key.clone(), "box");
        assert_eq!(picks.get(&key).map(String::as_str), Some("box"));
        toggle_pick(&mut picks, key.clone(), "trunk");
        assert_eq!(picks.get(&key).map(String::as_str), Some("trunk"));
        toggle_pick(&mut picks, key.clone(), "trunk");
        assert!(picks.is_empty());
    }

    #[test]
    fn marks_take_each_columns_tone_in_column_order() {
        let cols = charade_columns();
        let mut picks = Picks::new();
        for (i, word) in [(3, "by"), (0, "gold"), (2, "band"), (1, "box")] {
            toggle_pick(
                &mut picks,
                pick_key("0,0,a", DeviceKind::Charade, 0, i),
                word,
            );
        }
        // Picks for another answer, device or option stay out of it.
        toggle_pick(
            &mut picks,
            pick_key("2,0,a", DeviceKind::Charade, 0, 0),
            "x",
        );
        toggle_pick(
            &mut picks,
            pick_key("0,0,a", DeviceKind::Anagram, 0, 0),
            "y",
        );
        toggle_pick(
            &mut picks,
            pick_key("0,0,a", DeviceKind::Charade, 1, 0),
            "z",
        );
        let m = marks(&picks, "0,0,a", DeviceKind::Charade, 0, &cols);
        assert_eq!(
            m.all,
            vec![
                ("gold".to_string(), Tone::P1),
                ("box".to_string(), Tone::P2),
                ("band".to_string(), Tone::Definition),
                ("by".to_string(), Tone::Indicator),
            ]
        );
        assert_eq!(
            m.parts,
            vec![
                ("gold".to_string(), Tone::P1),
                ("box".to_string(), Tone::P2)
            ]
        );
        assert_eq!(m.definition.as_deref(), Some("band"));
    }

    #[test]
    fn parts_stay_empty_until_every_part_has_a_pick() {
        let cols = charade_columns();
        let mut picks = Picks::new();
        toggle_pick(
            &mut picks,
            pick_key("0,0,a", DeviceKind::Charade, 0, 0),
            "gold",
        );
        let m = marks(&picks, "0,0,a", DeviceKind::Charade, 0, &cols);
        assert_eq!(m.all, vec![("gold".to_string(), Tone::P1)]);
        assert!(m.parts.is_empty());
        assert_eq!(m.definition, None);
        assert_eq!(
            marks(&Picks::new(), "0,0,a", DeviceKind::Charade, 0, &cols),
            Marks::default()
        );
    }

    #[test]
    fn real_columns_carry_definition_and_indicator_tones() {
        let devices = build_bank("orchestra");
        let charade = devices
            .iter()
            .find(|d| d.kind == DeviceKind::Charade)
            .unwrap();
        let cols = &charade.options[0].columns;
        let tone_of = |title: &str| cols.iter().find(|c| c.title == title).map(|c| c.tone);
        assert_eq!(tone_of("Definition"), Some(Tone::Definition));
        assert_eq!(tone_of("Joining words"), Some(Tone::Indicator));
        assert_eq!(cols[0].tone, Tone::P1);
    }

    #[test]
    fn answers_show_blanks_as_dots_and_plain_tiles() {
        assert_eq!(answer_letters("or?hes?ra"), "OR·HES·RA");
        assert_eq!(
            plain_tiles("c?t"),
            vec![Tile {
                letters: "C?T".into(),
                tone: None,
                struck: false
            }]
        );
    }

    #[test]
    fn theme_cycles_system_light_dark() {
        assert_eq!(next_theme(Theme::System), Theme::Light);
        assert_eq!(next_theme(Theme::Light), Theme::Dark);
        assert_eq!(next_theme(Theme::Dark), Theme::System);
        assert!(theme_script(Theme::System).starts_with("delete "));
        assert!(theme_script(Theme::Dark).ends_with("dataset.theme = 'dark'"));
        assert!(theme_script(Theme::Light).ends_with("dataset.theme = 'light'"));
        assert_eq!(theme_label(Theme::System), "Theme: system");
    }

    #[test]
    fn shortcuts_need_their_modifier() {
        assert_eq!(
            shortcut("ArrowLeft", true, false),
            Some(Shortcut::PreviousAnswer)
        );
        assert_eq!(
            shortcut("ArrowRight", true, false),
            Some(Shortcut::NextAnswer)
        );
        assert_eq!(shortcut("s", false, true), Some(Shortcut::Save));
        assert_eq!(shortcut("S", false, true), Some(Shortcut::Save));
        assert_eq!(shortcut("ArrowRight", false, false), None);
        assert_eq!(shortcut("s", false, false), None);
        assert_eq!(shortcut("s", true, true), None);
    }

    #[test]
    fn shortcut_script_listens_for_every_shortcut() {
        let js = shortcut_script();
        for (key, alt, ctrl, _) in SHORTCUTS {
            assert!(js.contains(&format!("[\"{key}\",{alt},{ctrl}]")), "{js}");
        }
        assert!(js.contains("preventDefault") && js.contains("dioxus.send"));
    }

    #[test]
    fn only_unsaved_clues_count_as_work_to_lose() {
        let mut p = puzzle();
        assert!(!loses_work(true, Some(&p)));
        assert!(!loses_work(true, None));
        p.clues.entry("0,0,a".into()).or_default().text = "Pet".into();
        assert!(loses_work(true, Some(&p)));
        assert!(!loses_work(false, Some(&p)));
    }

    #[test]
    fn modes_map_to_the_switch_positions() {
        assert_eq!(Mode::ALL.map(Mode::label), ["Build grid", "Write clues"]);
        assert_eq!(Mode::from_index(Mode::BuildGrid.index()), Mode::BuildGrid);
        assert_eq!(Mode::from_index(Mode::WriteClues.index()), Mode::WriteClues);
    }

    #[test]
    fn state_shows_a_puzzle_and_edits_its_clue() {
        let html = crate::ds::test_render(|| {
            let state = use_app_state(Settings::default);
            use_hook(|| {
                state.show(puzzle(), None, true);
                state.step(1);
                state.set_clue_text("Farewell".into());
            });
            let answer = (state.answer)().map(|a| a.pattern).unwrap_or_default();
            let dirty = (state.dirty)();
            rsx! { "{answer}|{state.clue_text()}|{dirty}" }
        });
        assert_eq!(html, "bye|Farewell|true");
    }
}
