//! The app shell: startup, the top bar, and whichever page the mode shows.

use crate::components::button::{Button, ButtonSize, ButtonVariant};
use crate::ds::answer_nav::AnswerNav;
use crate::ds::empty_state::EmptyState;
use crate::ds::note::Note;
use crate::ds::notice::Notice;
use crate::ds::segmented::Segmented;
use crate::ds::top_bar::TopBar as Bar;
use crate::files;
use crate::pages::about::About;
use crate::pages::write_clues::WriteClues;
use crate::state::{self, AppState, Mode, Shortcut, DATA};
use dioxus::prelude::*;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

const PUZZLE_FILES: (&str, &[&str]) = ("Setter puzzle", &["xword"]);

/// How long a button keeps asking before it goes back to normal.
const CONFIRM_FOR: Duration = Duration::from_secs(4);

/// The path with a `.xword` extension, added if it has another or none.
pub fn with_xword(path: &Path) -> PathBuf {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("xword"))
    {
        return path.to_path_buf();
    }
    let mut name = path.as_os_str().to_owned();
    name.push(".xword");
    PathBuf::from(name)
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
            let warnings = tokio::task::spawn_blocking(|| DATA.warnings.clone());
            spawn(async move {
                let mut notice = state.notice;
                if let Some(line) = warnings.await.ok().and_then(|list| warning_line(&list)) {
                    notice.set(Some(line));
                }
            });
            let last = state.settings.peek().last_file.clone();
            match last.and_then(|path| files::load(&path).ok().map(|p| (p, path))) {
                Some((puzzle, path)) => state.show(puzzle, Some(path), true),
                None => fill_sample(state).await,
            }
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
    match tokio::task::spawn_blocking(|| files::sample_puzzle(&DATA.lex)).await {
        Ok(Ok(puzzle)) => state.show(puzzle, None, false),
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
            state.show(puzzle, Some(path.clone()), true);
            remember(state, path);
        }
        Err(e) => state.notice.set(Some(e)),
    }
}

async fn save(mut state: AppState) {
    if state.puzzle.peek().is_none() {
        return;
    }
    let known = state.path.peek().clone();
    let path = match known {
        Some(path) => path,
        None => {
            let picked = rfd::AsyncFileDialog::new()
                .add_filter(PUZZLE_FILES.0, PUZZLE_FILES.1)
                .set_file_name("puzzle.xword")
                .save_file()
                .await;
            match picked {
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
            state.path.set(Some(path.clone()));
            state.dirty.set(false);
            remember(state, path);
        }
        Err(e) => state.notice.set(Some(e)),
    }
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

/// Alt+← / Alt+→ step through the answers and Ctrl+S saves, wherever the
/// focus is.
fn use_shortcuts(state: AppState) {
    use_effect(move || {
        spawn(async move {
            let mut keys = document::eval(&state::shortcut_script());
            while let Ok((key, alt, ctrl)) = keys.recv::<(String, bool, bool)>().await {
                match state::shortcut(&key, alt, ctrl) {
                    Some(Shortcut::PreviousAnswer) => state.step(-1),
                    Some(Shortcut::NextAnswer) => state.step(1),
                    Some(Shortcut::Save) => save(state).await,
                    None => {}
                }
            }
        });
    });
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
    use_theme(state);
    rsx! {
        div { class: "shell",
            TopBar {}
            if let Some(message) = (state.notice)() {
                Notice { message, on_dismiss: move |_| state.notice.set(None) }
            }
            if (state.credits)() {
                About {}
            } else if !(state.ready)() {
                EmptyState { message: "Filling a sample grid…" }
            } else if (state.mode)() == Mode::BuildGrid {
                EmptyState { message: "Grid building comes next. Use New sample to get a filled grid." }
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
        let loses = state::loses_work(*state.dirty.peek(), state.puzzle.peek().as_ref());
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
            }
        });
    };
    let dirty = (state.dirty)();
    let replace_button = move |action: Replace, label: &'static str| {
        if is_asking(action, asking(), dirty) {
            (ButtonVariant::Destructive, "Discard clues?")
        } else {
            (ButtonVariant::Outline, label)
        }
    };

    let ready = (state.ready)();
    let answer = (state.answer)().filter(|_| ready);
    let (clued, total) = (state.clued)();
    let theme = state.settings.read().theme;
    let (open_style, open_label) = replace_button(Replace::Open, "Open");
    let (sample_style, sample_label) = replace_button(Replace::NewSample, "New sample");
    rsx! {
        Bar {
            brand: "Setter",
            start: rsx! {
            Segmented {
                label: "Mode",
                options: Mode::ALL.map(|m| m.label().to_string()).to_vec(),
                selected: (state.mode)().index(),
                on_change: move |i| state.mode.set(Mode::from_index(i)),
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
            if ready && total > 0 {
                Note { text: "{clued} of {total} clued" }
            }
            Button {
                variant: open_style,
                size: ButtonSize::Sm,
                disabled: !ready,
                onclick: move |_| replace(Replace::Open),
                "{open_label}"
            }
            Button {
                variant: if dirty { ButtonVariant::Primary } else { ButtonVariant::Outline },
                size: ButtonSize::Sm,
                disabled: !ready,
                title: "Save (Ctrl+S)",
                onclick: move |_| async move { save(state).await },
                "Save"
            }
            Button {
                variant: sample_style,
                size: ButtonSize::Sm,
                disabled: !ready,
                onclick: move |_| replace(Replace::NewSample),
                "{sample_label}"
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
            });
            use_hook(|| setup(state));
            rsx! { Shell {} }
        })
    }

    #[test]
    fn top_bar_has_everything_in_order() {
        let html = shell(|state| state.show(puzzle(), None, true));
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
    fn build_grid_mode_is_one_line() {
        let html = shell(|mut state| {
            state.show(puzzle(), None, true);
            state.mode.set(Mode::BuildGrid);
        });
        assert!(
            html.contains("Grid building comes next. Use New sample to get a filled grid."),
            "{html}"
        );
        assert!(!html.contains("shell__body"), "{html}");
    }

    #[test]
    fn a_notice_shows_under_the_top_bar() {
        let html = shell(|mut state| {
            state.show(puzzle(), None, true);
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
        let saved = shell(|state| state.show(puzzle(), None, true));
        let unsaved = shell(|state| state.show(puzzle(), None, false));
        let save_style = |html: &str| {
            let before = &html[..html.find(">Save<").unwrap()];
            let tag = &before[before.rfind("<button").unwrap()..];
            tag.contains(r#"data-style="primary""#)
        };
        assert!(!save_style(&saved), "{saved}");
        assert!(save_style(&unsaved), "{unsaved}");
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
