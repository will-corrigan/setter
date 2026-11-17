# Build Grid Screen Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Build grid screen where the user picks a standard British pattern,
fills it by clicking and typing, and always sees a side panel of safe words
that fit the selected answer, filterable by word split.

**Architecture:**
- Engine (`crates/engine`): templates, British grid-rule checks, split-aware
  suggestions, and split-aware autofill. All pure and tested.
- App: pure grid-editing logic and undo history (tested), then the page,
  composed from design-system components only. The page is built by an Orca
  worker, as the Write clues page was.

**Tech Stack:** Rust 1.99, Dioxus 0.7 desktop (via `dx`), Nix dev shell.
Existing `xw-engine` fill index and autofill.

**Spec:** `docs/superpowers/specs/2026-11-17-build-grid-screen-design.md`.
Mockup: `docs/design/build-grid-mockup.html`.

## Global Constraints

- **No instruction text on screen** ("Click to place", "Ctrl+Space jumps here"
  and the like). Shortcuts go in button tooltips only.
- **No notes under suggestions.** Words that would leave a crossing answer with
  nothing that fits are not listed. Tighter words rank lower.
- **Fixed crossword colours in both themes:** white squares, black blocks, dark
  letters, light-blue answer, blue current square. These are new grid tokens
  in `tokens.css` that do not change in dark mode.
- **Word breaks:** a thick bar between words, and a short dash across the
  border for a hyphen. The answer bar's pattern shows the same breaks.
- **Right-click or Ctrl+Space** opens the words popover beside the answer. It
  has icon buttons (tooltips only) for "Fill just this answer" and "Clear this
  answer". There is no side panel, no toggle, and no separate context menu.
- **The clue list** sits beside the grid in two fixed columns. Each row is
  number, clue and enumeration; there is an amber "no clue" tag when missing.
  It scrolls vertically only. No horizontal scrollbars anywhere.
- **Splits** are stored as `Clue.enumeration`. A split that doesn't add up to
  the answer's length is ignored.
- **Design system only**, styled through tokens. New reusable pieces go in
  `app/src/ds`, each with an SSR test and a gallery entry. The guard test in
  `ds/discipline.rs` must stay green.
- Fully offline. Sentence case. Never use the words "surface" or "recipe".

## Review Focus

1. **Split box that doesn't add up** ("4,4" for a 9-letter answer): no filter
   is applied and nothing is saved. (Test in Task 2.)
2. **Typing over a letter shared with a crossing answer:** both answers update,
   and suggestions and status recompute. (Test in Task 4.)
3. **A full answer that isn't a word:** the panel shows nothing that fits, and
   the status names it. (Test in Task 1 via `check_status`.)
4. **"Fill the rest" when it's impossible:** the grid is unchanged and the
   status says why. (Test in Task 3.)
5. **Switching to Write clues after filling:** splits arrive as enumerations,
   and new answers get banks. (Test in Task 4: an edit marks the puzzle dirty
   and keeps `clues[..].enumeration`.)

---

### Task 1: Templates, British rule checks and grid status (engine)

**Files:**
- Create: `crates/engine/src/templates.rs`, `crates/engine/src/rules.rs`
- Modify: `crates/engine/src/lib.rs`
- Modify: `app/src/files.rs`, so `BRITISH_15` becomes `templates::TEMPLATES[0].rows`

**Interfaces:**
- Produces:
  - `pub struct Template { pub name: &'static str, pub rows: &'static [&'static str] }`
  - `pub const TEMPLATES: &[Template]`: six entries named "British 15 · A"
    through "British 15 · D", then "British 13 · A" and "British 13 · B". Use
    the rows in `docs/design/build-grid-mockup.html` (`TEMPLATES` there).
  - `pub enum RuleIssue { UnderChecked { light: String, checked: usize, len: usize }, DoubleUnch { light: String }, Disconnected }`
    (Serialize). `light` is the label, e.g. "3d".
  - `pub fn british_rules(grid: &Grid) -> Vec<RuleIssue>`. The rules:
    - Every light has `checked * 2 >= len`.
    - No two adjacent unchecked squares within a light.
    - All open squares are connected.
  - `pub struct GridStatus { pub filled: usize, pub total: usize, pub non_words: Vec<String>, pub unfillable: Vec<String>, pub repeats: Vec<String>, pub rules: Vec<RuleIssue> }`
  - `pub fn grid_status(lex: &Lexicon, index: &FillIndex, grid: &Grid) -> GridStatus`.
    Labels are "3 down" style. `unfillable` lists partly filled lights whose
    pattern has 0 matches.

- [ ] **Step 1: Write failing tests**
  - `every_template_passes_british_rules`: `british_rules` is empty for all 6
    templates, and every template is rotationally symmetric.
  - `unchecked_run_is_reported`: a fully open 3×3 grid (`["...","...","..."]`)
    passes. A grid `["....", "####", "...."]` reports `UnderChecked` for every
    light.
  - `disconnected_grid_is_reported`: `["..#..", "..#..", "#####", "..#..", "..#.."]`
    gives `Disconnected`.
  - `status_names_problems`: on a 15×15 template with a non-word across, a
    repeated word and a stuck light:
    - `non_words` contains its "N across" label,
    - `repeats` contains the word,
    - `unfillable` contains the stuck light,
    - `filled` and `total` are counted.
