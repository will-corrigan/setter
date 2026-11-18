//! Shared app state, and the pure logic behind the pages.

use crate::answers::{self, Answer};
use crate::clue_draft;
use crate::ds::word_popover::WordEntry;
use crate::files::{self, EngineData, Settings, Theme};
use crate::grid_edit::{self, Cursor};
use crate::grid_fill;
use crate::grid_view::Room;
use crate::history::History;
use dioxus::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, LazyLock, OnceLock, PoisonError, RwLock};
use std::time::Instant;
use xw_engine::bank::{self, Column, DeviceKind, DeviceOption, DeviceView, Tile, Tone};
use xw_engine::fill::{FillIndex, FillOptions, WordCount};
use xw_engine::puzzle::{Clue, Puzzle};
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
            files::load_engine_data_from(&crate::files::scratch_dir("test-data"))
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

/// One option's picks, by column.
pub fn option_picks(
    picks: &Picks,
    light: &str,
    device: DeviceKind,
    option: usize,
    columns: usize,
) -> HashMap<usize, String> {
    (0..columns)
        .filter_map(|i| {
            picks
                .get(&pick_key(light, device, option, i))
                .map(|w| (i, w.clone()))
        })
        .collect()
}

/// The clue's annotation from the picks in one option's columns.
pub fn annotation_for(
    picks: &Picks,
    light: &str,
    device: DeviceKind,
    option: usize,
    columns: &DeviceOption,
) -> String {
    let picked: Vec<(usize, String)> = (0..columns.columns.len())
        .filter_map(|i| {
            picks
                .get(&pick_key(light, device, option, i))
                .map(|w| (i, w.clone()))
        })
        .collect();
    crate::annotate::annotation(device, columns, &picked)
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

/// The answer as uncoloured tiles (blank squares stay blank).
pub fn plain_tiles(pattern: &str) -> Vec<Tile> {
    vec![Tile {
        letters: pattern.to_uppercase(),
        tone: None,
        struck: false,
    }]
}

// ---------- theme ----------

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
    SaveAs,
    New,
    Open,
    Export,
    Undo,
    Redo,
    /// Open or close the command palette.
    Palette,
    NextUnclued,
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
pub const SHORTCUTS: [(&str, bool, bool, bool, Shortcut); 12] = [
    ("arrowleft", true, false, false, Shortcut::PreviousAnswer),
    ("arrowright", true, false, false, Shortcut::NextAnswer),
    ("s", false, true, false, Shortcut::Save),
    ("s", false, true, true, Shortcut::SaveAs),
    ("n", false, true, false, Shortcut::New),
    ("o", false, true, false, Shortcut::Open),
    ("e", false, true, false, Shortcut::Export),
    ("k", false, true, false, Shortcut::Palette),
    ("j", false, true, false, Shortcut::NextUnclued),
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
           const typing = !!(e.target.closest && e.target.closest('input, textarea, [contenteditable=true]')) && !(e.target.closest && e.target.closest('.clue-field__editor'));\
           if (combos.some(([k, a, c, s, t]) => k === key && a === alt && c === ctrl && s === shift && (t || !typing))) {{\
             e.preventDefault();\
             dioxus.send([key, alt, ctrl, shift]);\
           }}\
         }});\
         await new Promise(() => {{}});",
        combos.join(",")
    )
}

// ---------- unsaved work ----------

/// Is there work on screen that isn't saved? Anything that differs from the
/// clean copy counts: letters, splits, clues, the title. Recovered work has
/// no clean copy, so it counts until it is saved.
pub fn unsaved(puzzle: Option<&Puzzle>, pristine: Option<&Puzzle>) -> bool {
    match (puzzle, pristine) {
        (None, _) => false,
        (Some(p), Some(clean)) => p != clean,
        (Some(_), None) => true,
    }
}

// ---------- app state ----------

/// A word bank per answer pattern, kept so going back to an answer is instant.
pub type Banks = HashMap<String, Rc<Vec<DeviceView>>>;

/// Everything undo brings back: the puzzle, the picks, and where you were.
#[derive(Clone, PartialEq, Debug)]
pub struct Snap {
    puzzle: Puzzle,
    picks: Picks,
    selection: Selection,
    cursor: Cursor,
    mode: Mode,
    /// Explanations written by picks, so they keep following picks after undo.
    auto_notes: HashMap<String, String>,
    /// Where the definition goes, so undoing the switch flips it back.
    definition_last: bool,
}

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

/// The words found for the popover, and what they were found for: the
/// answer, its letters and split, the search text and the word-count chip.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Found {
    pub light: String,
    pub pattern: String,
    pub split: String,
    pub query: String,
    pub words: WordCount,
    pub entries: Vec<WordEntry>,
}

