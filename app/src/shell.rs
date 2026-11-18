//! The app shell: startup, the top bar, and whichever page the mode shows.

use crate::autosave;
use crate::components::button::{Button, ButtonSize, ButtonVariant};
use crate::ds::answer_nav::AnswerNav;
use crate::ds::crossword_grid::GridKey;
use crate::ds::empty_state::EmptyState;
use crate::ds::inline_edit::InlineEdit;
use crate::ds::menu_button::MenuButton;
use crate::ds::note::Note;
use crate::ds::notice::Notice;
use crate::ds::segmented::Segmented;
use crate::ds::top_bar::TopBar as Bar;
use crate::export::{self, ExportFormat};
use crate::files;
use crate::pages::about::About;
use crate::pages::build_grid::{self, BuildGrid};
use crate::pages::write_clues::WriteClues;
use crate::state::{self, AppState, Mode, Shortcut};
use dioxus::core::spawn_forever;
use dioxus::prelude::*;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

const PUZZLE_FILES: (&str, &[&str]) = ("Setter puzzle", &["xword"]);

/// How long a button keeps asking before it goes back to normal.
const CONFIRM_FOR: Duration = Duration::from_secs(4);

/// How long the puzzle has to sit still before it is autosaved.
const AUTOSAVE_AFTER: Duration = Duration::from_secs(1);

/// The path with extension `ext`, added if it has another or none.
pub fn with_extension(path: &Path, ext: &str) -> PathBuf {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
    {
        return path.to_path_buf();
    }
    let mut name = path.as_os_str().to_owned();
    name.push(".");
    name.push(ext);
    PathBuf::from(name)
}

/// The path with a `.xword` extension, added if it has another or none.
pub fn with_xword(path: &Path) -> PathBuf {
    with_extension(path, "xword")
}

/// One line for the notice: the first line of each warning.
pub fn warning_line(warnings: &[String]) -> Option<String> {
    let lines: Vec<&str> = warnings
        .iter()
        .filter_map(|w| w.lines().next())
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    (!lines.is_empty()).then(|| lines.join(" · "))
}

// ---------- startup ----------

/// Load the engine data off the UI thread, then show the last puzzle, or a
/// fresh sample if there isn't one.
pub fn use_startup(state: AppState) {
    use_hook(|| {
        spawn(async move {
            let warnings = tokio::task::spawn_blocking(|| state::engine().warnings.clone());
            spawn(async move {
                let mut notice = state.notice;
                if let Some(line) = warnings.await.ok().and_then(|list| warning_line(&list)) {
                    notice.set(Some(line));
                }
            });
            let last = state.settings.peek().last_file.clone();
            if let Some((work, from)) = autosave::recovery() {
                let mut notice = state.notice;
                let line = match &from {
                    Some(path) => format!("Recovered unsaved changes to {}.", file_name(path)),
                    None => "Recovered unsaved work.".to_string(),
                };
                state.recover(work, from);
                notice.set(Some(line));
            } else {
                match last.and_then(|path| files::load(&path).ok().map(|p| (p, path))) {
                    Some((puzzle, path)) => state.show(puzzle, Some(path)),
                    None => fill_sample(state).await,
                }
            }
            // Have the grid index ready before Build grid first asks for it.
            tokio::task::spawn_blocking(|| {
                state::engine().fill();
            });
            run_dev_script();
        });
    });
}

/// Dev hook, debug builds only: `SETTER_DEV_SCRIPT=<file.js>` runs that
/// script in the window once the first puzzle is on screen, and prints
/// `dev script finished` to stderr when it returns. Synthetic input can't be
/// sent to WSLg windows, so screenshots of a worked clue are driven this way
/// (clicks and typing as DOM events). See `scripts/dev/`.
fn run_dev_script() {
    #[cfg(debug_assertions)]
    if let Some(js) =
        std::env::var_os("SETTER_DEV_SCRIPT").and_then(|path| std::fs::read_to_string(path).ok())
    {
        spawn(async move {
            let result = document::eval(&js).await;
            eprintln!("dev script finished: {result:?}");
        });
    }
}

// ---------- file actions ----------

