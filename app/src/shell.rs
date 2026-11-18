//! The app shell: startup, the top bar, and whichever page the mode shows.

use crate::autosave;
use crate::ds::command_palette::{CommandPalette, PaletteGroup, PaletteItem};
use crate::ds::crossword_grid::GridKey;
use crate::ds::empty_state::EmptyState;
use crate::ds::icon::{Icon, IconName};
use crate::ds::inline_edit::InlineEdit;
use crate::ds::menu_button::{MenuButton, MenuItem};
use crate::ds::notice::Notice;
use crate::ds::segmented::Segmented;
use crate::ds::top_bar::TopBar as Bar;
use crate::export::{self, ExportFormat};
use crate::files::{self, Theme};
use crate::keys;
use crate::pages::about::About;
use crate::pages::build_grid::{self, BuildGrid};
use crate::pages::home::Home;
use crate::pages::write_clues::WriteClues;
use crate::palette::{self, Command, Target};
use crate::state::{self, AppState, Mode, Shortcut};
use dioxus::prelude::*;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

const PUZZLE_FILES: (&str, &[&str]) = ("Setter puzzle", &["xword"]);

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
    let shown = *state.shown_count.peek();
    let made = tokio::task::spawn_blocking(|| {
        let engine = state::engine();
        files::sample_puzzle(&engine.lex, engine.fill(), &engine.quality)
    })
    .await;
    if *state.shown_count.peek() != shown {
        // Something else was opened meanwhile: it stays.
        state.filling.set(false);
        return;
    }
    match made {
        Ok(Ok(puzzle)) => state.show(state.signed(puzzle), None),
        Ok(Err(why)) => state.notice.set(Some(why)),
        Err(_) => state
            .notice
            .set(Some("Couldn't fill a sample grid.".into())),
    }
    state.filling.set(false);
}

/// `path` was opened or saved: it tops the recent list and reopens next time.
fn remember(mut state: AppState, path: &Path) {
    state.settings.write().opened(path);
    files::save_settings(&state.settings.peek());
}

/// Open a puzzle from a file, after saving the one on screen.
async fn open_file(state: AppState, path: PathBuf) {
    let mut notice = state.notice;
    if let Err(e) = save_or_keep(state) {
        notice.set(Some(e));
        return;
    }
    match files::load(&path) {
        Ok(puzzle) => {
            state.show(puzzle, Some(path.clone()));
            remember(state, &path);
        }
        Err(e) => notice.set(Some(e)),
    }
}

async fn open(state: AppState) {
    let mut dialog = rfd::AsyncFileDialog::new().add_filter(PUZZLE_FILES.0, PUZZLE_FILES.1);
    if let Some(dir) = state.path.peek().as_ref().and_then(|p| p.parent()) {
        dialog = dialog.set_directory(dir);
    }
    if let Some(file) = dialog.pick_file().await {
        open_file(state, file.path().to_path_buf()).await;
    }
}

/// Write the puzzle to its file now, if it has changed. A puzzle without a
/// file gets one in the Setter folder, named after its title.
pub fn save_now(state: AppState) -> Result<(), String> {
    let Some(puzzle) = state.puzzle.peek().clone() else {
        return Ok(());
    };
    if !state::unsaved(Some(&puzzle), state.pristine.peek().as_ref()) {
        return Ok(());
    }
    let path = state
        .path
        .peek()
        .clone()
        .unwrap_or_else(|| files::new_file_in(&files::library_dir(), &puzzle.title));
    files::save(&path, &puzzle)?;
    let first = state.path.peek().is_none();
    state.saved_to(path.clone(), puzzle);
    if first {
        remember(state, &path);
    }
    Ok(())
}

/// Save, or if the puzzle's file can't be written, keep a recovery copy
/// that is offered at next start, and say how to get out.
pub fn save_or_keep(state: AppState) -> Result<(), String> {
    save_now(state).map_err(|e| {
        if let Some(p) = state.puzzle.peek().as_ref() {
            let path = state.path.peek().clone();
            let _ = autosave::write_autosave_in(&files::config_dir(), p, path.as_deref());
        }
        format!("{e}. Use Save as to keep it somewhere else.")
    })
}

