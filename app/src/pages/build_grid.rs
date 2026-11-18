//! Build grid: the answer bar, the grid and its status on the left, the clue
//! sheet on the right, with the words popover and the template picker over
//! them.

use crate::components::button::{Button, ButtonSize, ButtonVariant};
use crate::ds::answer_bar::AnswerBar;
use crate::ds::clue_sheet::ClueSheet;
use crate::ds::crossword_grid::{CrosswordGrid, GridKey};
use crate::ds::empty_state::EmptyState;
use crate::ds::status_line::StatusLine;
use crate::ds::template_picker::{GridThumb, TemplatePicker};
use crate::ds::word_popover::WordPopover;
use crate::files;
use crate::grid_edit::{self, Cursor};
use crate::grid_fill;
use crate::grid_view::{self, Room};
use crate::state::{self, AppState, Found, Mode};
use dioxus::core::spawn_forever;
use dioxus::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use xw_engine::fill::WordCount;
use xw_engine::grid::{Grid, Light};
use xw_engine::puzzle::Puzzle;
use xw_engine::rules::{grid_status, light_name};
use xw_engine::templates::TEMPLATES;

/// The popover's chips, in the order the popover shows them.
const WORD_COUNTS: [WordCount; 4] = [
    WordCount::Any,
    WordCount::One,
    WordCount::Two,
    WordCount::ThreePlus,
];

/// JavaScript that measures the room for the words popover, in grid squares:
/// `[from the grid's left edge to the window's right edge, the popover's
/// width with a square for the gap beside it]`.
const MEASURE_ROOM: &str = r#"
    const area = document.querySelector('.grid-area');
    const squares = document.querySelectorAll('.xw-grid__cell');
    if (!area || squares.length < 2) return null;
    const pitch = squares[1].getBoundingClientRect().left - squares[0].getBoundingClientRect().left;
    if (!(pitch > 0)) return null;
    const probe = document.createElement('div');
    probe.style.cssText = 'position:absolute;visibility:hidden;width:var(--popover-width)';
    area.appendChild(probe);
    const width = probe.getBoundingClientRect().width;
    probe.remove();
    const room = document.documentElement.clientWidth - area.getBoundingClientRect().left;
    return [room / pitch, width / pitch + 1];
"#;

/// Typing goes to the grid.
fn focus_grid() {
    document::eval("document.querySelector('.xw-grid')?.focus({ preventScroll: true })");
}

// ---------- what is selected ----------

/// The answer being filled, read from the puzzle on screen.
fn selected(p: &Puzzle, cursor: Cursor) -> Option<Light> {
    grid_edit::current_light(&p.grid, cursor)
}

/// The answer being filled, without subscribing to anything.
fn peek_selected(state: AppState) -> Option<Light> {
    let cursor = *state.build.cursor.peek();
    state
        .puzzle
        .peek()
        .as_ref()
        .and_then(|p| selected(p, cursor))
}

fn with_grid<R>(state: AppState, read: impl FnOnce(&Grid) -> R) -> Option<R> {
    state.puzzle.peek().as_ref().map(|p| read(&p.grid))
}

/// Is the page on screen and free to change the grid? Not while a fill runs,
/// and not behind the template picker.
fn free(state: AppState) -> bool {
    *state.mode.peek() == Mode::BuildGrid
        && !*state.credits.peek()
        && !*state.home.peek()
        && !*state.filling.peek()
        && !*state.build.working.peek()
        && !*state.build.picker_open.peek()
        && state.puzzle.peek().is_some()
}

fn move_to(mut state: AppState, next: Cursor) {
    if *state.build.cursor.peek() != next {
        state.build.cursor.set(next);
    }
}

// ---------- the words popover ----------

/// Close the words popover, if it is open. The shell calls this for a click
/// anywhere in the window; clicks inside the popover never get that far.
pub fn close_words(mut state: AppState) {
    if *state.build.words_open.peek() {
        state.build.words_open.set(false);
    }
}

/// Open the popover beside the selected answer, once its room is measured.
fn open_words(mut state: AppState) {
    if peek_selected(state).is_none() {
        return;
    }
    state.build.query.set(String::new());
    state.build.found.set(None);
    spawn(async move {
        let measured = document::eval(MEASURE_ROOM).join::<Vec<f32>>().await.ok();
        let room = match measured.as_deref() {
            Some([columns, popover_columns]) => Room {
                columns: *columns,
                popover_columns: *popover_columns,
            },
            _ => Room::default(),
        };
        state.build.room.set(room);
        state.build.words_open.set(true);
    });
}

