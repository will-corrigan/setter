//! Shared app state, and the pure logic behind the pages.

use crate::answers::{self, Answer};
use crate::ds::word_popover::WordEntry;
use crate::files::{self, EngineData, Settings, Theme};
use crate::grid_edit::{self, Cursor};
use crate::grid_fill::{self, Snap};
use crate::grid_view::Room;
use crate::history::History;
use dioxus::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, LazyLock, OnceLock, PoisonError, RwLock};
use xw_engine::bank::{self, Column, DeviceKind, DeviceOption, DeviceView, Tile, Tone};
use xw_engine::fill::{FillIndex, FillOptions, WordCount};
use xw_engine::puzzle::Puzzle;
use xw_engine::rules::GridStatus;

// ---------- engine data ----------

/// The words that may go in a grid: the whole word list with its phrases,
/// most popular first. Names are left out, as in the sample grid.
pub const GRID_WORDS: FillOptions = FillOptions {
    max_rank: u32::MAX,
    allow_proper: false,
    allow_phrases: true,
};

/// Lexicon, lists and thesaurus, plus the index used to fill grids.
pub struct Engine {
    data: EngineData,
    fill: OnceLock<FillIndex>,
}

impl std::ops::Deref for Engine {
    type Target = EngineData;

    fn deref(&self) -> &EngineData {
        &self.data
    }
}

impl Engine {
    fn load() -> Engine {
        // Tests must not read or write the real config folder.
        let data = if cfg!(test) {
            files::load_engine_data_from(&std::env::temp_dir().join("setter-test-data"))
        } else {
            files::load_engine_data()
        };
        Engine {
            data,
            fill: OnceLock::new(),
        }
    }

    /// The index for filling grids, built on first use. Slow enough to keep
    /// off the UI thread.
    pub fn fill(&self) -> &FillIndex {
        self.fill
            .get_or_init(|| FillIndex::build(&self.data.lex, GRID_WORDS))
    }
}

static ENGINE: LazyLock<RwLock<Arc<Engine>>> =
    LazyLock::new(|| RwLock::new(Arc::new(Engine::load())));