/// Open a puzzle chosen on Home.
pub fn open_path(state: AppState, path: PathBuf) {
    spawn(async move { open_file(state, path).await });
}

/// Open… from Home.
pub fn open_from_home(state: AppState) {
    spawn(async move { open(state).await });
}

/// Show Home, after saving the puzzle on screen.
pub fn go_home(mut state: AppState) {
    if let Err(e) = save_or_keep(state) {
        state.notice.set(Some(e));
        return;
    }
    state.build.words_open.set(false);
    state.credits.set(false);
    state.home.set(true);
}

/// Save as: choose a new name and place; the puzzle carries on from there.
async fn save_as(state: AppState) {
    let mut notice = state.notice;
    let Some(puzzle) = state.puzzle.peek().clone() else {
        return;
    };
    let known = state.path.peek().clone();
    let mut dialog = rfd::AsyncFileDialog::new()
        .add_filter(PUZZLE_FILES.0, PUZZLE_FILES.1)
        .set_file_name(format!("{}.xword", export::file_stem(&puzzle.title)));
    if let Some(dir) = known.as_ref().and_then(|p| p.parent()) {
        dialog = dialog.set_directory(dir);
    }
    let Some(file) = dialog.save_file().await else {
        return;
    };
    let path = with_xword(file.path());
    // Taken after the dialog, so typing while it was open goes too.
    let Some(puzzle) = state.puzzle.peek().clone() else {
        return;
    };
    match files::save(&path, &puzzle) {
        Ok(()) => {
            state.saved_to(path.clone(), puzzle);
            remember(state, &path);
            // A file Setter named itself in its own folder moves with Save as,
            // so Home doesn't list a stale copy.
            if let Some(old) = known.filter(|old| moves_with_save_as(old, &path)) {
                let _ = std::fs::remove_file(old);
            }
        }
        Err(e) => notice.set(Some(e)),
    }
}

/// Is `old` a file in the Setter folder, left behind by saving as `new`?
fn moves_with_save_as(old: &Path, new: &Path) -> bool {
    let same = |a: &Path, b: &Path| {
        std::fs::canonicalize(a)
            .ok()
            .zip(std::fs::canonicalize(b).ok())
            .is_some_and(|(a, b)| a == b)
    };
    !same(old, new)
        && old
            .parent()
            .is_some_and(|dir| same(dir, &files::library_dir()))
}

/// Show the puzzle's file in the system's file manager.
fn show_in_folder(state: AppState) {
    let mut notice = state.notice;
    let Some(path) = state.path.peek().clone() else {
        return;
    };
    if let Err(e) = files::reveal(&path) {
        notice.set(Some(format!("Couldn't show the folder: {e}")));
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
    if *state.home.peek() || *state.credits.peek() {
        return;
    }
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
                    Some(Shortcut::Save) => {
                        if let Err(e) = save_or_keep(state) {
                            let mut notice = state.notice;
                            notice.set(Some(e));
                        }
                    }
                    Some(Shortcut::SaveAs) => save_as(state).await,
                    Some(Shortcut::New) => new_puzzle(state),
                    Some(Shortcut::Open) => open(state).await,
                    Some(Shortcut::Export) => export(state, ExportFormat::WebPage).await,
                    Some(Shortcut::Undo) => build_grid::undo(state),
                    Some(Shortcut::Redo) => build_grid::redo(state),
                    Some(Shortcut::Words) => build_grid::toggle_words(state),
                    Some(Shortcut::Palette) => {
                        let mut open = state.palette_open;
                        open.toggle();
                    }
                    Some(Shortcut::NextUnclued) if !*state.home.peek() => state.next_unclued(),
                    Some(Shortcut::NextUnclued) => {}
                    None => {}
                }
            }
        });
    });
}

/// Save the puzzle to its own file a second after it last changed. (The old
/// separate autosave file is cleared once everything is saved.)
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
                if let Err(e) = save_or_keep(state) {
                    let mut notice = state.notice;
                    notice.set(Some(e));
                }
            }))),
        }
    });
}