/// Words that fit (Ctrl+F): open the words for the selected answer, or close them.
pub fn toggle_words(state: AppState) {
    if !free(state) {
        return;
    }
    if *state.build.words_open.peek() {
        close_words(state);
        focus_grid();
    } else {
        open_words(state);
    }
}

// ---------- editing ----------

/// Can undo and redo act now? On either page, but not while a fill runs,
/// behind the template picker, or with Home or the credits showing.
fn can_travel(state: AppState) -> bool {
    !*state.credits.peek()
        && !*state.home.peek()
        && !*state.filling.peek()
        && !*state.build.working.peek()
        && !*state.build.picker_open.peek()
        && state.puzzle.peek().is_some()
}

pub fn undo(state: AppState) {
    if can_travel(state) {
        state.undo();
    }
}

pub fn redo(state: AppState) {
    if can_travel(state) {
        state.redo();
    }
}

/// A key pressed in the grid.
pub fn key(state: AppState, key: GridKey) {
    if !free(state) || peek_selected(state).is_none() {
        return;
    }
    let c = *state.build.cursor.peek();
    let next = match key {
        // Letters typed into one answer undo together, as a word.
        GridKey::Letter(ch) => state.edit_grouped(Some(&typing(state, c)), |p| {
            grid_edit::type_letter(p, c, ch)
        }),
        GridKey::Backspace => {
            state.edit_grouped(Some(&typing(state, c)), |p| grid_edit::backspace(p, c))
        }
        GridKey::Delete => state.edit_grouped(Some(&typing(state, c)), |p| grid_edit::delete(p, c)),
        GridKey::Move(rows, columns) => with_grid(state, |g| grid_edit::arrow(g, c, rows, columns)),
        GridKey::Turn => with_grid(state, |g| grid_edit::click(g, c, c.cell)),
        GridKey::NextAnswer => with_grid(state, |g| grid_edit::next_light(g, c, 1)),
        GridKey::PreviousAnswer => with_grid(state, |g| grid_edit::next_light(g, c, -1)),
        GridKey::Escape => {
            close_words(state);
            None
        }
    };
    if let Some(next) = next {
        move_to(state, next);
    }
}

/// A square was clicked.
fn select(state: AppState, cell: usize) {
    let c = *state.build.cursor.peek();
    if let Some(next) = with_grid(state, |g| grid_edit::click(g, c, cell)) {
        move_to(state, next);
    }
}

/// A square was right-clicked: select it, keeping the direction if it can,
/// and show the words for its answer.
fn words_at(state: AppState, cell: usize) {
    if !free(state) {
        return;
    }
    let c = *state.build.cursor.peek();
    if cell != c.cell {
        select(state, cell);
    }
    open_words(state);
}

/// A row of the clue sheet was clicked.
fn select_answer(state: AppState, light_id: &str) {
    if let Some(Some(next)) = with_grid(state, |g| grid_fill::cursor_on(g, light_id)) {
        move_to(state, next);
    }
    focus_grid();
}

/// The split box was changed. A split that doesn't add up changes nothing.
fn set_split(state: AppState, text: &str) {
    if !free(state) {
        return;
    }
    if let Some(light) = peek_selected(state) {
        state.edit(|p| grid_edit::set_split(p, &light.id, text));
    }
    focus_grid();
}

/// After an answer is filled: on to the next one with room, popover away.
fn move_on(state: AppState) {
    let c = *state.build.cursor.peek();
    if let Some(next) = with_grid(state, |g| grid_fill::next_open(g, c)) {
        move_to(state, next);
    }
    close_words(state);
    focus_grid();
}

/// The undo group for typing at the cursor: the answer being typed in.
fn typing(state: AppState, c: Cursor) -> String {
    let light = state
        .puzzle
        .peek()
        .as_ref()
        .and_then(|p| grid_edit::current_light(&p.grid, c))
        .map(|l| l.id)
        .unwrap_or_default();
    format!("grid:{light}")
}

/// A word was picked in the popover.
fn place_word(state: AppState, text: &str) {
    if !free(state) {
        return;
    }
    if let Some(light) = peek_selected(state) {
        state.edit(|p| grid_edit::place(p, &light.id, text));
        move_on(state);
    }
}