/// The engine data in use. Loading takes a moment, so the first call must
/// happen off the UI thread.
pub fn engine() -> Arc<Engine> {
    ENGINE
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Load the engine data again, after the user's own word list changed. Work
/// already under way keeps the data it started with.
pub fn reload_engine() {
    let fresh = Arc::new(Engine::load());
    *ENGINE.write().unwrap_or_else(PoisonError::into_inner) = fresh;
}

/// Every device's material for one answer pattern. Slow enough to keep off
/// the UI thread.
pub fn build_bank(pattern: &str) -> Vec<DeviceView> {
    let data = engine();
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
    // Every picked part; whether they "add up" is the device's rule
    // (xw_engine::bank::required_parts), applied by highlight::checks_for.
    let parts: Vec<(String, Tone)> = all.iter().filter(|(_, t)| is_part(*t)).cloned().collect();

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
    Undo,
    Redo,
    /// Open or close the words popover.
    Words,
}

impl Shortcut {
    /// Does it also work while typing in a text box? Undo and redo don't:
    /// there, Ctrl+Z belongs to the text.
    pub fn while_typing(self) -> bool {
        !matches!(self, Shortcut::Undo | Shortcut::Redo)
    }
}

/// (key, Alt held, Ctrl held, Shift held, action). The page's key listener
/// and `shortcut` both read this table.
pub const SHORTCUTS: [(&str, bool, bool, bool, Shortcut); 6] = [
    ("arrowleft", true, false, false, Shortcut::PreviousAnswer),
    ("arrowright", true, false, false, Shortcut::NextAnswer),
    ("s", false, true, false, Shortcut::Save),
    ("z", false, true, false, Shortcut::Undo),
    ("z", false, true, true, Shortcut::Redo),
    (" ", false, true, false, Shortcut::Words),
];

pub fn shortcut(key: &str, alt: bool, ctrl: bool, shift: bool) -> Option<Shortcut> {
    let key = key.to_lowercase();
    SHORTCUTS
        .iter()
        .find(|(k, a, c, s, _)| *k == key && *a == alt && *c == ctrl && *s == shift)
        .map(|s| s.4)
}

/// JavaScript that listens for the shortcut keys anywhere in the window,
/// stops the webview's own handling of them, and sends
/// `[key, alt, ctrl, shift]`. Each combo ends with whether it also works
/// while typing in a text box.
pub fn shortcut_script() -> String {
    let combos: Vec<String> = SHORTCUTS
        .iter()
        .map(|(k, a, c, s, action)| format!("[\"{k}\",{a},{c},{s},{}]", action.while_typing()))
        .collect();
    format!(
        "const combos = [{}];\
         document.addEventListener('keydown', (e) => {{\
           const key = e.key.toLowerCase(), alt = e.altKey, ctrl = e.ctrlKey || e.metaKey, shift = e.shiftKey;\
           const typing = !!(e.target.closest && e.target.closest('input, textarea'));\
           if (combos.some(([k, a, c, s, t]) => k === key && a === alt && c === ctrl && s === shift && (t || !typing))) {{\
             e.preventDefault();\
             dioxus.send([key, alt, ctrl, shift]);\
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

/// The words found for the popover, and the answer they were found for.
#[derive(Clone, PartialEq, Debug)]
pub struct Found {
    pub light: String,
    pub pattern: String,
    pub entries: Vec<WordEntry>,
}

/// What the Build grid page keeps between renders.
#[derive(Clone, Copy, PartialEq)]
pub struct BuildState {
    /// The selected square, and the direction being filled.
    pub cursor: Signal<Cursor>,
    /// Grid changes that can be undone and redone.
    pub history: Signal<History<Snap>>,
    /// The words popover is showing.
    pub words_open: Signal<bool>,
    /// What is typed in the popover's search box.
    pub query: Signal<String>,
    /// The popover's word-count chip.
    pub words: Signal<WordCount>,
    /// Room for the popover, measured as it opens.
    pub room: Signal<Room>,
    /// The words that fit, once found.
    pub found: Signal<Option<Found>>,
    /// The template picker is showing.
    pub picker_open: Signal<bool>,
    /// The status under the grid, once worked out.
    pub status: Signal<Option<GridStatus>>,
    /// A fill is running; the grid waits for it.
    pub working: Signal<bool>,
    /// Goes up each time the word list is loaded again.
    pub reloads: Signal<u32>,
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
    /// The credits are showing instead of the page.
    pub credits: Signal<bool>,
    /// There is a puzzle to show, and no new one on its way.
    pub ready: Memo<bool>,
    /// The answer on screen.
    pub answer: Memo<Option<Answer>>,
    /// (answers with a clue, all answers)
    pub clued: Memo<(usize, usize)>,
    /// (answers with every square filled, all answers)
    pub filled: Memo<(usize, usize)>,
    pub build: BuildState,
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
    let filled = use_memo(move || {
        puzzle
            .read()
            .as_ref()
            .map(answers::filled_count)
            .unwrap_or_default()
    });
    let filling = use_signal(|| false);
    let ready = use_memo(move || puzzle.read().is_some() && !filling());
    let build = BuildState {
        cursor: use_signal(|| Cursor {
            cell: 0,
            dir: xw_engine::grid::Dir::Across,
        }),
        history: use_signal(|| History::new(grid_fill::UNDO_STEPS)),
        words_open: use_signal(|| false),
        query: use_signal(String::new),
        words: use_signal(WordCount::default),
        room: use_signal(Room::default),
        found: use_signal(|| None),
        picker_open: use_signal(|| false),
        status: use_signal(|| None),
        working: use_signal(|| false),
        reloads: use_signal(|| 0),
    };
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
        credits: use_signal(|| false),
        ready,
        answer,
        clued,
        filled,
        build,
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
        self.build.cursor.set(grid_fill::start_cursor(&puzzle.grid));
        self.build.history.set(History::new(grid_fill::UNDO_STEPS));
        self.build.words_open.set(false);
        self.build.picker_open.set(false);
        self.build.query.set(String::new());
        self.build.found.set(None);
        self.build.status.set(None);
        self.puzzle.set(Some(puzzle));
        self.path.set(path);
        self.dirty.set(!saved);
    }

    /// Switch page, staying on the same answer.
    pub fn set_mode(mut self, mode: Mode) {
        if let Some(p) = self.puzzle.peek().as_ref() {
            let cursor = *self.build.cursor.peek();
            let on_grid = grid_edit::current_light(&p.grid, cursor).map(|l| l.id);
            let chosen = self.selection.peek().light.clone();
            if on_grid.as_deref() != Some(chosen.as_str()) {
                match (mode, on_grid) {
                    (Mode::WriteClues, Some(light)) => self.selection.set(Selection::at(light)),
                    (Mode::BuildGrid, _) => {
                        if let Some(c) = grid_fill::cursor_on(&p.grid, &chosen) {
                            self.build.cursor.set(c);
                        }
                    }
                    _ => {}
                }
            }
        }
        self.build.words_open.set(false);
        self.mode.set(mode);
    }

    /// Change the grid. What it was before goes on the undo stack and the
    /// puzzle counts as unsaved, unless the change left everything as it was.
    pub fn edit<R>(mut self, change: impl FnOnce(&mut Puzzle) -> R) -> Option<R> {
        let cursor = *self.build.cursor.peek();
        let mut puzzle = self.puzzle.peek().clone()?;
        let before = grid_fill::snapshot(&puzzle, cursor);
        let result = change(&mut puzzle);
        if grid_fill::snapshot(&puzzle, cursor) != before {
            self.build.history.write().push(before);
            self.puzzle.set(Some(puzzle));
            if !*self.dirty.peek() {
                self.dirty.set(true);
            }
        }
        Some(result)
    }

    /// Take back the last grid change. Clue text is left alone.
    pub fn undo(self) {
        self.travel(History::undo);
    }

    pub fn redo(self) {
        self.travel(History::redo);
    }

    fn travel(mut self, step: fn(&mut History<Snap>, Snap) -> Option<Snap>) {
        let Some(mut puzzle) = self.puzzle.peek().clone() else {
            return;
        };
        let now = grid_fill::snapshot(&puzzle, *self.build.cursor.peek());
        let Some(snap) = step(&mut self.build.history.write(), now) else {
            return;
        };
        grid_fill::restore(&mut puzzle, &snap);
        self.build.cursor.set(snap.cursor);
        self.puzzle.set(Some(puzzle));
        self.dirty.set(true);
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
    fn parts_hold_every_picked_part() {
        let cols = charade_columns();
        let mut picks = Picks::new();
        toggle_pick(
            &mut picks,
            pick_key("0,0,a", DeviceKind::Charade, 0, 0),
            "gold",
        );
        let m = marks(&picks, "0,0,a", DeviceKind::Charade, 0, &cols);
        assert_eq!(m.all, vec![("gold".to_string(), Tone::P1)]);
        assert_eq!(m.parts, vec![("gold".to_string(), Tone::P1)]);
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
            shortcut("ArrowLeft", true, false, false),
            Some(Shortcut::PreviousAnswer)
        );
        assert_eq!(
            shortcut("ArrowRight", true, false, false),
            Some(Shortcut::NextAnswer)
        );
        assert_eq!(shortcut("s", false, true, false), Some(Shortcut::Save));
        assert_eq!(shortcut("S", false, true, false), Some(Shortcut::Save));
        assert_eq!(shortcut("ArrowRight", false, false, false), None);
        assert_eq!(shortcut("s", false, false, false), None);
        assert_eq!(shortcut("s", true, true, false), None);
        assert_eq!(shortcut("s", false, true, true), None);
    }

    #[test]
    fn undo_redo_and_the_words_popover_have_shortcuts() {
        assert_eq!(shortcut("z", false, true, false), Some(Shortcut::Undo));
        // Shift makes the key a capital.
        assert_eq!(shortcut("Z", false, true, true), Some(Shortcut::Redo));
        assert_eq!(shortcut(" ", false, true, false), Some(Shortcut::Words));
        assert_eq!(shortcut("z", false, false, false), None);
        assert_eq!(shortcut(" ", false, false, false), None);
    }

    #[test]
    fn undo_and_redo_leave_text_boxes_alone() {
        assert!(!Shortcut::Undo.while_typing());
        assert!(!Shortcut::Redo.while_typing());
        for everywhere in [
            Shortcut::Save,
            Shortcut::Words,
            Shortcut::PreviousAnswer,
            Shortcut::NextAnswer,
        ] {
            assert!(everywhere.while_typing(), "{everywhere:?}");
        }
    }

    #[test]
    fn shortcut_script_listens_for_every_shortcut() {
        let js = shortcut_script();
        for (key, alt, ctrl, shift, action) in SHORTCUTS {
            let typing = action.while_typing();
            assert!(
                js.contains(&format!("[\"{key}\",{alt},{ctrl},{shift},{typing}]")),
                "{js}"
            );
        }
        assert!(js.contains("preventDefault") && js.contains("dioxus.send"));
        assert!(js.contains("input, textarea"), "{js}");
    }

    #[test]
    fn the_engine_can_be_loaded_again() {
        let before = engine();
        assert!(before.lex.contains("orchestra"));
        // The grid index covers the whole list, phrases included.
        assert_eq!(before.fill().count(b"orch?stra"), 1);
        assert!(before.fill().count(b"cart?orse") >= 1);
        reload_engine();
        let after = engine();
        assert!(!Arc::ptr_eq(&before, &after));
        assert!(after.lex.contains("orchestra"));
        // Work that started with the old data can finish with it.
        assert!(before.lex.contains("cat"));
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

    /// Run `act` on a fresh app state showing `puzzle()`, then report with `say`.
    fn after(
        act: impl FnOnce(AppState) + 'static,
        say: impl Fn(AppState) -> String + 'static,
    ) -> String {
        crate::ds::test_render(move || {
            let state = use_app_state(Settings::default);
            use_hook(|| {
                state.show(puzzle(), None, true);
                act(state);
            });
            let text = say(state);
            rsx! { "{text}" }
        })
    }

    fn first_row(state: AppState) -> String {
        let puzzle = state.puzzle.read();
        let p = puzzle.as_ref().unwrap();
        p.grid.pattern(&p.grid.lights()[0])
    }

    fn can(state: AppState) -> String {
        let history = state.build.history.read();
        format!("undo={} redo={}", history.can_undo(), history.can_redo())
    }

    #[test]
    fn a_grid_edit_can_be_undone_and_marks_the_puzzle_unsaved() {
        let edited = after(
            |state| {
                state.edit(|p| grid_edit::place(p, "0,0,a", "cot"));
            },
            |state| format!("{} {} {}", first_row(state), (state.dirty)(), can(state)),
        );
        assert_eq!(edited, "cot true undo=true redo=false");
    }

    #[test]
    fn an_edit_that_changes_nothing_is_not_a_step() {
        let same = after(
            |state| {
                state.edit(|p| grid_edit::place(p, "0,0,a", "cat"));
            },
            |state| format!("{} {}", (state.dirty)(), can(state)),
        );
        assert_eq!(same, "false undo=false redo=false");
    }

    #[test]
    fn undo_and_redo_move_the_grid_but_never_the_clue_text() {
        let undone = after(
            |mut state| {
                state.build.cursor.set(Cursor {
                    cell: 2,
                    dir: xw_engine::grid::Dir::Down,
                });
                state.edit(|p| grid_edit::place(p, "0,0,a", "cot"));
                state.build.cursor.set(Cursor {
                    cell: 6,
                    dir: xw_engine::grid::Dir::Across,
                });
                state.set_clue_text("Bed".into());
                state.undo();
            },
            |state| {
                format!(
                    "{} {} {} cell={}",
                    first_row(state),
                    state.clue_text(),
                    can(state),
                    (state.build.cursor)().cell
                )
            },
        );
        assert_eq!(undone, "cat Bed undo=false redo=true cell=2");
        let redone = after(
            |state| {
                state.edit(|p| grid_edit::place(p, "0,0,a", "cot"));
                state.set_clue_text("Bed".into());
                state.undo();
                state.redo();
            },
            |state| format!("{} {} {}", first_row(state), state.clue_text(), can(state)),
        );
        assert_eq!(redone, "cot Bed undo=true redo=false");
    }

    #[test]
    fn showing_another_puzzle_forgets_the_old_ones_history() {
        let fresh = after(
            |mut state| {
                state.edit(|p| grid_edit::place(p, "0,0,a", "cot"));
                state.build.words_open.set(true);
                state.build.picker_open.set(true);
                state.show(puzzle(), None, true);
            },
            |state| {
                format!(
                    "{} words={} picker={}",
                    can(state),
                    (state.build.words_open)(),
                    (state.build.picker_open)()
                )
            },
        );
        assert_eq!(fresh, "undo=false redo=false words=false picker=false");
    }

    #[test]
    fn switching_page_stays_on_the_same_answer() {
        let to_clues = after(
            |mut state| {
                // 2 down, by its last square.
                state.build.cursor.set(Cursor {
                    cell: 8,
                    dir: xw_engine::grid::Dir::Down,
                });
                state.build.words_open.set(true);
                state.set_mode(Mode::WriteClues);
            },
            |state| {
                format!(
                    "{:?} {} words={}",
                    (state.mode)(),
                    state.selection.read().light,
                    (state.build.words_open)()
                )
            },
        );
        assert_eq!(to_clues, "WriteClues 0,2,d words=false");
        let to_grid = after(
            |mut state| {
                state.selection.set(Selection::at("2,0,a"));
                state.set_mode(Mode::BuildGrid);
            },
            |state| {
                let c = (state.build.cursor)();
                format!("{:?} {} {:?}", (state.mode)(), c.cell, c.dir)
            },
        );
        assert_eq!(to_grid, "BuildGrid 6 Across");
    }

    #[test]
    fn the_filled_count_follows_the_grid() {
        let counted = after(
            |state| {
                state.edit(|p| grid_edit::clear_light(p, "0,0,a"));
            },
            |state| format!("{:?}", (state.filled)()),
        );
        // CAT loses its A (C and T belong to full down answers): 3 of 4 left.
        assert_eq!(counted, "(3, 4)");
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