- [ ] **Step 2:** Run `nix develop --command cargo test -p xw-engine templates rules`.
  Expected: FAIL.
- [ ] **Step 3:** Implement. If a template fails the rules, fix the template's
  rows, keeping it symmetric, and ledger it.
- [ ] **Step 4:** Rerun the tests, then `cargo test --workspace`. Expected: PASS.
- [ ] **Step 5: Commit:** `feat(engine): grid templates, British rule checks, grid status`

### Task 2: Split-aware suggestions (engine)

**Files:**
- Modify: `crates/engine/src/fill.rs`

**Interfaces:**
- Produces:
  - `pub enum WordCount { Any, One, Two, ThreePlus }`
  - `pub struct Shape { pub enumeration: Option<String>, pub words: WordCount }`
    (Default = Any, no enumeration)
  - `pub fn shape_for(enumeration: &str, len: usize) -> Option<String>`: returns
    the normalised split when its numbers add up to `len` and it contains `,` or
    `-`. Otherwise `None`, since a plain length is no constraint.
  - `pub fn safe_suggestions(lex: &Lexicon, index: &FillIndex, grid: &Grid, light_id: &str, shape: &Shape, limit: usize) -> Vec<Suggestion>`.
    Returns only viable entries whose `Word::enumeration()` matches the shape.
    Sort by `(rank / 1000, Reverse(min_crossing))`: popularity first, and
    within a band of 1,000 ranks, words leaving their crossings more room come
    first.

- [ ] **Step 1: Write failing tests**
  - `shape_for_validates`:
    - `shape_for("4,5", 9) == Some("4,5")`
    - `shape_for("4,4", 9) == None`
    - `shape_for("9", 9) == None`
    - `shape_for(" 3 - 4 ", 7) == Some("3-4")`
  - `split_filter`: on an empty 9-letter row grid, `Shape { enumeration: Some("4,5") }`
    gives all `enumeration == "4,5"` and is non-empty. `WordCount::Two` gives
    only 2-word entries. `WordCount::One` gives no spaces or hyphens.
  - `only_safe_words`: on `Grid::from_rows(&["...", ".#.", "..."])`, every
    result has `viable`.
- [ ] **Step 2:** Run `cargo test -p xw-engine fill`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run `cargo test --workspace`. Expected: PASS.
- [ ] **Step 5: Commit:** `feat(engine): split-aware safe suggestions`

### Task 3: Split-aware autofill (engine)

**Files:**
- Modify: `crates/engine/src/fill.rs`

**Interfaces:**
- Produces: `pub fn autofill_shaped(lex, index, grid, shapes: &HashMap<String, Shape>, time_limit, randomness, cancel) -> AutofillResult`.
  Candidates for a light with a shape must match it. `autofill` becomes
  `autofill_shaped` with an empty map.

- [ ] **Step 1: Write failing tests**
  - `autofill_respects_splits`: a one-row 9-letter grid with shape "4,5"
    fills, and the chosen entry's enumeration is "4,5".
  - `impossible_fill_leaves_grid_unchanged`: `Lexicon::from_words(&["cat"])`
    on a 3×3 open grid gives `ok == false` and `grid ==` the input.
- [ ] **Step 2:** Run `cargo test -p xw-engine autofill`. Expected: FAIL.
- [ ] **Step 3:** Implement by filtering candidates in `Search::run` with the
  light's shape.
- [ ] **Step 4:** Run `cargo test --workspace`. Expected: PASS.
- [ ] **Step 5: Commit:** `feat(engine): autofill honours word splits`

### Task 4: Grid editing logic and undo (app, pure)

**Files:**
- Create: `app/src/grid_edit.rs`, `app/src/history.rs`