/// What the Build grid page keeps between renders.
#[derive(Clone, Copy, PartialEq)]
pub struct BuildState {
    /// The selected square, and the direction being filled.
    pub cursor: Signal<Cursor>,
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
    /// The grid size the picker opens at, if one was chosen on Home.
    pub picker_size: Signal<Option<usize>>,
    /// The status under the grid, once worked out.
    pub status: Signal<Option<GridStatus>>,
    /// A fill is running; the grid waits for it.
    pub working: Signal<bool>,
    /// Set to stop the running fill.
    pub stop: Signal<Arc<std::sync::atomic::AtomicBool>>,
    /// Goes up each time the word list is loaded again.
    pub reloads: Signal<u32>,
}

#[derive(Clone, Copy, PartialEq)]
pub struct AppState {
    /// `None` while the first puzzle is being made.
    pub puzzle: Signal<Option<Puzzle>>,
    /// Where the puzzle is saved, if it has been.
    pub path: Signal<Option<PathBuf>>,
    /// Differs from how it was when last opened or saved.
    pub dirty: Memo<bool>,
    /// The puzzle as it was when last shown or saved; `None` for recovered
    /// work. Autosave keeps whatever differs from it.
    pub pristine: Signal<Option<Puzzle>>,
    /// Changes that can be undone and redone: the grid, the clues, the picks.
    pub history: Signal<History<Snap>>,
    /// Each clue's explanation as the picks last wrote it; one the setter has
    /// changed since is theirs and left alone.
    pub auto_notes: Signal<HashMap<String, String>>,
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
    /// Home is showing: the list of puzzles, instead of a puzzle.
    pub home: Signal<bool>,
    /// The command palette is open.
    pub palette_open: Signal<bool>,
    /// A newer version, downloaded and ready to install.
    pub update_ready: Signal<Option<String>>,
    /// How many puzzles have been shown: work started for one puzzle checks
    /// it is still on screen before it lands.
    pub shown_count: Signal<u64>,
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

/// The answer through `cell`: the one going the way `current` goes if two
/// cross there; the other if `current` is already that one.
pub fn light_through(
    lights: &[xw_engine::grid::Light],
    cell: usize,
    current: &str,
) -> Option<String> {
    let through: Vec<&xw_engine::grid::Light> =
        lights.iter().filter(|l| l.cells.contains(&cell)).collect();
    let same_way = current.ends_with(",a");
    let here = through.iter().find(|l| l.id == current);
    let pick = match here {
        // Clicking inside the answer on screen: keep it.
        Some(l) => Some(*l),
        None => through
            .iter()
            .find(|l| (l.dir == xw_engine::grid::Dir::Across) == same_way)
            .or(through.first())
            .copied(),
    };
    pick.map(|l| l.id.clone())
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
    let pristine = use_signal(|| None::<Puzzle>);
    let dirty = use_memo(move || unsaved(puzzle.read().as_ref(), pristine.read().as_ref()));
    let filling = use_signal(|| false);
    let ready = use_memo(move || puzzle.read().is_some() && !filling());
    let build = BuildState {
        cursor: use_signal(|| Cursor {
            cell: 0,
            dir: xw_engine::grid::Dir::Across,
        }),
        words_open: use_signal(|| false),
        query: use_signal(String::new),
        words: use_signal(WordCount::default),
        room: use_signal(Room::default),
        found: use_signal(|| None),
        picker_open: use_signal(|| false),
        picker_size: use_signal(|| None),
        status: use_signal(|| None),
        working: use_signal(|| false),
        stop: use_signal(|| Arc::new(std::sync::atomic::AtomicBool::new(false))),
        reloads: use_signal(|| 0),
    };
    let state = AppState {
        puzzle,
        path: use_signal(|| None),
        dirty,
        pristine,
        auto_notes: use_signal(HashMap::new),
        history: use_signal(|| History::new(grid_fill::UNDO_STEPS)),
        mode: use_signal(|| Mode::WriteClues),
        selection,
        picks: use_signal(Picks::new),
        banks: use_signal(Banks::new),
        settings: use_signal(settings),
        notice: use_signal(|| None),
        filling,
        credits: use_signal(|| false),
        home: use_signal(|| false),
        palette_open: use_signal(|| false),
        update_ready: use_signal(|| None),
        shown_count: use_signal(|| 0),
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
    /// Show a puzzle, starting at its first answer, as the clean copy: it
    /// matches the file at `path`, or is new.
    pub fn show(mut self, puzzle: Puzzle, path: Option<PathBuf>) {
        // A fill still running was for the puzzle going away: stop it, and
        // let its result know it is too late.
        self.build
            .stop
            .peek()
            .store(true, std::sync::atomic::Ordering::Relaxed);
        *self.shown_count.write() += 1;
        // A sample still being made is no longer wanted either.
        self.filling.set(false);
        let first = answers::answers(&puzzle)
            .first()
            .map(|a| a.light_id.clone())
            .unwrap_or_default();
        self.picks.write().clear();
        self.selection.set(Selection::at(first));
        self.build.cursor.set(grid_fill::start_cursor(&puzzle.grid));
        self.history.set(History::new(grid_fill::UNDO_STEPS));
        self.build.words_open.set(false);
        self.build.picker_open.set(false);
        self.build.picker_size.set(None);
        self.home.set(false);
        self.build.query.set(String::new());
        self.build.found.set(None);
        self.build.status.set(None);
        self.auto_notes.write().clear();
        self.pristine.set(Some(puzzle.clone()));
        self.puzzle.set(Some(puzzle));
        self.path.set(path);
    }

    /// Show work recovered from the autosave, with the file it was a change
    /// to, if any. Nothing clean to compare with: it stays unsaved until it
    /// is saved.
    pub fn recover(mut self, puzzle: Puzzle, path: Option<PathBuf>) {
        self.show(puzzle, path);
        self.pristine.set(None);
    }

    /// `puzzle` was written to `path`: it is the clean copy now. (Typing
    /// since it was taken still counts as unsaved.)
    pub fn saved_to(mut self, path: PathBuf, puzzle: Puzzle) {
        self.pristine.set(Some(puzzle));
        self.path.set(Some(path));
    }

    /// A new puzzle, signed with the setter's name.
    pub fn signed(self, mut puzzle: Puzzle) -> Puzzle {
        puzzle.author = self.settings.peek().setter_name.clone();
        puzzle
    }

    pub fn set_title(self, title: String) {
        self.change(Some("title"), |s| s.change_details(|p| p.title = title));
    }

    /// The setter's name, kept for new puzzles too.
    pub fn set_setter(mut self, name: String) {
        if self.settings.peek().setter_name != name {
            self.settings.write().setter_name = name.clone();
            files::save_settings(&self.settings.peek());
        }
        self.change(Some("setter"), |s| s.change_details(|p| p.author = name));
    }

    fn change_details(mut self, change: impl FnOnce(&mut Puzzle)) {
        let Some(mut puzzle) = self.puzzle.peek().clone() else {
            return;
        };
        let before = (puzzle.title.clone(), puzzle.author.clone());
        change(&mut puzzle);
        if (puzzle.title.clone(), puzzle.author.clone()) != before {
            self.puzzle.set(Some(puzzle));
        }
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

    /// Change the puzzle. What it was before goes on the undo stack, unless
    /// the change left everything as it was.
    pub fn edit<R>(self, change: impl FnOnce(&mut Puzzle) -> R) -> Option<R> {
        self.edit_grouped(None, change)
    }

    /// Like `edit`, but quick changes in the same `group` (typing one
    /// answer's letters) undo together.
    pub fn edit_grouped<R>(
        self,
        group: Option<&str>,
        change: impl FnOnce(&mut Puzzle) -> R,
    ) -> Option<R> {
        self.change(group, |mut s| {
            let mut puzzle = s.puzzle.peek().clone()?;
            let result = change(&mut puzzle);
            if Some(&puzzle) != s.puzzle.peek().as_ref() {
                s.puzzle.set(Some(puzzle));
            }
            Some(result)
        })
    }

    /// The state undo would bring back, as it is now.
    fn snap(self) -> Option<Snap> {
        Some(Snap {
            puzzle: self.puzzle.peek().clone()?,
            picks: self.picks.peek().clone(),
            selection: self.selection.peek().clone(),
            cursor: *self.build.cursor.peek(),
            mode: *self.mode.peek(),
            auto_notes: self.auto_notes.peek().clone(),
            definition_last: self.settings.peek().definition_last,
        })
    }

    /// Run `f`; if it changed the puzzle or the picks, what was there before
    /// becomes an undo step. With a `group`, a quick run of changes in that
    /// group (typing a clue) is one step.
    fn change<R>(mut self, group: Option<&str>, f: impl FnOnce(Self) -> R) -> R {
        let before = self.snap();
        let result = f(self);
        let after = self.snap();
        if let (Some(before), Some(after)) = (before, after) {
            if before.puzzle != after.puzzle || before.picks != after.picks {
                let mut history = self.history.write();
                match group {
                    Some(g) => history.record(before, g, Instant::now()),
                    None => history.push(before),
                }
            }
        }
        result
    }

    /// Take back the last change, and go back to where it was made.
    pub fn undo(self) {
        self.travel(History::undo);
    }

    pub fn redo(self) {
        self.travel(History::redo);
    }

    fn travel(mut self, step: fn(&mut History<Snap>, Snap) -> Option<Snap>) {
        let Some(now) = self.snap() else {
            return;
        };
        let Some(snap) = step(&mut self.history.write(), now) else {
            return;
        };
        self.picks.set(snap.picks);
        self.selection.set(snap.selection);
        self.build.cursor.set(snap.cursor);
        self.auto_notes.set(snap.auto_notes);
        if self.settings.peek().definition_last != snap.definition_last {
            self.settings.write().definition_last = snap.definition_last;
            files::save_settings(&self.settings.peek());
        }
        if *self.mode.peek() != snap.mode {
            self.build.words_open.set(false);
            self.mode.set(snap.mode);
        }
        self.puzzle.set(Some(snap.puzzle));
    }

    /// Go to an answer, on whichever page is showing.
    pub fn go_to(mut self, light: &str) {
        let cursor = self
            .puzzle
            .peek()
            .as_ref()
            .and_then(|p| grid_fill::cursor_on(&p.grid, light));
        if let Some(c) = cursor {
            self.build.cursor.set(c);
        }
        // Already there: keep the device and option chosen for it.
        if self.selection.peek().light != light {
            self.selection.set(Selection::at(light));
        }
    }

    /// Go to the next answer with no clue yet (stays put if all have one).
    pub fn next_unclued(self) {
        let next = self.puzzle.peek().as_ref().and_then(|p| {
            answers::next_unclued(&answers::answers(p), p, &self.selection.peek().light)
        });
        if let Some(light) = next {
            self.go_to(&light);
        }
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

    /// Go to the answer through a square of the grid: the one running the
    /// same way as the answer on screen if there is one, else the other.
    pub fn select_cell(self, cell: usize) {
        let light = self.puzzle.peek().as_ref().and_then(|p| {
            let current = self.selection.peek().light.clone();
            light_through(&p.grid.lights(), cell, &current)
        });
        if let Some(l) = light {
            self.go_to(&l);
        }
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

    /// Pick or un-pick a word. The clue's explanation follows the picks in
    /// that option, unless the setter has written their own.
    pub fn toggle_pick(self, key: PickKey, item: &str) {
        self.change(None, |s| s.pick_and_place(key, item));
    }

    fn pick_and_place(mut self, key: PickKey, item: &str) {
        let old = self.picks.peek().get(&key).cloned();
        toggle_pick(&mut self.picks.write(), key.clone(), item);
        let new = self.picks.peek().get(&key).cloned();
        let (light, device, option, column) = key;
        let Some(shown) = self.shown() else {
            return;
        };
        let Some(found) = shown
            .devices
            .iter()
            .find(|d| d.kind == device)
            .and_then(|d| d.options.get(option))
        else {
            return;
        };
        // The picked word goes into the clue, or out of it.
        let picked = option_picks(
            &self.picks.peek(),
            &light,
            device,
            option,
            found.columns.len(),
        );
        let order = clue_draft::order(&found.columns, self.settings.peek().definition_last);
        let text = self.clue_field(&light, |c| c.text.clone());
        let text = clue_draft::place(
            &text,
            &order,
            &picked,
            column,
            old.as_deref(),
            new.as_deref(),
        );
        self.set_clue_field(&light, text, |c| &mut c.text);

        let note = annotation_for(&self.picks.peek(), &light, device, option, found);
        let current = self.clue_field(&light, |c| c.annotation.clone());
        let written_by_picks = self.auto_notes.peek().get(&light) == Some(&current);
        if current.is_empty() || written_by_picks {
            self.auto_notes.write().insert(light.clone(), note.clone());
            self.set_clue_field(&light, note, |c| &mut c.annotation);
        }
    }

    /// Put picked definitions at the end of clues, or the start, and move
    /// the one in the clue on screen.
    pub fn set_definition_last(self, last: bool) {
        self.change(None, |s| s.move_definition_to(last));
    }

    fn move_definition_to(mut self, last: bool) {
        if self.settings.peek().definition_last != last {
            self.settings.write().definition_last = last;
            files::save_settings(&self.settings.peek());
        }
        let Some(shown) = self.shown() else {
            return;
        };
        let Some(found) = shown.option() else {
            return;
        };
        let light = self.selection.peek().light.clone();
        let picked = option_picks(
            &self.picks.peek(),
            &light,
            shown.device,
            shown.option,
            found.columns.len(),
        );
        let definition = found
            .columns
            .iter()
            .position(|c| c.tone == Tone::Definition)
            .and_then(|i| picked.get(&i));
        if let Some(def) = definition {
            let text = self.clue_field(&light, |c| c.text.clone());
            let moved = clue_draft::move_definition(&text, def, last, &picked);
            self.set_clue_field(&light, moved, |c| &mut c.text);
        }
    }

    /// The setter dragged the clue's pieces about: keep their order, and
    /// tidy spaces and capitals.
    pub fn arrange_clue(self, text: String) {
        self.change(None, |s| s.arrange(text));
    }

    fn arrange(self, text: String) {
        let light = self.selection.peek().light.clone();
        let picked = self
            .shown()
            .and_then(|shown| {
                shown.option().map(|o| {
                    option_picks(
                        &self.picks.peek(),
                        &light,
                        shown.device,
                        shown.option,
                        o.columns.len(),
                    )
                })
            })
            .unwrap_or_default();
        self.set_clue_field(&light, clue_draft::arrange(&text, &picked), |c| &mut c.text);
    }

    pub fn clue_text(self) -> String {
        let light = self.selection.read().light.clone();
        self.read_clue(&light, |c| c.text.clone())
    }

    /// How the clue on screen works, shown to solvers once they finish.
    pub fn clue_note(self) -> String {
        let light = self.selection.read().light.clone();
        self.read_clue(&light, |c| c.annotation.clone())
    }

    /// Typing in the clue. A quick run of typing undoes as one step.
    pub fn set_clue_text(self, text: String) {
        let light = self.selection.peek().light.clone();
        self.change(Some(&format!("clue:{light}")), |s| {
            s.set_clue_field(&light, text, |c| &mut c.text)
        });
    }

    /// The setter's own explanation; picks no longer change it.
    pub fn set_clue_note(self, note: String) {
        let light = self.selection.peek().light.clone();
        self.change(Some(&format!("note:{light}")), |s| {
            s.set_clue_field(&light, note, |c| &mut c.annotation)
        });
    }

    fn read_clue(self, light: &str, get: impl Fn(&Clue) -> String) -> String {
        self.puzzle
            .read()
            .as_ref()
            .and_then(|p| p.clues.get(light))
            .map(get)
            .unwrap_or_default()
    }

    fn clue_field(self, light: &str, get: impl Fn(&Clue) -> String) -> String {
        self.puzzle
            .peek()
            .as_ref()
            .and_then(|p| p.clues.get(light))
            .map(get)
            .unwrap_or_default()
    }

    /// Change one text of a clue, writing to the puzzle only if it differs.
    fn set_clue_field(
        mut self,
        light: &str,
        value: String,
        field: impl Fn(&mut Clue) -> &mut String,
    ) {
        let same = self
            .puzzle
            .peek()
            .as_ref()
            .map(|p| {
                let mut clue = p.clues.get(light).cloned().unwrap_or_default();
                *field(&mut clue) == value
            })
            .unwrap_or(true);
        if same {
            return;
        }
        if let Some(p) = self.puzzle.write().as_mut() {
            *field(p.clues.entry(light.to_string()).or_default()) = value;
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
    fn plain_tiles_keep_blanks() {
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
    fn each_theme_has_a_script() {
        assert!(theme_script(Theme::System).starts_with("delete "));
        assert!(theme_script(Theme::Dark).ends_with("dataset.theme = 'dark'"));
        assert!(theme_script(Theme::Light).ends_with("dataset.theme = 'light'"));
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
        assert_eq!(shortcut("s", false, true, true), Some(Shortcut::SaveAs));
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
        // In the clue box, Ctrl+Z is the app's undo (the box redraws itself,
        // so the browser's own undo can't follow it).
        assert!(js.contains(".clue-field__editor"), "{js}");
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
    fn any_change_from_the_clean_copy_is_unsaved() {
        let clean = puzzle();
        let mut lettered = clean.clone();
        grid_edit::place(&mut lettered, "0,0,a", "cot");
        let mut titled = clean.clone();
        titled.title = "Prize".into();
        assert!(!unsaved(Some(&clean), Some(&clean)));
        assert!(unsaved(Some(&lettered), Some(&clean)), "letters alone");
        assert!(unsaved(Some(&titled), Some(&clean)), "the title alone");
        assert!(unsaved(Some(&clean), None), "recovered work");
        assert!(!unsaved(None, None));
    }

    #[test]
    fn undoing_back_to_the_saved_grid_is_saved_again() {
        let said = after(
            |state| {
                state.edit(|p| grid_edit::place(p, "0,0,a", "cot"));
                state.undo();
            },
            |state| format!("{}", (state.dirty)()),
        );
        assert_eq!(said, "false");
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
                state.show(puzzle(), None);
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
        let history = state.history.read();
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
    fn undo_steps_back_through_clues_and_grid_and_goes_where_they_were() {
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
                // Undo the clue first: the grid change stays.
                state.undo();
            },
            |state| {
                format!(
                    "{} [{}] {} cell={}",
                    first_row(state),
                    state.clue_text(),
                    can(state),
                    (state.build.cursor)().cell
                )
            },
        );
        assert_eq!(undone, "cot [] undo=true redo=true cell=6");
        let twice = after(
            |state| {
                state.edit(|p| grid_edit::place(p, "0,0,a", "cot"));
                state.set_clue_text("Bed".into());
                state.undo();
                state.undo();
            },
            |state| {
                format!(
                    "{} [{}] {}",
                    first_row(state),
                    state.clue_text(),
                    can(state)
                )
            },
        );
        assert_eq!(twice, "cat [] undo=false redo=true");
        let redone = after(
            |state| {
                state.edit(|p| grid_edit::place(p, "0,0,a", "cot"));
                state.set_clue_text("Bed".into());
                state.undo();
                state.undo();
                state.redo();
                state.redo();
            },
            |state| {
                format!(
                    "{} [{}] {}",
                    first_row(state),
                    state.clue_text(),
                    can(state)
                )
            },
        );
        assert_eq!(redone, "cot [Bed] undo=true redo=false");
    }

    #[test]
    fn typing_a_clue_quickly_undoes_as_one_step() {
        let said = after(
            |state| {
                for text in ["G", "Go", "Gol", "Gold"] {
                    state.set_clue_text(text.into());
                }
                state.undo();
            },
            |state| format!("[{}] {}", state.clue_text(), can(state)),
        );
        assert_eq!(said, "[] undo=false redo=true");
    }

    #[test]
    fn a_pick_and_the_words_it_put_in_the_clue_undo_together() {
        let said = with_charade_text(|state| {
            state.toggle_pick(pick_key("0,0,a", DeviceKind::Charade, 0, 0), "gold");
            state.undo();
        });
        assert_eq!(said, "[] picks=0");
    }

    #[test]
    fn showing_another_puzzle_forgets_the_old_ones_history() {
        let fresh = after(
            |mut state| {
                state.edit(|p| grid_edit::place(p, "0,0,a", "cot"));
                state.build.words_open.set(true);
                state.build.picker_open.set(true);
                state.show(puzzle(), None);
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
                state.show(puzzle(), None);
                state.step(1);
                state.set_clue_text("Farewell".into());
            });
            let answer = (state.answer)().map(|a| a.pattern).unwrap_or_default();
            let dirty = (state.dirty)();
            rsx! { "{answer}|{state.clue_text()}|{dirty}" }
        });
        assert_eq!(html, "bye|Farewell|true");
    }

    #[test]
    fn annotation_comes_from_the_option_s_picks() {
        let option = DeviceOption {
            label: String::new(),
            tiles: vec![],
            columns: charade_columns(),
            reads_as: String::new(),
        };
        let mut picks = Picks::new();
        let key = |c| pick_key("0,0,a", DeviceKind::Charade, 0, c);
        assert_eq!(
            annotation_for(&picks, "0,0,a", DeviceKind::Charade, 0, &option),
            ""
        );
        toggle_pick(&mut picks, key(0), "gold");
        toggle_pick(&mut picks, key(1), "box");
        toggle_pick(&mut picks, key(2), "band");
        // Another answer's picks don't count.
        toggle_pick(
            &mut picks,
            pick_key("2,0,a", DeviceKind::Charade, 0, 0),
            "art",
        );
        assert_eq!(
            annotation_for(&picks, "0,0,a", DeviceKind::Charade, 0, &option),
            "OR (gold) + CHEST (box); definition: band"
        );
    }

    #[test]
    fn picking_words_writes_the_clue_s_annotation() {
        let said = after(
            |mut state| {
                let pattern = (state.answer)().unwrap().pattern;
                let mut charade = device(DeviceKind::Charade, 1, 1);
                charade.options[0].columns = charade_columns();
                state.banks.write().insert(pattern, Rc::new(vec![charade]));
                state.toggle_pick(pick_key("0,0,a", DeviceKind::Charade, 0, 0), "gold");
                state.toggle_pick(pick_key("0,0,a", DeviceKind::Charade, 0, 1), "box");
            },
            |state| {
                let puzzle = state.puzzle.read();
                let clue = puzzle.as_ref().unwrap().clues.get("0,0,a").cloned();
                format!(
                    "{}|{}",
                    clue.unwrap_or_default().annotation,
                    (state.dirty)()
                )
            },
        );
        assert_eq!(said, "OR (gold) + CHEST (box)|true");
    }

    /// Show a charade bank for the first answer, then run `act`.
    fn with_charade(act: impl FnOnce(AppState) + 'static) -> String {
        after(
            move |mut state| {
                let pattern = (state.answer)().unwrap().pattern;
                let mut charade = device(DeviceKind::Charade, 1, 1);
                charade.options[0].columns = charade_columns();
                state.banks.write().insert(pattern, Rc::new(vec![charade]));
                act(state);
            },
            |state| state.clue_note(),
        )
    }

    /// A charade bank for the first answer; reports the clue and how many picks.
    fn with_charade_text(act: impl FnOnce(AppState) + 'static) -> String {
        after(
            move |mut state| {
                let pattern = (state.answer)().unwrap().pattern;
                let mut charade = device(DeviceKind::Charade, 1, 1);
                charade.options[0].columns = charade_columns();
                state.banks.write().insert(pattern, Rc::new(vec![charade]));
                act(state);
            },
            |state| format!("[{}] picks={}", state.clue_text(), state.picks.read().len()),
        )
    }

    /// Like `with_charade`, but reports the clue text.
    fn clue_after(act: impl FnOnce(AppState) + 'static) -> String {
        after(
            move |mut state| {
                let pattern = (state.answer)().unwrap().pattern;
                let mut charade = device(DeviceKind::Charade, 1, 1);
                charade.options[0].columns = charade_columns();
                state.banks.write().insert(pattern, Rc::new(vec![charade]));
                act(state);
            },
            |state| state.clue_text(),
        )
    }

    #[test]
    fn picked_words_go_into_the_clue_and_swap_in_place() {
        let key = |c| pick_key("0,0,a", DeviceKind::Charade, 0, c);
        let text = clue_after(move |state| {
            state.toggle_pick(key(0), "gold");
            state.toggle_pick(key(1), "box");
            state.toggle_pick(key(3), "with");
            state.toggle_pick(key(2), "band");
            state.toggle_pick(key(0), "or");
        });
        assert_eq!(text, "Band or with box");
    }

    #[test]
    fn the_definition_side_moves_the_clue_s_definition() {
        let key = |c| pick_key("0,0,a", DeviceKind::Charade, 0, c);
        let text = clue_after(move |state| {
            state.toggle_pick(key(0), "gold");
            state.toggle_pick(key(2), "band");
            state.set_definition_last(true);
            state.toggle_pick(key(1), "box");
        });
        assert_eq!(text, "Gold box band");
    }

    #[test]
    fn un_picking_everything_clears_the_explanation() {
        let note = with_charade(|state| {
            let key = pick_key("0,0,a", DeviceKind::Charade, 0, 0);
            state.toggle_pick(key.clone(), "gold");
            state.toggle_pick(key, "gold");
        });
        assert_eq!(note, "");
    }

    #[test]
    fn the_setter_s_own_explanation_is_left_alone() {
        let note = with_charade(|state| {
            state.toggle_pick(pick_key("0,0,a", DeviceKind::Charade, 0, 0), "gold");
            state.set_clue_note("OR + CHEST, my way".into());
            state.toggle_pick(pick_key("0,0,a", DeviceKind::Charade, 0, 1), "box");
        });
        assert_eq!(note, "OR + CHEST, my way");
        // One from the file counts as the setter's too.
        let note = with_charade(|state| {
            state.set_clue_note("From the file".into());
            state.toggle_pick(pick_key("0,0,a", DeviceKind::Charade, 0, 0), "gold");
        });
        assert_eq!(note, "From the file");
    }

    #[test]
    fn after_undo_the_explanation_still_follows_the_picks() {
        let key = |c| pick_key("0,0,a", DeviceKind::Charade, 0, c);
        let note = with_charade(move |state| {
            state.toggle_pick(key(0), "gold");
            state.toggle_pick(key(1), "box");
            state.undo();
            state.toggle_pick(key(2), "band");
        });
        assert!(
            note.contains("BAND") || note.to_lowercase().contains("band"),
            "{note}"
        );
    }

    #[test]
    fn undoing_the_definition_switch_flips_it_back() {
        let key = |c| pick_key("0,0,a", DeviceKind::Charade, 0, c);
        let text = clue_after(move |state| {
            state.toggle_pick(key(0), "gold");
            state.toggle_pick(key(2), "band");
            let before = state.clue_text();
            state.set_definition_last(true);
            assert_ne!(state.clue_text(), before);
            state.undo();
            assert!(!state.settings.peek().definition_last);
            assert_eq!(state.clue_text(), before);
        });
        assert!(text.starts_with("Band"), "{text}");
    }

    #[test]
    fn going_to_the_answer_already_on_screen_keeps_its_device() {
        let said = after(
            |mut state| {
                let anagram = state.selection.peek().with_device(DeviceKind::Anagram);
                state.selection.set(anagram);
                state.go_to("0,0,a");
            },
            |state| format!("{:?}", state.selection.read().device),
        );
        assert_eq!(said, "Some(Anagram)");
    }

    #[test]
    fn showing_a_puzzle_stops_a_fill_and_counts_the_change() {
        let said = after(
            |state| {
                let flag = state.build.stop.peek().clone();
                state.show(puzzle(), None);
                assert!(flag.load(std::sync::atomic::Ordering::Relaxed));
            },
            |state| format!("{}", state.shown_count.read()),
        );
        assert_eq!(said, "2");
    }

    #[test]
    fn title_and_setter_are_edited_and_the_setter_remembered() {
        let said = after(
            |state| {
                state.set_title("Prize 12".into());
                state.set_setter("Quixote".into());
            },
            |state| {
                let puzzle = state.puzzle.read();
                let p = puzzle.as_ref().unwrap();
                format!(
                    "{}|{}|{}|{}",
                    p.title,
                    p.author,
                    state.settings.read().setter_name,
                    (state.dirty)()
                )
            },
        );
        assert_eq!(said, "Prize 12|Quixote|Quixote|true");
    }

    #[test]
    fn fresh_puzzles_are_signed_by_the_remembered_setter() {
        let said = after(
            |mut state| {
                state.settings.write().setter_name = "Quixote".into();
                let fresh = state.signed(puzzle());
                state.show(fresh, None);
            },
            |state| state.puzzle.read().as_ref().unwrap().author.clone(),
        );
        assert_eq!(said, "Quixote");
    }

    #[test]
    fn the_clean_copy_follows_shows_saves_and_recoveries() {
        let said = after(
            |state| {
                state.edit(|p| grid_edit::place(p, "0,0,a", "cot"));
            },
            |state| {
                let step = crate::autosave::step(
                    state.puzzle.read().as_ref(),
                    state.pristine.read().as_ref(),
                );
                format!("{step:?}")
            },
        );
        assert_eq!(said, "Write");
        let shown = after(
            |_| {},
            |state| {
                format!(
                    "{:?}",
                    crate::autosave::step(
                        state.puzzle.read().as_ref(),
                        state.pristine.read().as_ref()
                    )
                )
            },
        );
        assert_eq!(shown, "Remove");
        let saved = after(
            |state| {
                state.edit(|p| grid_edit::place(p, "0,0,a", "cot"));
                let now = state.puzzle.peek().clone().unwrap();
                state.saved_to("/tmp/x.xword".into(), now);
            },
            |state| {
                format!(
                    "{:?} {}",
                    crate::autosave::step(
                        state.puzzle.read().as_ref(),
                        state.pristine.read().as_ref()
                    ),
                    (state.dirty)()
                )
            },
        );
        assert_eq!(saved, "Remove false");
        let recovered = after(
            |state| state.recover(puzzle(), None),
            |state| {
                format!(
                    "{:?} {}",
                    crate::autosave::step(
                        state.puzzle.read().as_ref(),
                        state.pristine.read().as_ref()
                    ),
                    (state.dirty)()
                )
            },
        );
        assert_eq!(recovered, "Write true");
    }

    #[test]
    fn a_square_of_the_mini_grid_goes_to_the_answer_running_the_same_way() {
        let grid = xw_engine::grid::Grid::from_rows(&["cat", "a#o", "bye"]);
        let lights = grid.lights();
        // Square 2 (top right) is in 1 across and 2 down.
        assert_eq!(light_through(&lights, 2, "2,0,a").as_deref(), Some("0,0,a"));
        assert_eq!(light_through(&lights, 2, "0,0,d").as_deref(), Some("0,2,d"));
        // Inside the answer on screen: stays.
        assert_eq!(light_through(&lights, 2, "0,2,d").as_deref(), Some("0,2,d"));
        assert_eq!(light_through(&lights, 4, "0,0,a"), None, "a block");
    }
}
