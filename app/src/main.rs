//! Setter: a cryptic crossword setter's desktop app.

mod annotate;
mod answers;
mod autosave;
mod clue_draft;
mod components;
mod ds;
mod export;
mod files;
mod gallery;
mod grid_edit;
mod grid_fill;
mod grid_view;
mod highlight;
mod history;
mod keys;
mod library;
mod pages;
mod palette;
mod shell;
mod state;

use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;

const TOKENS_CSS: Asset = asset!("/assets/tokens.css");
const BRIDGE_CSS: Asset = asset!("/assets/dx-components-theme.css");
const COMPONENTS_CSS: Asset = asset!("/assets/components.css");

/// The window's size when the app starts.
const WINDOW_SIZE: (f64, f64) = (1440.0, 920.0);

/// A window size written as "1000x700".
fn parse_size(text: &str) -> Option<(f64, f64)> {
    let (width, height) = text.split_once(['x', 'X'])?;
    let side = |s: &str| s.trim().parse::<f64>().ok().filter(|n| *n > 0.0);
    Some((side(width)?, side(height)?))
}

/// Dev hook, debug builds only: `SETTER_DEV_SIZE=1000x700` opens the window
/// at that size, to see how a page copes with less room. (The window can't
/// be resized by script under WSLg.)
fn window_size() -> (f64, f64) {
    let asked = std::env::var("SETTER_DEV_SIZE").ok();
    asked
        .filter(|_| cfg!(debug_assertions))
        .and_then(|text| parse_size(&text))
        .unwrap_or(WINDOW_SIZE)
}

/// The app icon for the window and taskbar: icons/icon.svg at 64 px, as raw
/// RGBA. (Installers take the .ico/.icns files; see Dioxus.toml.)
const WINDOW_ICON: &[u8] = include_bytes!("../icons/window-64.rgba");

fn window_icon() -> Option<dioxus::desktop::tao::window::Icon> {
    dioxus::desktop::tao::window::Icon::from_rgba(WINDOW_ICON.to_vec(), 64, 64).ok()
}

/// The app's name in the title bar. Dev hook, debug builds only:
/// `SETTER_DEV_TITLE=<name>` renames the window, so a test window can be told
/// apart from the one in use.
fn app_name() -> String {
    std::env::var("SETTER_DEV_TITLE")
        .ok()
        .filter(|_| cfg!(debug_assertions))
        .unwrap_or_else(|| "Setter".to_string())
}

/// The window's title for a puzzle: its name, then the app's.
pub fn window_title(puzzle_title: &str, app: &str) -> String {
    let t = puzzle_title.trim();
    if t.is_empty() {
        format!("Untitled — {app}")
    } else {
        format!("{t} — {app}")
    }
}

fn main() {
    let (width, height) = window_size();
    let cfg = Config::new()
        .with_custom_head(files::theme_head(files::load_settings().theme))
        .with_menu(None)
        .with_window(
            WindowBuilder::new()
                .with_title(app_name())
                .with_window_icon(window_icon())
                .with_inner_size(LogicalSize::new(width, height)),
        );
    if std::env::args().any(|a| a == "--gallery") {
        dioxus::LaunchBuilder::desktop()
            .with_cfg(cfg)
            .launch(GalleryApp);
    } else {
        dioxus::LaunchBuilder::desktop().with_cfg(cfg).launch(App);
    }
}

/// The typefaces, inlined (see `xw_engine::fonts`). Made once.
fn font_faces() -> &'static str {
    static FACES: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    FACES.get_or_init(xw_engine::fonts::app_faces)
}

/// Fonts, tokens, the registry bridge, our components, then the user's theme.css.
#[component]
fn Styles() -> Element {
    rsx! {
        style { {font_faces()} }
        document::Stylesheet { href: TOKENS_CSS }
        document::Stylesheet { href: BRIDGE_CSS }
        document::Stylesheet { href: COMPONENTS_CSS }
        style { {files::user_css()} }
    }
}

#[component]
fn GalleryApp() -> Element {
    rsx! {
        Styles {}
        gallery::Gallery {}
    }
}

/// The app: its state, what it loads at startup, and the shell around the pages.
#[component]
fn App() -> Element {
    let state = state::use_app_state(|| {
        files::ensure_user_files();
        files::load_settings()
    });
    shell::use_startup(state);
    // The last second of work is written before the window goes.
    dioxus::desktop::use_wry_event_handler(move |event, _| {
        use dioxus::desktop::tao::event::{Event, WindowEvent};
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested | WindowEvent::Destroyed,
            ..
        } = event
        {
            let _ = shell::write_autosave_now(state);
        }
    });
    // The window's title names the puzzle, for the taskbar and switching.
    use_effect(move || {
        let title = state.puzzle.read().as_ref().map(|p| p.title.clone());
        if let Some(title) = title {
            dioxus::desktop::window().set_title(&window_title(&title, &app_name()));
        }
    });
    rsx! {
        Styles {}
        shell::Shell {}
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_window_title_names_the_puzzle() {
        assert_eq!(
            super::window_title("Prize 12", "Setter"),
            "Prize 12 — Setter"
        );
        assert_eq!(super::window_title("  ", "Setter"), "Untitled — Setter");
    }

    #[test]
    fn the_window_icon_is_a_64px_square() {
        assert_eq!(super::WINDOW_ICON.len(), 64 * 64 * 4);
        assert!(super::window_icon().is_some());
    }

    use super::*;

    #[test]
    fn window_sizes_are_written_width_by_height() {
        assert_eq!(parse_size("1000x700"), Some((1000.0, 700.0)));
        assert_eq!(parse_size(" 820 X 900 "), Some((820.0, 900.0)));
        for bad in ["", "1000", "x700", "0x700", "1000x-5", "wide x tall"] {
            assert_eq!(parse_size(bad), None, "{bad}");
        }
    }
}