/// The eraser: clear the selected answer.
fn clear_answer(state: AppState) {
    if !free(state) {
        return;
    }
    if let Some(light) = peek_selected(state) {
        state.edit(|p| grid_edit::clear_light(p, &light.id));
    }
    close_words(state);
    focus_grid();
}

/// Put a finished fill's letters and splits into the puzzle, as one change.
/// Clue text written while it ran is kept.
fn take_fill(state: AppState, filled: &Puzzle) {
    let cursor = *state.build.cursor.peek();
    state.edit(|p| grid_fill::restore(p, &grid_fill::snapshot(filled, cursor)));
}

/// What the notice says when a fill fails: nothing if it was stopped on
/// purpose.
pub fn fill_notice(why: String, stopped: bool) -> Option<String> {
    (!stopped).then_some(why)
}

/// When a fill could only be done with rare words, which ones, so the setter
/// can swap them.
pub fn rare_notice(rare: &[String]) -> Option<String> {
    (!rare.is_empty()).then(|| format!("Filled, but with some rare words: {}.", rare.join(", ")))
}

/// Stop the fill that is running, leaving the grid as it was.
pub fn stop_fill(state: AppState) {
    state.build.stop.peek().store(true, Ordering::Relaxed);
}

/// While a fill runs. However its task ends, even dropped part-way, the fill
/// is told to stop and the grid is free again.
struct Busy {
    working: Signal<bool>,
    stop: Arc<AtomicBool>,
}

impl Drop for Busy {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.working.set(false);
    }
}

/// Run a fill off the UI thread. The grid waits until it is done or
/// stopped; if it fails, nothing changes and the notice says why.
async fn run_fill(
    mut state: AppState,
    fill: impl FnOnce(&state::Engine, &Puzzle, &AtomicBool) -> Result<Puzzle, String> + Send + 'static,
) -> bool {
    let Some(puzzle) = state.puzzle.peek().clone() else {
        return false;
    };
    let shown = *state.shown_count.peek();
    let stop = Arc::new(AtomicBool::new(false));
    state.build.stop.set(stop.clone());
    state.build.working.set(true);
    let busy = Busy {
        working: state.build.working,
        stop: stop.clone(),
    };
    let flag = stop.clone();
    let done = tokio::task::spawn_blocking(move || {
        let engine = state::engine();
        let filled = fill(&engine, &puzzle, &flag);
        let rare = filled
            .as_ref()
            .map(|f| grid_fill::rare_entries(&engine.lex, &engine.quality, &puzzle, f))
            .unwrap_or_default();
        (filled, rare)
    })
    .await
    .map(|(filled, rare)| filled.map(|f| (f, rare)));
    // Before the guard goes: it raises the flag too.
    let stopped = stop.load(Ordering::Relaxed);
    drop(busy);
    if *state.shown_count.peek() != shown {
        // Another puzzle is on screen now: this fill was for the old one.
        return false;
    }
    match done {
        Ok(Ok((filled, rare))) => {
            take_fill(state, &filled);
            if let Some(line) = rare_notice(&rare) {
                state.notice.set(Some(line));
            }
            true
        }
        Ok(Err(why)) => {
            if let Some(line) = fill_notice(why, stopped) {
                state.notice.set(Some(line));
            }
            false
        }
        Err(_) => {
            state.notice.set(Some("Couldn't fill the grid.".into()));
            false
        }
    }
}

/// "Fill the rest": every empty square, keeping the letters already there.
pub async fn fill_rest(state: AppState) {
    if !free(state) {
        return;
    }
    close_words(state);
    run_fill(state, |engine, puzzle, stop| {
        grid_fill::fill_rest(
            &engine.lex,
            engine.fill(),
            &engine.quality,
            puzzle,
            grid_fill::FILL_TIME,
            stop,
        )
    })
    .await;
    focus_grid();
}

/// The wand: fill the selected answer and nothing else.
async fn fill_answer(state: AppState) {
    if !free(state) {
        return;
    }
    let Some(light) = peek_selected(state) else {
        return;
    };
    let filled = run_fill(state, move |engine, puzzle, stop| {
        grid_fill::fill_answer(
            &engine.lex,
            engine.fill(),
            &engine.quality,
            puzzle,
            &light.id,
            grid_fill::ANSWER_TIME,
            stop,
        )
    })
    .await;
    if filled {
        move_on(state);
    }
}