async fn fill_sample(mut state: AppState) {
    state.filling.set(true);
    match tokio::task::spawn_blocking(|| {
        let engine = state::engine();
        files::sample_puzzle(&engine.lex, engine.fill(), &engine.quality)
    })
    .await
    {
        Ok(Ok(puzzle)) => state.show(state.signed(puzzle), None),
        Ok(Err(why)) => state.notice.set(Some(why)),
        Err(_) => state
            .notice
            .set(Some("Couldn't fill a sample grid.".into())),
    }
    state.filling.set(false);
}

fn remember(mut state: AppState, path: PathBuf) {
    state.settings.write().last_file = Some(path);
    files::save_settings(&state.settings.peek());
}

async fn open(mut state: AppState) {
    let picked = rfd::AsyncFileDialog::new()
        .add_filter(PUZZLE_FILES.0, PUZZLE_FILES.1)
        .pick_file()
        .await;
    let Some(file) = picked else {
        return;
    };
    let path = file.path().to_path_buf();
    match files::load(&path) {
        Ok(puzzle) => {
            state.show(puzzle, Some(path.clone()));
            remember(state, path);
        }
        Err(e) => state.notice.set(Some(e)),
    }
}

/// Save to the puzzle's file, or ask where if it has none or `ask` is set
/// (Save as).
async fn save(mut state: AppState, ask: bool) {
    let Some(title) = state.puzzle.peek().as_ref().map(|p| p.title.clone()) else {
        return;
    };
    let known = state.path.peek().clone();
    let path = match known {
        Some(path) if !ask => path,
        _ => {
            let mut dialog = rfd::AsyncFileDialog::new()
                .add_filter(PUZZLE_FILES.0, PUZZLE_FILES.1)
                .set_file_name(format!("{}.xword", export::file_stem(&title)));
            if let Some(dir) = known.as_ref().and_then(|p| p.parent()) {
                dialog = dialog.set_directory(dir);
            }
            match dialog.save_file().await {
                Some(file) => with_xword(file.path()),
                None => return,
            }
        }
    };
    // Taken after the dialog, so typing while it was open is saved too.
    let Some(puzzle) = state.puzzle.peek().clone() else {
        return;
    };
    match files::save(&path, &puzzle) {
        Ok(()) => {
            // Typing while it was written still counts as unsaved.
            state.saved_to(path.clone(), puzzle);
            remember(state, path);
        }
        Err(e) => state.notice.set(Some(e)),
    }
}