/// Save straight away if anything has changed, e.g. as the window closes.
pub fn write_autosave_now(state: AppState) -> Result<(), String> {
    save_or_keep(state)
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
            if (state.palette_open)() {
                Palette {}
            }
            if let Some(message) = (state.notice)() {
                Notice { message, on_dismiss: move |_| state.notice.set(None) }
            }
            if (state.credits)() {
                About {}
            } else if (state.home)() {
                Home {}
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

/// Start a new, empty puzzle: choose its grid on Build grid. The puzzle on
/// screen is saved first.
fn new_puzzle(mut state: AppState) {
    if let Err(e) = save_or_keep(state) {
        state.notice.set(Some(e));
        return;
    }
    state.set_mode(Mode::BuildGrid);
    state.build.picker_open.set(true);
}

async fn new_sample(mut state: AppState) {
    if let Err(e) = save_or_keep(state) {
        state.notice.set(Some(e));
        return;
    }
    fill_sample(state).await;
}

/// What the File menu does.
#[derive(Clone, PartialEq, Debug)]
pub enum FileAction {
    Home,
    NewPuzzle,
    NewSample,
    Open,
    SaveAs,
    ShowInFolder,
    Export(ExportFormat),
    Recent(PathBuf),
}

/// The File menu: its lines and what each does. `recent` are other puzzles
/// to reopen; "Show in folder" needs the puzzle to have a file.
pub fn file_menu(recent: &[PathBuf], has_file: bool, busy: bool) -> Vec<(MenuItem, FileAction)> {
    let mut out = vec![
        (MenuItem::new("Home"), FileAction::Home),
        (
            MenuItem::new("New puzzle…")
                .separated()
                .keys(keys::label("Ctrl+N"))
                .disabled(busy),
            FileAction::NewPuzzle,
        ),
        (
            MenuItem::new("New filled sample").disabled(busy),
            FileAction::NewSample,
        ),
        (
            MenuItem::new("Open…")
                .keys(keys::label("Ctrl+O"))
                .disabled(busy),
            FileAction::Open,
        ),
        (
            MenuItem::new("Save as…")
                .keys(keys::label("Ctrl+Shift+S"))
                .separated(),
            FileAction::SaveAs,
        ),
        (
            MenuItem::new("Show in folder").disabled(!has_file),
            FileAction::ShowInFolder,
        ),
        (
            MenuItem::new("Export web page…")
                .keys(keys::label("Ctrl+E"))
                .separated(),
            FileAction::Export(ExportFormat::WebPage),
        ),
        (
            MenuItem::new("Export Across Lite (.puz)…"),
            FileAction::Export(ExportFormat::Puz),
        ),
        (
            MenuItem::new("Export ipuz…"),
            FileAction::Export(ExportFormat::Ipuz),
        ),
    ];
    for (i, path) in recent.iter().take(RECENT_SHOWN).enumerate() {
        let name = path
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut item = MenuItem::new(name).disabled(busy);
        if i == 0 {
            item.separated = true;
        }
        out.push((item, FileAction::Recent(path.clone())));
    }
    out
}

/// How many recent puzzles the File menu lists.
const RECENT_SHOWN: usize = 5;

/// What the settings menu does.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SettingsAction {
    Theme(Theme),
    WordLists,
    About,
}

/// The settings menu; a heading has no action.
pub fn settings_menu(theme: Theme) -> Vec<(MenuItem, Option<SettingsAction>)> {
    let choice = |label: &str, t: Theme| {
        (
            MenuItem::new(label).checked(theme == t),
            Some(SettingsAction::Theme(t)),
        )
    };
    vec![
        (MenuItem::heading("Theme"), None),
        choice("Match the system", Theme::System),
        choice("Light", Theme::Light),
        choice("Dark", Theme::Dark),
        (
            MenuItem::new("My word lists").separated(),
            Some(SettingsAction::WordLists),
        ),
        (MenuItem::new("About Setter"), Some(SettingsAction::About)),
    ]
}

fn run_file_action(state: AppState, action: FileAction) {
    match action {
        FileAction::Home => go_home(state),
        FileAction::NewPuzzle => new_puzzle(state),
        FileAction::NewSample => {
            spawn(async move { new_sample(state).await });
        }
        FileAction::Open => {
            spawn(async move { open(state).await });
        }
        FileAction::SaveAs => {
            spawn(async move { save_as(state).await });
        }
        FileAction::ShowInFolder => show_in_folder(state),
        FileAction::Export(format) => {
            spawn(async move { export(state, format).await });
        }
        FileAction::Recent(path) => {
            spawn(async move { open_file(state, path).await });
        }
    }
}

fn run_settings_action(mut state: AppState, action: SettingsAction) {
    match action {
        SettingsAction::Theme(theme) => {
            state.settings.write().theme = theme;
            files::save_settings(&state.settings.peek());
        }
        SettingsAction::WordLists => {
            if let Err(e) = files::reveal(&files::config_dir()) {
                state
                    .notice
                    .set(Some(format!("Couldn't show the folder: {e}")));
            }
        }
        SettingsAction::About => state.credits.toggle(),
    }
}

/// Run what was chosen in the palette.
fn run_target(mut state: AppState, target: Target) {
    match target {
        Target::Answer(light) => {
            state.home.set(false);
            state.credits.set(false);
            state.go_to(&light);
        }
        Target::Puzzle(path) => open_path(state, path),
        Target::Command(c) => match c {
            Command::Home => go_home(state),
            Command::NewPuzzle => new_puzzle(state),
            Command::NewSample => run_file_action(state, FileAction::NewSample),
            Command::Open => run_file_action(state, FileAction::Open),
            Command::SaveAs => run_file_action(state, FileAction::SaveAs),
            Command::ShowInFolder => show_in_folder(state),
            Command::Export(format) => run_file_action(state, FileAction::Export(format)),
            Command::BuildGrid | Command::WriteClues => {
                state.home.set(false);
                state.credits.set(false);
                state.set_mode(if c == Command::BuildGrid {
                    Mode::BuildGrid
                } else {
                    Mode::WriteClues
                });
            }
            Command::Undo => build_grid::undo(state),
            Command::Redo => build_grid::redo(state),
            Command::FillRest => {
                state.home.set(false);
                state.set_mode(Mode::BuildGrid);
                dioxus::core::spawn_forever(build_grid::fill_rest(state));
            }
            Command::NextUnclued => {
                state.home.set(false);
                state.next_unclued();
            }
            Command::PreviousAnswer => step_answer(state, -1),
            Command::NextAnswer => step_answer(state, 1),
            Command::Theme(theme) => run_settings_action(state, SettingsAction::Theme(theme)),
            Command::About => run_settings_action(state, SettingsAction::About),
        },
    }
}

/// How many matches each palette group shows.
const PALETTE_GROUP: usize = 6;

#[component]
fn Palette() -> Element {
    let mut state = use_context::<AppState>();
    let mut query = use_signal(String::new);
    let mut active = use_signal(|| 0usize);
    let entries = {
        let puzzle = state.puzzle.read();
        let history = state.history.read();
        let path = state.path.read().clone();
        let mut all = palette::commands(
            puzzle.is_some(),
            path.is_some(),
            history.can_undo(),
            history.can_redo(),
        );
        if let Some(p) = puzzle.as_ref() {
            all.extend(palette::answer_entries(&crate::answers::answers(p), p));
        }
        let recent: Vec<PathBuf> = state
            .settings
            .read()
            .recent
            .iter()
            .filter(|r| Some(*r) != path.as_ref())
            .cloned()
            .collect();
        all.extend(palette::puzzle_entries(&recent));
        all
    };
    let found = palette::search(&entries, &query(), PALETTE_GROUP);
    let targets: Vec<Target> = found
        .iter()
        .flat_map(|(_, hits)| hits.iter().map(|h| h.entry.target.clone()))
        .collect();
    let total = targets.len();
    let groups: Vec<PaletteGroup> = found
        .into_iter()
        .map(|(g, hits)| PaletteGroup {
            title: g.title().to_string(),
            items: hits
                .into_iter()
                .map(|h| PaletteItem {
                    label: h.entry.label,
                    marks: h.marks,
                    detail: h.entry.detail,
                    keys: h.entry.keys,
                })
                .collect(),
        })
        .collect();
    let mut close = move || {
        state.palette_open.set(false);
    };
    rsx! {
        CommandPalette {
            query: query(),
            groups,
            active: active().min(total.saturating_sub(1)),
            on_query: move |q: String| {
                query.set(q);
                active.set(0);
            },
            on_move: move |d: i32| {
                if total > 0 {
                    let next = (active() as i64 + d as i64).rem_euclid(total as i64) as usize;
                    active.set(next);
                }
            },
            on_run: move |i: usize| {
                close();
                if let Some(t) = targets.get(i) {
                    run_target(state, t.clone());
                }
            },
            on_close: move |_| close(),
        }
    }
}

#[component]
fn TopBar() -> Element {
    let state = use_context::<AppState>();
    let ready = (state.ready)();
    // A fill is running: the puzzle can't be changed or replaced until it ends.
    let working = (state.build.working)();
    let (can_undo, can_redo) = {
        let history = state.history.read();
        (history.can_undo(), history.can_redo())
    };
    let theme = state.settings.read().theme;
    let path = state.path.read().clone();
    let recent: Vec<PathBuf> = state
        .settings
        .read()
        .recent
        .iter()
        .filter(|p| Some(*p) != path.as_ref())
        .cloned()
        .collect();
    let (file_items, file_actions): (Vec<_>, Vec<_>) =
        file_menu(&recent, path.is_some(), !ready || working)
            .into_iter()
            .unzip();
    let (settings_items, settings_actions): (Vec<_>, Vec<_>) =
        settings_menu(theme).into_iter().unzip();
    let (title, setter) = state
        .puzzle
        .read()
        .as_ref()
        .map(|p| (p.title.clone(), p.author.clone()))
        .unwrap_or_default();
    let home = (state.home)();
    rsx! {
        Bar {
            brand: "Setter",
            on_brand: move |_| go_home(state),
            start: rsx! {
                if ready && !home {
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
            },
            middle: rsx! {
                if !home {
                Segmented {
                    label: "Mode",
                    options: Mode::ALL.map(|m| m.label().to_string()).to_vec(),
                    selected: (state.mode)().index(),
                    on_change: move |i| state.set_mode(Mode::from_index(i)),
                }
                }
            },
            end: rsx! {
                if !home {
                button {
                    class: "topbar__icon",
                    r#type: "button",
                    "aria-label": "Undo",
                    title: keys::tip("Undo", "Ctrl+Z"),
                    disabled: !ready || working || !can_undo,
                    onclick: move |_| build_grid::undo(state),
                    Icon { name: IconName::Undo }
                }
                button {
                    class: "topbar__icon",
                    r#type: "button",
                    "aria-label": "Redo",
                    title: keys::tip("Redo", "Ctrl+Shift+Z"),
                    disabled: !ready || working || !can_redo,
                    onclick: move |_| build_grid::redo(state),
                    Icon { name: IconName::Redo }
                }
                MenuButton {
                    label: "File",
                    items: file_items,
                    on_pick: move |i: usize| {
                        if let Some(action) = file_actions.get(i) {
                            run_file_action(state, action.clone());
                        }
                    },
                }
                }
                button {
                    class: "topbar__search",
                    r#type: "button",
                    title: keys::tip("Search", "Ctrl+K"),
                    onclick: move |_| {
                        let mut open = state.palette_open;
                        open.set(true);
                    },
                    Icon { name: IconName::Search }
                    "Search"
                    kbd { "{keys::label(\"Ctrl+K\")}" }
                }
                MenuButton {
                    label: "Settings",
                    icon: rsx! { Icon { name: IconName::Settings } },
                    items: settings_items,
                    on_pick: move |i: usize| {
                        if let Some(Some(action)) = settings_actions.get(i) {
                            run_settings_action(state, *action);
                        }
                    },
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

    /// Pieces in order in the html, or a panic naming the first missing.
    fn in_order(html: &str, order: &[&str]) {
        let mut from = 0;
        for piece in order {
            let at = html[from..]
                .find(piece)
                .unwrap_or_else(|| panic!("{piece} missing or out of order: {html}"));
            from += at;
        }
    }

    #[test]
    fn the_top_bar_is_the_same_on_both_pages() {
        for mode in [Mode::BuildGrid, Mode::WriteClues] {
            let html = shell(move |mut state| {
                state.show(puzzle(), None);
                state.mode.set(mode);
            });
            let end = html.find("</header>").expect("top bar");
            let bar = &html[..end];
            in_order(
                bar,
                &[
                    "topbar__logo",
                    r#"aria-label="Puzzle title""#,
                    r#"aria-label="Setter""#,
                    ">Build grid<",
                    ">Write clues<",
                    r#"aria-label="Undo""#,
                    r#"aria-label="Redo""#,
                    ">File<",
                    r#"aria-label="Settings""#,
                ],
            );
            // Page actions live in the pages, not the bar.
            for gone in [
                "Fill the rest",
                "answer-nav",
                "Theme:",
                "Credits",
                ">Save<",
                "New sample",
            ] {
                assert!(!bar.contains(gone), "{gone} in the bar: {bar}");
            }
        }
    }

    #[test]
    fn home_offers_new_puzzles_and_open_with_a_slim_top_bar() {
        let html = shell(|mut state| {
            state.show(puzzle(), None);
            state.home.set(true);
        });
        assert!(html.contains("size-tiles__tile"), "{html}");
        assert!(html.contains(">5×5<") && html.contains(">15×15<"), "{html}");
        assert!(html.contains(">Open…<"), "{html}");
        let end = html.find("</header>").unwrap();
        let bar = &html[..end];
        assert!(bar.contains(r#"aria-label="Settings""#), "{bar}");
        for gone in [
            ">File<",
            r#"aria-label="Undo""#,
            ">Build grid<",
            "Puzzle title",
        ] {
            assert!(!bar.contains(gone), "{gone}: {bar}");
        }
        // The logo goes home from a puzzle.
        let page = shell(|state| state.show(puzzle(), None));
        assert!(
            page.contains(r#"class="topbar__logo topbar__logo--button""#),
            "{page}"
        );
    }

    #[test]
    fn undo_works_on_write_clues_but_not_behind_home() {
        let said = |home: bool| {
            crate::ds::test_render(move || {
                let mut state = use_app_state(Settings::default);
                use_hook(|| {
                    state.show(puzzle(), None);
                    state.set_mode(Mode::WriteClues);
                    state.set_clue_text("Gold".into());
                    state.home.set(home);
                    build_grid::undo(state);
                });
                let text = state.clue_text();
                rsx! { "[{text}]" }
            })
        };
        assert_eq!(said(false), "[]");
        assert_eq!(said(true), "[Gold]");
    }

    #[test]
    fn save_as_moves_only_files_in_the_setter_folder() {
        let lib = files::library_dir();
        let own = lib.join("Moved.xword");
        std::fs::write(&own, "{}").unwrap();
        let elsewhere = files::scratch_dir(&format!("save-as-{}", std::process::id()));
        let kept = elsewhere.join("Kept.xword");
        std::fs::write(&kept, "{}").unwrap();
        let to = elsewhere.join("New.xword");
        assert!(moves_with_save_as(&own, &to));
        assert!(!moves_with_save_as(&kept, &to), "files elsewhere stay");
        assert!(!moves_with_save_as(&own, &own), "saving over itself");
    }

    #[test]
    fn page_actions_sit_in_their_pages() {
        let grid = shell(|mut state| {
            state.show(puzzle(), None);
            state.mode.set(Mode::BuildGrid);
        });
        let end = grid.find("</header>").unwrap();
        assert!(grid[end..].contains(">Fill the rest<"), "{grid}");
        let clues = shell(|state| state.show(puzzle(), None));
        let end = clues.find("</header>").unwrap();
        assert!(clues[end..].contains("answer-nav"), "{clues}");
        assert!(clues[end..].contains(">1 of 4 clued<"), "{clues}");
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

    /// The opening tag of the button with this aria-label or text.
    fn button(html: &str, name: &str) -> String {
        let at = html
            .find(&format!(r#"aria-label="{name}""#))
            .or_else(|| html.find(&format!(">{name}<")))
            .unwrap_or_else(|| panic!("{name}: {html}"));
        let before = &html[..at];
        let start = before.rfind("<button").unwrap();
        let close = html[start..].find('>').unwrap();
        html[start..start + close].to_string()
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
        // Shortcuts are named in tooltips, for this platform.
        assert!(
            edited.contains(&format!(r#"title="Undo ({})""#, keys::label("Ctrl+Z"))),
            "{edited}"
        );
    }

    #[test]
    fn while_a_fill_runs_it_can_be_stopped_and_nothing_else_changes() {
        let html = shell(|mut state| {
            state.show(puzzle(), None);
            state.mode.set(Mode::BuildGrid);
            state.edit(|p| crate::grid_edit::clear_light(p, "2,0,a"));
            state.build.working.set(true);
        });
        assert!(!html.contains(">Fill the rest<"), "{html}");
        assert!(
            !button(&html, "Stop filling").contains("disabled"),
            "{html}"
        );
        assert!(button(&html, "Undo").contains("disabled"), "{html}");
    }

    #[test]
    fn the_file_menu_in_order_with_recent_puzzles() {
        let recent = [
            PathBuf::from("/p/Prize 12.xword"),
            PathBuf::from("/p/Mini.xword"),
        ];
        let menu = file_menu(&recent, true, false);
        let labels: Vec<&str> = menu.iter().map(|(m, _)| m.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Home",
                "New puzzle…",
                "New filled sample",
                "Open…",
                "Save as…",
                "Show in folder",
                "Export web page…",
                "Export Across Lite (.puz)…",
                "Export ipuz…",
                "Prize 12",
                "Mini",
            ]
        );
        assert_eq!(
            menu[1].0.keys.as_deref(),
            Some(keys::label("Ctrl+N").as_str())
        );
        assert_eq!(menu[9].1, FileAction::Recent(recent[0].clone()));
        assert!(
            menu[9].0.separated,
            "recent puzzles are a group of their own"
        );
        // No file yet: nothing to show in a folder.
        let fresh = file_menu(&[], false, false);
        assert!(
            fresh
                .iter()
                .find(|(m, _)| m.label == "Show in folder")
                .unwrap()
                .0
                .disabled
        );
        // While a fill runs, nothing that replaces the puzzle.
        let busy = file_menu(&recent, true, true);
        for (m, a) in &busy {
            let replaces = matches!(
                a,
                FileAction::NewPuzzle
                    | FileAction::NewSample
                    | FileAction::Open
                    | FileAction::Recent(_)
            );
            assert_eq!(m.disabled, replaces, "{}", m.label);
        }
    }

    #[test]
    fn the_settings_menu_ticks_the_theme_in_use() {
        let menu = settings_menu(Theme::Dark);
        let ticked: Vec<&str> = menu
            .iter()
            .filter(|(m, _)| m.checked == Some(true))
            .map(|(m, _)| m.label.as_str())
            .collect();
        assert_eq!(ticked, ["Dark"]);
        assert!(menu.iter().any(|(_, a)| *a == Some(SettingsAction::About)));
        assert!(menu[0].0.heading && menu[0].1.is_none(), "headed Theme");
    }

    #[test]
    fn changes_are_saved_to_a_new_file_in_the_setter_folder() {
        let said = render(|| {
            let state = use_app_state(Settings::default);
            let result = use_hook(|| {
                let mut p = puzzle();
                p.title = "Autosave test".into();
                state.show(p, None);
                state.edit(|p| crate::grid_edit::place(p, "0,0,a", "cot"));
                save_now(state).map(|_| state.path.peek().clone())
            });
            let path = result.clone().unwrap().unwrap();
            let in_library = path.starts_with(files::library_dir());
            let written = files::load(&path)
                .map(|p| p.grid.pattern(&p.grid.lights()[0]))
                .unwrap_or_default();
            let clean = !(state.dirty)();
            let _ = std::fs::remove_file(&path);
            rsx! { "{in_library} {written} {clean}" }
        });
        assert_eq!(said, "true cot true");
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