/// "Add to my words": add an answer to the user's word list, then load the
/// word list again so it counts as a word everywhere.
fn add_word(mut state: AppState, light_id: &str) {
    let word = state
        .puzzle
        .peek()
        .as_ref()
        .and_then(|p| grid_fill::own_word(p, light_id));
    let Some(word) = word else {
        return;
    };
    spawn_forever(async move {
        let added = tokio::task::spawn_blocking(move || {
            files::add_own_word(&word)?;
            state::reload_engine();
            Ok::<(), String>(())
        })
        .await;
        match added {
            Ok(Ok(())) => {
                // Word banks were built from the old list.
                state.banks.write().clear();
                *state.build.reloads.write() += 1;
            }
            Ok(Err(why)) => state.notice.set(Some(why)),
            Err(_) => state.notice.set(Some("Couldn't add the word.".into())),
        }
    });
}

/// A grid was picked: start an empty puzzle on it.
fn start_on(state: AppState, template: usize) {
    if let Some(t) = TEMPLATES.get(template) {
        state.show(state.signed(grid_fill::new_puzzle(t.rows)), None);
        // A new grid is filled before it is clued, wherever it was started.
        state.set_mode(Mode::BuildGrid);
        focus_grid();
    }
}

// ---------- hooks ----------

/// Work out the status under the grid, off the UI thread, whenever the
/// letters or the word list change.
fn use_status(state: AppState) {
    let grid = use_memo(move || state.puzzle.read().as_ref().map(|p| p.grid.clone()));
    let _status = use_resource(move || {
        let grid = grid();
        let _reloads = (state.build.reloads)();
        async move {
            let Some(grid) = grid else {
                return;
            };
            let worked_out = tokio::task::spawn_blocking(move || {
                let engine = state::engine();
                grid_status(&engine.lex, engine.fill(), &grid)
            })
            .await;
            if let Ok(status) = worked_out {
                let mut shown = state.build.status;
                shown.set(Some(status));
            }
        }
    });
}

/// What the popover's list depends on.
#[derive(Clone, PartialEq)]
struct Wanted {
    grid: Grid,
    light: String,
    pattern: String,
    split: String,
    words: WordCount,
    query: String,
    reloads: u32,
}

/// Find the words for the popover, off the UI thread, while it is open.
fn use_words(state: AppState) {
    let build = state.build;
    let wanted = use_memo(move || {
        if !(build.words_open)() {
            return None;
        }
        let puzzle = state.puzzle.read();
        let p = puzzle.as_ref()?;
        let light = selected(p, (build.cursor)())?;
        Some(Wanted {
            grid: p.grid.clone(),
            pattern: p.grid.pattern(&light),
            split: grid_fill::split_of(p, &light.id).to_string(),
            light: light.id,
            words: (build.words)(),
            query: (build.query)(),
            reloads: (build.reloads)(),
        })
    });
    let _words = use_resource(move || {
        let wanted = wanted();
        async move {
            let Some(w) = wanted else {
                return;
            };
            let asked = Found {
                light: w.light.clone(),
                pattern: w.pattern.clone(),
                split: w.split.clone(),
                query: w.query.clone(),
                words: w.words,
                entries: vec![],
            };
            let search = tokio::task::spawn_blocking(move || {
                let engine = state::engine();
                grid_fill::suggest(
                    &engine.lex,
                    engine.fill(),
                    &engine.quality,
                    &w.grid,
                    &w.light,
                    &w.split,
                    w.words,
                    &w.query,
                    grid_fill::WORD_LIMIT,
                )
                .into_iter()
                .map(|s| {
                    let q = grid_fill::quality_of(&engine.lex, &engine.quality, &s);
                    (s, q)
                })
                .collect::<Vec<_>>()
            })
            .await;
            if let Ok(words) = search {
                let mut found = build.found;
                found.set(Some(Found {
                    entries: grid_view::word_entries(words),
                    ..asked
                }));
            }
        }
    });
}

/// Keep the selected answer's row in view in the clue sheet.
fn use_row_in_view(light: Memo<Option<String>>) {
    use_effect(move || {
        light();
        document::eval(
            "document.querySelector('.clue-sheet__row[aria-current]')?.scrollIntoView({ block: 'nearest' })",
        );
    });
}

// ---------- components ----------