/// Export the puzzle in `format`: pick where, then write it.
async fn export(mut state: AppState, format: ExportFormat) {
    let Some(puzzle) = state.puzzle.peek().clone() else {
        return;
    };
    // Say why first, before asking where to put a file that can't be made.
    let bytes = match export::render(&puzzle, format) {
        Ok(bytes) => bytes,
        Err(why) => {
            state.notice.set(Some(why));
            return;
        }
    };
    let mut dialog = rfd::AsyncFileDialog::new()
        .add_filter(format.label(), &[format.extension()])
        .set_file_name(format.file_name(&puzzle.title));
    if let Some(dir) = state.path.peek().as_ref().and_then(|p| p.parent()) {
        dialog = dialog.set_directory(dir);
    }
    let Some(file) = dialog.save_file().await else {
        return;
    };
    let path = with_extension(file.path(), format.extension());
    let line = match std::fs::write(&path, bytes) {
        Ok(()) => format!("Exported {}", file_name(&path)),
        Err(e) => format!("Couldn't export {}: {e}", file_name(&path)),
    };
    state.notice.set(Some(line));
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

// ---------- hooks ----------

/// Build the word bank for the answer on screen, off the UI thread. Each
/// pattern is built once and kept.
fn use_bank_loader(state: AppState) {
    let pattern = use_memo(move || (state.answer)().map(|a| a.pattern));
    let _bank = use_resource(move || {
        let pattern = pattern();
        async move {
            let Some(pattern) = pattern else {
                return;
            };
            if state.banks.peek().contains_key(&pattern) {
                return;
            }
            let key = pattern.clone();
            let built = tokio::task::spawn_blocking(move || state::build_bank(&key)).await;
            if let Ok(devices) = built {
                let mut banks = state.banks;
                banks.write().insert(pattern, Rc::new(devices));
            }
        }
    });
}

/// Step to the answer before or after, on whichever page is showing.
fn step_answer(state: AppState, delta: i32) {
    if *state.mode.peek() == Mode::BuildGrid {
        let key = if delta < 0 {
            GridKey::PreviousAnswer
        } else {
            GridKey::NextAnswer
        };
        build_grid::key(state, key);
    } else {
        state.step(delta);
    }
}

/// Alt+← / Alt+→ step through the answers and Ctrl+S saves, wherever the
/// focus is. On Build grid, Ctrl+Z and Ctrl+Shift+Z undo and redo, and
/// Ctrl+Space shows the words for the selected answer.
fn use_shortcuts(state: AppState) {
    use_effect(move || {
        spawn(async move {
            let mut keys = document::eval(&state::shortcut_script());
            while let Ok((key, alt, ctrl, shift)) = keys.recv::<(String, bool, bool, bool)>().await
            {
                match state::shortcut(&key, alt, ctrl, shift) {
                    Some(Shortcut::PreviousAnswer) => step_answer(state, -1),
                    Some(Shortcut::NextAnswer) => step_answer(state, 1),
                    Some(Shortcut::Save) => save(state, false).await,
                    Some(Shortcut::SaveAs) => save(state, true).await,
                    Some(Shortcut::Undo) => build_grid::undo(state),
                    Some(Shortcut::Redo) => build_grid::redo(state),
                    Some(Shortcut::Words) => build_grid::toggle_words(state),
                    None => {}
                }
            }
        });
    });
}

/// Keep unsaved work in the autosave file, a second after it last changed;
/// once nothing is unsaved, the autosave goes.
fn use_autosave(state: AppState) {
    let mut pending = use_signal(|| None::<dioxus::core::Task>);
    use_effect(move || {
        let step = autosave::step(state.puzzle.read().as_ref(), state.pristine.read().as_ref());
        if let Some(task) = pending.take() {
            task.cancel();
        }
        match step {
            autosave::Step::Nothing => {}
            autosave::Step::Remove => autosave::remove_autosave(),
            autosave::Step::Write => pending.set(Some(spawn(async move {
                tokio::time::sleep(AUTOSAVE_AFTER).await;
                if let Err(e) = write_autosave_now(state) {
                    let mut notice = state.notice;
                    notice.set(Some(format!("Couldn't autosave: {e}")));
                }
            }))),
        }
    });
}

/// Write the autosave straight away if there is unsaved work, e.g. as the
/// window closes.
pub fn write_autosave_now(state: AppState) -> Result<(), String> {
    let puzzle = state.puzzle.peek();
    let pristine = state.pristine.peek();
    if autosave::step(puzzle.as_ref(), pristine.as_ref()) != autosave::Step::Write {
        return Ok(());
    }
    let Some(p) = puzzle.as_ref() else {
        return Ok(());
    };
    autosave::write_autosave(p, state.path.peek().as_deref())
}

fn use_theme(state: AppState) {
    use_effect(move || {
        document::eval(state::theme_script(state.settings.read().theme));
    });
}

// ---------- components ----------

#[component]
pub fn Shell() -> Element {
    let mut state = use_context::<AppState>();
    use_bank_loader(state);
    use_shortcuts(state);
    use_autosave(state);
    use_theme(state);
    rsx! {
        // A click anywhere but in the words popover closes it.
        div { class: "shell", onclick: move |_| build_grid::close_words(state),
            TopBar {}
            if let Some(message) = (state.notice)() {
                Notice { message, on_dismiss: move |_| state.notice.set(None) }
            }
            if (state.credits)() {
                About {}
            } else if !(state.ready)() {
                EmptyState { message: "Filling a sample grid…" }
            } else if (state.mode)() == Mode::BuildGrid {
                BuildGrid {}
            } else {
                WriteClues {}
            }
        }
    }
}

/// Actions that replace the puzzle on screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Replace {
    Open,
    NewSample,
    /// Choose a grid for a new, empty puzzle.
    NewPuzzle,
}