**Interfaces:**
- Consumes: `xw_engine::grid::{Grid, Dir, Light}`, `xw_engine::puzzle::Puzzle`
- Produces:
  - `pub struct Cursor { pub cell: usize, pub dir: Dir }`
  - `pub fn click(grid: &Grid, c: Cursor, cell: usize) -> Cursor`: the same
    cell toggles the direction if the other direction has a light. A new cell
    keeps the direction if a light exists there, else switches.
  - `pub fn type_letter(p: &mut Puzzle, c: Cursor, ch: char) -> Cursor`: writes
    the uppercase letter and advances within the light.
  - `pub fn backspace(p: &mut Puzzle, c: Cursor) -> Cursor`: if the cell is
    empty, steps back and then clears.
  - `pub fn arrow(grid: &Grid, c: Cursor, dr: i32, dc: i32) -> Cursor`: skips
    blocks and sets the direction from the axis.
  - `pub fn next_light(grid: &Grid, c: Cursor, delta: i32) -> Cursor`: goes to
    the first empty cell of the next light, wrapping.
  - `pub fn place(p: &mut Puzzle, light_id: &str, entry_display: &str)`: writes
    the letters and sets `clues[light].enumeration` to the entry's split, or
    clears it if single.
  - `pub fn clear_light(p: &mut Puzzle, light_id: &str)`: clears letters that
    don't belong to a *full* crossing answer.
  - `pub fn set_split(p: &mut Puzzle, light_id: &str, text: &str) -> bool`:
    applies `fill::shape_for`. Returns false and changes nothing if invalid.
  - `pub enum Break { Word, Hyphen }` and
    `pub fn word_breaks(light: &Light, enumeration: &str) -> Vec<(usize, Break)>`.
    These are the cells after which a break falls, e.g. "4,5" →
    `[(cells[3], Word)]` and "3-4" → `[(cells[2], Hyphen)]`. An invalid or
    plain split gives `[]`. The grid component draws a right-edge bar for
    across lights and a bottom-edge bar for down lights.
  - `pub fn display_with_split(letters: &str, enumeration: &str) -> String`:
    "zobrist" with "3,4" gives "zob rist", with "3-4" gives "zob-rist", and
    with "7" or an invalid split gives "zobrist".
  - In `files.rs`: `pub fn add_own_word(text: &str) -> Result<(), String>` and
    `add_own_word_in(dir, text)`. They append a line to `words.txt` without
    duplicates; the caller then reloads the engine data.
  - `history.rs`: `pub struct History<T> { .. }` with `new(cap)`,
    `push(&mut self, before: T)`, `undo(&mut self, current: T) -> Option<T>`,
    `redo(&mut self, current: T) -> Option<T>`. Capacity 200. A push clears
    redo.

- [ ] **Step 1: Write failing tests**, covering each function on
  `Grid::from_rows(&["cat", "a#o", "bye"])`. Include:
  - Typing over a shared letter changes both answers' patterns.
  - `set_split(…, "4,4")` on a 9-letter light returns false and leaves the
    enumeration as it was.
  - `place` of "cart horse" sets "4,5".
  - Undo, redo and the capacity cap.
  - `word_breaks` for "4,5", "2,1,3", "3-4", "9" and "4,4" (the last two give
    `[]`).
  - `display_with_split` cases as above.
  - `add_own_word_in` on a temp dir: added once even if called twice. After
    `load_engine_data_from(dir)` the word is in the lexicon and `grid_status`
    no longer lists it in `non_words`.
- [ ] **Step 2:** Run `cargo test -p setter grid_edit history`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run `cargo test --workspace`. Expected: PASS.
- [ ] **Step 5: Commit:** `feat(app): grid editing logic and undo history`

### Task 5: The Build grid page (Orca worker)

**Files:**
- Create: `app/src/pages/build_grid.rs`
- Create new ds components:
  - `ds/crossword_grid.rs` (the grid, with selection, separators and fixed
    colours),
  - `ds/word_popover.rs` (header with the wand and eraser icon buttons,
    search, word-count chips via `Segmented { pills }`, the list),
  - `ds/clue_sheet.rs` (the clue list) and `ds/icon.rs`,
  - `ds/template_picker.rs` (thumbnail grid),
- Modify: `shell.rs` (the Build grid mode renders the page; top bar gets New
  puzzle, Fill the rest, Undo/Redo with tooltips), `tokens.css` (grid tokens),
  `components.css`, `gallery.rs`.

**Behaviour:** the left column holds the answer bar, grid and status. The
right column holds the clue list, a new ds component `ds/clue_sheet.rs`:
`ClueSheet { across: Vec<ClueRow>, down: Vec<ClueRow>, current: Option<String>, on_select }`,
where `ClueRow { key, number, clue: Option<String>, enumeration }`. It gets an
SSR test (no answer text rendered; "no clue" tag present) and a gallery entry.
The words popover is a new ds component `ds/word_popover.rs`. It is anchored to
the selected answer and has icon buttons via a new `ds/icon.rs` (inline SVG
wand and eraser, `aria-label` and tooltip). Then the spec's Layout section and the mockup, using the Task 1–4
interfaces:
- `grid_status` for the status line,
- `safe_suggestions` for the popover (limit 120, filtered by the search box
  prefix),
- An **Add to my words** action next to each status `non_words` entry. It uses
  `display_with_split` and `add_own_word`, reloads the engine data off the UI
  thread, then recomputes status and suggestions.
- `autofill_shaped` for "Fill the rest" and for "Fill just this answer"
  (only that light open),
- `grid_edit` and `History` for editing.
- Run engine work off the UI thread, as the Write clues page does.

**Done means:**
- `cargo test --workspace` and `dx build` pass.
- An SSR test renders the page for a small fixed puzzle (grid, panel, status
  present; no `hint` or instruction text).
- Screenshots in light and dark show the grid identical in both, plus word
  breaks after placing a phrase.
- Rulings are ledgered and the commit reads `feat(app): Build grid page`.