#[component]
pub fn BuildGrid() -> Element {
    let state = use_context::<AppState>();
    use_status(state);
    use_words(state);
    let light = use_memo(move || {
        let puzzle = state.puzzle.read();
        let p = puzzle.as_ref()?;
        selected(p, (state.build.cursor)()).map(|l| l.id)
    });
    use_row_in_view(light);
    let (_, answers) = (state.filled)();
    // The grid's size, for its squares to fill the room the page has.
    let (columns, rows) = state
        .puzzle
        .read()
        .as_ref()
        .map_or((15, 15), |p| (p.grid.width, p.grid.height));
    rsx! {
        if answers == 0 {
            EmptyState { message: "This puzzle has no answers." }
        } else {
            div { class: "build-grid", style: "--columns: {columns}; --rows: {rows}",
                div { class: "build-grid__left",
                    Bar {}
                    div { class: "grid-area",
                        Squares {}
                        Words {}
                    }
                    Status {}
                }
                Clues {}
            }
        }
        Picker {}
    }
}

/// The selected answer's name, letters and split.
#[component]
fn Bar() -> Element {
    let state = use_context::<AppState>();
    let puzzle = state.puzzle.read();
    let Some(p) = puzzle.as_ref() else {
        return rsx! {};
    };
    let Some(light) = selected(p, (state.build.cursor)()) else {
        return rsx! {};
    };
    let split = grid_fill::split_of(p, &light.id);
    let (filled, total) = (state.filled)();
    let working = (state.build.working)();
    rsx! {
        div { class: "build-grid__head",
            AnswerBar {
                label: light_name(&light),
                pattern: grid_view::shown_pattern(&p.grid.pattern(&light), split),
                split: grid_view::split_text(p, &light),
                on_split: move |text: String| set_split(state, &text),
            }
            if working {
                Button {
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    onclick: move |_| stop_fill(state),
                    "Stop filling"
                }
            } else {
                Button {
                    variant: ButtonVariant::Primary,
                    size: ButtonSize::Sm,
                    disabled: filled == total,
                    onclick: move |_| {
                        spawn_forever(fill_rest(state));
                    },
                    "Fill the rest"
                }
            }
        }
    }
}

#[component]
fn Squares() -> Element {
    let state = use_context::<AppState>();
    let puzzle = state.puzzle.read();
    let Some(p) = puzzle.as_ref() else {
        return rsx! {};
    };
    rsx! {
        CrosswordGrid {
            cells: grid_view::grid_cells(p, (state.build.cursor)()),
            columns: p.grid.width,
            on_select: move |cell| select(state, cell),
            on_words: move |cell| words_at(state, cell),
            on_key: move |k| key(state, k),
        }
    }
}

/// The words popover, while it is open.
#[component]
fn Words() -> Element {
    let mut state = use_context::<AppState>();
    let build = state.build;
    if !(build.words_open)() {
        return rsx! {};
    }
    let puzzle = state.puzzle.read();
    let Some(p) = puzzle.as_ref() else {
        return rsx! {};
    };
    let Some(light) = selected(p, (build.cursor)()) else {
        return rsx! {};
    };
    let pattern = p.grid.pattern(&light);
    // Words found for another answer, other letters or split, or another
    // search, are not these: Enter mustn't place one of them.
    let split = grid_fill::split_of(p, &light.id).to_string();
    let entries = build
        .found
        .read()
        .as_ref()
        .filter(|f| {
            f.light == light.id
                && f.pattern == pattern
                && f.split == split
                && f.query == *build.query.read()
                && f.words == *build.words.read()
        })
        .map(|f| f.entries.clone());
    let chip = WORD_COUNTS
        .iter()
        .position(|w| *w == (build.words)())
        .unwrap_or(0);
    rsx! {
        WordPopover {
            label: light_name(&light),
            pattern: grid_view::shown_pattern(&pattern, grid_fill::split_of(p, &light.id)),
            at: grid_view::place_popover(&light, p.grid.width, (build.room)()),
            query: (build.query)(),
            words: chip,
            entries,
            busy: (build.working)(),
            on_query: move |text| state.build.query.set(text),
            on_words: move |i: usize| state.build.words.set(WORD_COUNTS[i.min(WORD_COUNTS.len() - 1)]),
            on_place: move |text: String| place_word(state, &text),
            // Not tied to the popover: the fill finishes even if the page goes.
            on_fill: move |_| {
                spawn_forever(fill_answer(state));
            },
            on_clear: move |_| clear_answer(state),
            on_close: move |_| {
                close_words(state);
                focus_grid();
            },
        }
    }
}