/// Is this button asking before it throws unsaved clues away?
fn is_asking(action: Replace, asking: Option<Replace>, dirty: bool) -> bool {
    dirty && asking == Some(action)
}

#[component]
fn TopBar() -> Element {
    let mut state = use_context::<AppState>();
    // The button that is asking before it throws unsaved clues away.
    let mut asking = use_signal(|| None::<Replace>);
    let mut asking_timer = use_signal(|| None::<dioxus::core::Task>);

    let mut replace = move |action: Replace| {
        if let Some(timer) = asking_timer.take() {
            timer.cancel();
        }
        let loses = *state.dirty.peek();
        if loses && *asking.peek() != Some(action) {
            asking.set(Some(action));
            asking_timer.set(Some(spawn(async move {
                tokio::time::sleep(CONFIRM_FOR).await;
                asking.set(None);
            })));
            return;
        }
        asking.set(None);
        spawn(async move {
            match action {
                Replace::Open => open(state).await,
                Replace::NewSample => fill_sample(state).await,
                Replace::NewPuzzle => state.build.picker_open.set(true),
            }
        });
    };
    let dirty = (state.dirty)();
    let replace_button = move |action: Replace, label: &'static str| {
        if is_asking(action, asking(), dirty) {
            (ButtonVariant::Destructive, "Discard changes?")
        } else {
            (ButtonVariant::Outline, label)
        }
    };

    let ready = (state.ready)();
    let building = (state.mode)() == Mode::BuildGrid;
    // A fill is running: the puzzle can't be changed or replaced until it ends.
    let working = (state.build.working)();
    let answer = (state.answer)().filter(|_| ready && !building);
    let (clued, total) = (state.clued)();
    let (filled, _) = (state.filled)();
    let (can_undo, can_redo) = {
        let history = state.history.read();
        (history.can_undo(), history.can_redo())
    };
    let theme = state.settings.read().theme;
    let (open_style, open_label) = replace_button(Replace::Open, "Open");
    let (sample_style, sample_label) = replace_button(Replace::NewSample, "New sample");
    let (new_style, new_label) = replace_button(Replace::NewPuzzle, "New puzzle");
    let (title, setter) = state
        .puzzle
        .read()
        .as_ref()
        .map(|p| (p.title.clone(), p.author.clone()))
        .unwrap_or_default();
    rsx! {
        Bar {
            brand: "Setter",
            start: rsx! {
            Segmented {
                label: "Mode",
                options: Mode::ALL.map(|m| m.label().to_string()).to_vec(),
                selected: (state.mode)().index(),
                on_change: move |i| state.set_mode(Mode::from_index(i)),
            }
            if building && ready {
                div { class: "topbar__details",
                    InlineEdit {
                        value: title,
                        placeholder: "Untitled",
                        label: "Puzzle title",
                        on_change: move |t| state.set_title(t),
                    }
                    InlineEdit {
                        value: setter,
                        placeholder: "Setter",
                        label: "Setter",
                        on_change: move |name| state.set_setter(name),
                    }
                }
            }
            if building {
                Button {
                    variant: new_style,
                    size: ButtonSize::Sm,
                    disabled: !ready || working,
                    onclick: move |_| replace(Replace::NewPuzzle),
                    "{new_label}"
                }
            }
            if let Some(answer) = answer {
                AnswerNav {
                    word: state::answer_letters(&answer.pattern),
                    label: "{answer.label} · {answer.enumeration}",
                    on_step: move |delta| state.step(delta),
                }
            }
            },
            end: rsx! {
            if building {
                if ready && total > 0 {
                    Note { text: "{filled} of {total} filled" }
                }
                if working {
                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        onclick: move |_| build_grid::stop_fill(state),
                        "Stop filling"
                    }
                } else {
                    Button {
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
                        disabled: !ready || filled == total,
                        onclick: move |_| {
                            spawn_forever(build_grid::fill_rest(state));
                        },
                        "Fill the rest"
                    }
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    disabled: !ready || working || !can_undo,
                    title: "Undo (Ctrl+Z)",
                    onclick: move |_| build_grid::undo(state),
                    "Undo"
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    disabled: !ready || working || !can_redo,
                    title: "Redo (Ctrl+Shift+Z)",
                    onclick: move |_| build_grid::redo(state),
                    "Redo"
                }
            } else if ready && total > 0 {
                Note { text: "{clued} of {total} clued" }
            }
            Button {
                variant: open_style,
                size: ButtonSize::Sm,
                disabled: !ready || working,
                onclick: move |_| replace(Replace::Open),
                "{open_label}"
            }
            Button {
                variant: if dirty { ButtonVariant::Primary } else { ButtonVariant::Outline },
                size: ButtonSize::Sm,
                disabled: !ready,
                title: "Save (Ctrl+S) · Save as (Ctrl+Shift+S)",
                onclick: move |_| async move { save(state, false).await },
                "Save"
            }
            if ready {
                MenuButton {
                    label: "Export",
                    items: ExportFormat::ALL.map(|f| f.label().to_string()).to_vec(),
                    on_pick: move |i: usize| {
                        let format = ExportFormat::ALL[i];
                        spawn(async move { export(state, format).await });
                    },
                }
            }
            if !building {
                Button {
                    variant: sample_style,
                    size: ButtonSize::Sm,
                    disabled: !ready || working,
                    onclick: move |_| replace(Replace::NewSample),
                    "{sample_label}"
                }
            }
            Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Sm,
                onclick: move |_| {
                    let next = state::next_theme(state.settings.peek().theme);
                    state.settings.write().theme = next;
                    files::save_settings(&state.settings.peek());
                },
                "{state::theme_label(theme)}"
            }
            Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Sm,
                onclick: move |_| state.credits.toggle(),
                if (state.credits)() { "Close credits" } else { "Credits" }
            }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;
    use crate::files::{Settings, Theme};
    use crate::state::{use_app_state, Mode};
    use xw_engine::grid::Grid;
    use xw_engine::puzzle::Puzzle;

    fn puzzle() -> Puzzle {
        let mut p = Puzzle::new(3, 3);
        p.grid = Grid::from_rows(&["c?t", "a#o", "bye"]);
        p.clues.entry("2,0,a".into()).or_default().text = "Farewell".into();
        p
    }

    fn shell(setup: impl FnOnce(AppState) + 'static) -> String {
        render(move || {
            let state = use_app_state(|| Settings {
                theme: Theme::Dark,
                last_file: None,
                ..Default::default()
            });
            use_hook(|| setup(state));
            rsx! { Shell {} }
        })
    }

    #[test]
    fn top_bar_has_everything_in_order() {
        let html = shell(|state| state.show(puzzle(), None));
        let order = [
            "topbar__logo",
            ">Build grid<",
            ">Write clues<",
            "answer-nav",
            ">C·T<",
            ">1a · 3<",
            "topbar__spacer",
            ">1 of 4 clued<",
            ">Open<",
            ">Save<",
            ">New sample<",
            ">Theme: dark<",
        ];
        let mut from = 0;
        for piece in order {
            let at = html[from..]
                .find(piece)
                .unwrap_or_else(|| panic!("{piece} missing or out of order: {html}"));
            from += at;
        }
        assert!(html.contains(">Setter<"), "{html}");
        // Write clues is the page on show.
        assert!(
            html.contains("shell__body") && html.contains("side-nav"),
            "{html}"
        );
    }

    #[test]
    fn waiting_for_a_sample_says_so() {
        let html = shell(|_| {});
        assert!(html.contains("Filling a sample grid…"), "{html}");
        assert!(html.contains("topbar__logo"), "{html}");
        assert!(!html.contains("answer-nav"), "{html}");
        assert!(!html.contains("shell__body"), "{html}");
    }

    #[test]
    fn build_grid_mode_shows_the_grid_page() {
        let html = shell(|mut state| {
            state.show(puzzle(), None);
            state.mode.set(Mode::BuildGrid);
        });
        for piece in ["build-grid", "xw-grid", "status-line", "clue-sheet"] {
            assert!(
                html.contains(&format!(r#"class="{piece}""#)),
                "{piece}: {html}"
            );
        }
        // Not the Write clues page.
        assert!(!html.contains("shell__body"), "{html}");
        assert!(!html.contains("side-nav"), "{html}");
    }

    #[test]
    fn build_grid_has_its_own_top_bar() {
        let html = shell(|mut state| {
            state.show(puzzle(), None);
            state.mode.set(Mode::BuildGrid);
        });
        let order = [
            "topbar__logo",
            ">Build grid<",
            ">Write clues<",
            ">New puzzle<",
            "topbar__spacer",
            ">3 of 4 filled<",
            ">Fill the rest<",
            ">Undo<",
            ">Redo<",
            ">Open<",
            ">Save<",
            ">Theme: dark<",
        ];
        let mut from = 0;
        for piece in order {
            let at = html[from..]
                .find(piece)
                .unwrap_or_else(|| panic!("{piece} missing or out of order: {html}"));
            from += at;
        }
        // The answer navigator and New sample belong to Write clues.
        assert!(!html.contains("answer-nav"), "{html}");
        assert!(!html.contains(">New sample<"), "{html}");
        assert!(!html.contains("clued<"), "{html}");
        // Shortcuts are named in tooltips only.
        assert!(html.contains(r#"title="Undo (Ctrl+Z)""#), "{html}");
        assert!(html.contains(r#"title="Redo (Ctrl+Shift+Z)""#), "{html}");
        assert!(!html.contains(">Undo (Ctrl+Z)<"), "{html}");
    }

    /// The opening tag of the top-bar button with this label.
    fn button(html: &str, label: &str) -> String {
        let before = &html[..html
            .find(&format!(">{label}<"))
            .unwrap_or_else(|| panic!("{label}: {html}"))];
        before[before.rfind("<button").unwrap()..].to_string()
    }

    #[test]
    fn undo_and_redo_are_only_live_when_there_is_something_to_do() {
        let fresh = shell(|mut state| {
            state.show(puzzle(), None);
            state.mode.set(Mode::BuildGrid);
        });
        assert!(button(&fresh, "Undo").contains("disabled"), "{fresh}");
        assert!(button(&fresh, "Redo").contains("disabled"), "{fresh}");
        assert!(
            !button(&fresh, "Fill the rest").contains("disabled"),
            "{fresh}"
        );
        let edited = shell(|mut state| {
            state.show(puzzle(), None);
            state.mode.set(Mode::BuildGrid);
            state.edit(|p| crate::grid_edit::place(p, "0,0,a", "cot"));
        });
        assert!(!button(&edited, "Undo").contains("disabled"), "{edited}");
        assert!(button(&edited, "Redo").contains("disabled"), "{edited}");
        // Every square is full now: nothing left to fill.
        assert!(
            button(&edited, "Fill the rest").contains("disabled"),
            "{edited}"
        );
    }

    #[test]
    fn while_a_fill_runs_the_grid_cannot_be_changed_or_replaced() {
        let html = shell(|mut state| {
            state.show(puzzle(), None);
            state.mode.set(Mode::BuildGrid);
            state.edit(|p| crate::grid_edit::clear_light(p, "2,0,a"));
            state.build.working.set(true);
        });
        assert!(!html.contains(">Fill the rest<"), "{html}");
        for label in ["Undo", "New puzzle", "Open"] {
            assert!(button(&html, label).contains("disabled"), "{label}: {html}");
        }
        for label in ["Save", "Stop filling"] {
            assert!(
                !button(&html, label).contains("disabled"),
                "{label}: {html}"
            );
        }
    }

    #[test]
    fn export_sits_after_save_on_both_pages() {
        for mode in [Mode::BuildGrid, Mode::WriteClues] {
            let html = shell(move |mut state| {
                state.show(puzzle(), None);
                state.mode.set(mode);
            });
            let save = html.find(">Save<").expect("save");
            let export = html
                .find(">Export<")
                .unwrap_or_else(|| panic!("{mode:?}: {html}"));
            assert!(save < export, "{html}");
            assert!(html.contains(r#"aria-haspopup="menu""#), "{html}");
        }
        // Nothing to export yet.
        assert!(!shell(|_| {}).contains(">Export<"));
    }

    #[test]
    fn build_grid_shows_the_title_and_setter_to_edit() {
        let html = shell(|mut state| {
            let mut p = puzzle();
            p.title = "Prize 12".into();
            state.show(p, None);
            state.mode.set(Mode::BuildGrid);
        });
        let details = html.find("topbar__details").expect("details");
        let title = html.find(">Prize 12<").expect("title");
        let setter = html.find(r#"aria-label="Setter""#).expect("setter");
        let new = html.find(">New puzzle<").unwrap();
        assert!(details < title && title < setter && setter < new, "{html}");
        // No setter yet: the placeholder says what goes there.
        assert!(html.contains(">Setter<"), "{html}");
        let clues = shell(|state| state.show(puzzle(), None));
        assert!(!clues.contains("topbar__details"), "{clues}");
    }

    #[test]
    fn a_notice_shows_under_the_top_bar() {
        let html = shell(|mut state| {
            state.show(puzzle(), None);
            state.notice.set(Some("indicators.toml: bad line".into()));
        });
        let bar = html.find("topbar").unwrap();
        let notice = html.find(r#"class="notice""#).expect("notice");
        let body = html.find("shell__body").unwrap();
        assert!(bar < notice && notice < body, "{html}");
        assert!(html.contains("indicators.toml: bad line"), "{html}");
    }

    #[test]
    fn unsaved_puzzles_make_save_the_main_button() {
        let saved = shell(|state| state.show(puzzle(), None));
        let unsaved = shell(|state| {
            state.show(puzzle(), None);
            // Letters alone are work to keep, clues or not.
            state.edit(|p| crate::grid_edit::place(p, "0,0,a", "cot"));
        });
        let recovered = shell(|state| state.recover(puzzle(), None));
        let save_style = |html: &str| {
            let before = &html[..html.find(">Save<").unwrap()];
            let tag = &before[before.rfind("<button").unwrap()..];
            tag.contains(r#"data-style="primary""#)
        };
        assert!(!save_style(&saved), "{saved}");
        assert!(save_style(&unsaved), "{unsaved}");
        assert!(save_style(&recovered), "{recovered}");
    }

    #[test]
    fn warnings_become_one_line() {
        let toml = "abbreviations.toml: TOML parse error at line 1, column 6\n  |\n1 | this is = not [toml\n";
        assert_eq!(
            warning_line(&[toml.to_string(), "indicators.toml: bad list".to_string()]).as_deref(),
            Some("abbreviations.toml: TOML parse error at line 1, column 6 · indicators.toml: bad list")
        );
        assert_eq!(warning_line(&[]), None);
        assert_eq!(warning_line(&["\n".to_string()]), None);
    }

    #[test]
    fn a_button_asks_only_while_there_are_unsaved_clues() {
        let asking = Some(Replace::NewSample);
        assert!(is_asking(Replace::NewSample, asking, true));
        assert!(!is_asking(Replace::Open, asking, true));
        assert!(!is_asking(Replace::NewSample, None, true));
        // Saved in the meantime: nothing to ask about.
        assert!(!is_asking(Replace::NewSample, asking, false));
    }

    #[test]
    fn exported_files_get_their_extension() {
        assert_eq!(
            with_extension(Path::new("/tmp/Prize 12"), "html"),
            PathBuf::from("/tmp/Prize 12.html")
        );
        assert_eq!(
            with_extension(Path::new("/tmp/a.PUZ"), "puz"),
            PathBuf::from("/tmp/a.PUZ")
        );
        assert_eq!(
            with_extension(Path::new("/tmp/a.html"), "ipuz"),
            PathBuf::from("/tmp/a.html.ipuz")
        );
    }

    #[test]
    fn saved_files_get_the_xword_extension() {
        assert_eq!(
            with_xword(Path::new("/tmp/a")),
            PathBuf::from("/tmp/a.xword")
        );
        assert_eq!(
            with_xword(Path::new("/tmp/a.xword")),
            PathBuf::from("/tmp/a.xword")
        );
        assert_eq!(
            with_xword(Path::new("/tmp/a.XWORD")),
            PathBuf::from("/tmp/a.XWORD")
        );
        assert_eq!(
            with_xword(Path::new("/tmp/a.txt")),
            PathBuf::from("/tmp/a.txt.xword")
        );
    }
}