/// The checks under the grid. Empty until they have been worked out.
#[component]
fn Status() -> Element {
    let state = use_context::<AppState>();
    let puzzle = state.puzzle.read();
    let status = state.build.status.read();
    let items = match (puzzle.as_ref(), status.as_ref()) {
        (Some(p), Some(status)) => grid_view::status_items(status, p),
        _ => vec![],
    };
    rsx! {
        StatusLine { items, on_action: move |light: String| add_word(state, &light) }
    }
}

#[component]
fn Clues() -> Element {
    let state = use_context::<AppState>();
    let puzzle = state.puzzle.read();
    let Some(p) = puzzle.as_ref() else {
        return rsx! {};
    };
    let (across, down) = grid_view::clue_rows(p);
    let current = selected(p, (state.build.cursor)()).map(|l| l.id);
    rsx! {
        ClueSheet {
            across,
            down,
            current,
            on_select: move |light: String| select_answer(state, &light),
            on_write: move |light: String| {
                state.go_to(&light);
                state.set_mode(Mode::WriteClues);
            },
        }
    }
}

/// The template picker, while it is open.
#[component]
pub fn Picker() -> Element {
    let mut state = use_context::<AppState>();
    if !(state.build.picker_open)() {
        return rsx! {};
    }
    let grids: Vec<GridThumb> = TEMPLATES
        .iter()
        .map(|t| GridThumb {
            name: t.name.to_string(),
            rows: t.rows.iter().map(|r| r.to_string()).collect(),
        })
        .collect();
    rsx! {
        TemplatePicker {
            title: "New puzzle",
            grids,
            size: (state.build.picker_size)(),
            on_pick: move |i| start_on(state, i),
            on_close: move |_| {
                state.build.picker_open.set(false);
                state.build.picker_size.set(None);
                focus_grid();
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;
    use crate::files::Settings;
    use crate::state::use_app_state;
    use xw_engine::grid::Dir;

    #[test]
    fn a_fill_dropped_part_way_frees_the_grid_and_stops() {
        let html = render(|| {
            let state = use_app_state(Settings::default);
            let stop = use_hook(|| {
                let mut working = state.build.working;
                working.set(true);
                let stop = Arc::new(AtomicBool::new(false));
                drop(Busy {
                    working,
                    stop: stop.clone(),
                });
                stop
            });
            let text = format!(
                "{} {}",
                (state.build.working)(),
                stop.load(Ordering::Relaxed)
            );
            rsx! { "{text}" }
        });
        assert_eq!(html, "false true");
    }

    #[test]
    fn rare_words_in_a_fill_are_named() {
        assert_eq!(rare_notice(&[]), None);
        assert_eq!(
            rare_notice(&["EMBUS".into(), "WHOOM".into()]).as_deref(),
            Some("Filled, but with some rare words: EMBUS, WHOOM.")
        );
    }

    #[test]
    fn a_stopped_fill_says_nothing() {
        let why = "Couldn't fill the rest: stopped.".to_string();
        assert_eq!(fill_notice(why.clone(), true), None);
        assert_eq!(fill_notice(why.clone(), false), Some(why));
    }

    fn puzzle() -> Puzzle {
        let mut p = Puzzle::new(3, 3);
        p.grid = Grid::from_rows(&["cat", "a#o", "bye"]);
        p.clues.entry("0,0,a".into()).or_default().text = "Pet".into();
        p.clues.entry("0,2,d".into()).or_default().text = "Digit".into();
        p
    }

    /// Render the page for a puzzle, after `setup` has arranged the state.
    fn page(p: Puzzle, setup: impl FnOnce(AppState) + 'static) -> String {
        render(move || {
            let mut state = use_app_state(Settings::default);
            use_hook(|| {
                let engine = state::engine();
                let status = grid_status(&engine.lex, engine.fill(), &p.grid);
                state.show(p, None);
                state.mode.set(Mode::BuildGrid);
                state.build.status.set(Some(status));
                setup(state);
            });
            rsx! { BuildGrid {} }
        })
    }

    /// The words on screen in one part of the page, without the markup.
    fn text_of(html: &str, class: &str) -> String {
        let start = html
            .find(&format!(r#"class="{class}""#))
            .unwrap_or_else(|| panic!("{class} missing: {html}"));
        let open = html[..start].rfind('<').unwrap();
        let tag: String = html[open + 1..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        let (mut depth, mut at, mut out) = (0usize, open, String::new());
        while at < html.len() {
            let rest = &html[at..];
            if rest.starts_with(&format!("</{tag}>")) {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                at += tag.len() + 3;
            } else if rest.starts_with(&format!("<{tag}"))
                && rest[tag.len() + 1..].starts_with([' ', '>'])
            {
                depth += 1;
                at += rest.find('>').unwrap() + 1;
            } else if rest.starts_with('<') {
                at += rest.find('>').unwrap() + 1;
            } else {
                let end = rest.find('<').unwrap_or(rest.len());
                out.push_str(&rest[..end]);
                out.push(' ');
                at += end;
            }
        }
        out
    }

    #[test]
    fn shows_the_answer_bar_the_grid_its_status_and_the_clue_sheet() {
        let html = page(puzzle(), |_| {});
        let order = [
            "build-grid__left",
            "answer-bar",
            "grid-area",
            "xw-grid",
            "status-line",
            "clue-sheet",
        ];
        let mut from = 0;
        for piece in order {
            let at = html[from..]
                .find(&format!(r#"class="{piece}""#))
                .unwrap_or_else(|| panic!("{piece} missing or out of order: {html}"));
            from += at;
        }
        // The grid: nine squares, one a block, 1 across selected at its first square.
        assert_eq!(html.matches("xw-grid__cell").count(), 9, "{html}");
        assert_eq!(html.matches("is-block").count(), 1, "{html}");
        assert_eq!(html.matches("is-current").count(), 1, "{html}");
        assert_eq!(html.matches("is-answer").count(), 2, "{html}");
        assert!(
            html.contains(r#"<span class="xw-grid__number">1</span>C"#),
            "{html}"
        );
        // The answer bar.
        let bar = text_of(&html, "answer-bar");
        assert!(bar.contains("1 across") && bar.contains("CAT"), "{bar}");
        assert!(html.contains(r#"value="3""#), "{html}");
        // The status.
        let status = text_of(&html, "status-line");
        for check in [
            "4 of 4 answers filled",
            "every answer is a word",
            "everything still fillable",
            "no repeats",
            "British checking rules met",
        ] {
            assert!(status.contains(check), "{check}: {status}");
        }
        // Nothing is open over the page.
        assert!(!html.contains("word-popover"), "{html}");
        assert!(!html.contains("template-picker"), "{html}");
    }

    #[test]
    fn the_clue_sheet_never_shows_an_answer() {
        let html = page(puzzle(), |_| {});
        let sheet = text_of(&html, "clue-sheet");
        assert!(
            sheet.contains("Across") && sheet.contains("Down"),
            "{sheet}"
        );
        assert!(sheet.contains("Pet") && sheet.contains("(3)"), "{sheet}");
        assert!(sheet.contains("Digit"), "{sheet}");
        assert_eq!(sheet.matches("no clue").count(), 2, "{sheet}");
        let lower = sheet.to_lowercase();
        for answer in ["cat", "bye", "cab", "toe"] {
            assert!(
                !lower.contains(answer),
                "{answer} is in the clue sheet: {sheet}"
            );
        }
        // The selected answer's row is the marked one.
        let marked = &html[html.find(r#"aria-current="true""#).expect("a marked row")..];
        assert!(
            marked[..marked.find("</button>").unwrap()].contains("Pet"),
            "{html}"
        );
    }

    #[test]
    fn there_is_no_instruction_text() {
        let html = page(puzzle(), |mut state| state.build.words_open.set(true));
        let seen = text_of(&html, "build-grid").to_lowercase();
        assert!(seen.contains("1 across"), "{seen}");
        for banned in ["click", "press", "ctrl", "type", "hint", "jump"] {
            assert!(!seen.contains(banned), "{banned}: {seen}");
        }
        let lower = html.to_lowercase();
        for banned in ["surface", "recipe"] {
            assert!(!lower.contains(banned), "{banned}: {html}");
        }
    }

    #[test]
    fn the_popover_opens_beside_the_selected_answer() {
        let html = page(puzzle(), |mut state| {
            state.build.cursor.set(Cursor {
                cell: 2,
                dir: Dir::Down,
            });
            state.build.words_open.set(true);
            state.build.found.set(Some(Found {
                light: "0,2,d".into(),
                pattern: "toe".into(),
                entries: vec![crate::ds::word_popover::WordEntry {
                    text: "toe".into(),
                    split: None,
                    rare: false,
                }],
                ..Default::default()
            }));
        });
        let popover = text_of(&html, "word-popover");
        assert!(popover.contains("2 down"), "{popover}");
        assert!(popover.contains("toe"), "{popover}");
        for chip in ["Any", "1 word", "2 words", "3+ words"] {
            assert!(popover.contains(chip), "{popover}");
        }
        assert!(html.contains(r#"placeholder="Search TOE""#), "{html}");
        // Beside column 2, level with the top row.
        assert!(html.contains("--at-column: 3; --at-row: 0"), "{html}");
        assert!(html.contains(r#"data-side="right""#), "{html}");
        // It sits in the grid's own area, so it moves with the grid.
        let area = &html[html.find(r#"class="grid-area""#).unwrap()..];
        let status = area.find(r#"class="status-line""#).unwrap();
        assert!(area[..status].contains("word-popover"), "{html}");
    }

    #[test]
    fn words_found_for_other_letters_are_not_shown() {
        let html = page(puzzle(), |mut state| {
            state.build.words_open.set(true);
            state.build.found.set(Some(Found {
                light: "0,0,a".into(),
                pattern: "c?t".into(),
                entries: vec![crate::ds::word_popover::WordEntry {
                    text: "cot".into(),
                    split: None,
                    rare: false,
                }],
                ..Default::default()
            }));
        });
        assert!(html.contains("Finding words…"), "{html}");
        assert!(!html.contains("word-popover__item"), "{html}");
    }

    #[test]
    fn words_found_for_an_older_search_are_not_shown() {
        let html = page(puzzle(), |mut state| {
            state.build.words_open.set(true);
            state.build.query.set("co".into());
            // Found while the box said "c", for the same letters.
            let pattern = {
                let p = state.puzzle.peek();
                let p = p.as_ref().unwrap();
                p.grid.pattern(&p.grid.lights()[0])
            };
            state.build.found.set(Some(Found {
                light: "0,0,a".into(),
                pattern,
                query: "c".into(),
                entries: vec![crate::ds::word_popover::WordEntry {
                    text: "cat".into(),
                    split: None,
                    rare: false,
                }],
                ..Default::default()
            }));
        });
        assert!(!html.contains("word-popover__item"), "{html}");
    }

    #[test]
    fn a_non_word_can_be_added_from_the_status_line() {
        let mut p = puzzle();
        p.grid = Grid::from_rows(&["cat", "a#o", "byq"]);
        let html = page(p, |_| {});
        let status = text_of(&html, "status-line");
        assert!(
            status.contains("BYQ (3 across) is not in the word list"),
            "{status}"
        );
        assert!(status.contains("Add to my words"), "{status}");
        assert_eq!(html.matches(">Add to my words<").count(), 2, "{html}");
    }

    #[test]
    fn the_picker_lists_every_standard_grid() {
        let html = page(puzzle(), |mut state| state.build.picker_open.set(true));
        assert!(html.contains(r#"class="template-picker""#), "{html}");
        // Every size has a pill; the 15×15 grids show first.
        for n in [5, 7, 9, 11, 13, 15] {
            assert!(html.contains(&format!(">{n}×{n}<")), "{n}: {html}");
        }
        let fifteens = TEMPLATES.iter().filter(|t| t.rows.len() == 15).count();
        assert_eq!(
            html.matches("template-picker__thumb").count(),
            fifteens,
            "{html}"
        );
        for t in TEMPLATES.iter().filter(|t| t.rows.len() == 15) {
            assert!(html.contains(&format!(">{}<", t.name)), "{}", t.name);
        }
    }

    #[test]
    fn a_puzzle_with_no_answers_says_so_and_can_still_be_replaced() {
        let mut p = Puzzle::new(2, 2);
        p.grid = Grid::from_rows(&["##", "##"]);
        let html = page(p, |mut state| state.build.picker_open.set(true));
        assert!(html.contains("This puzzle has no answers."), "{html}");
        assert!(!html.contains("xw-grid"), "{html}");
        assert!(html.contains("template-picker"), "{html}");
    }
}
