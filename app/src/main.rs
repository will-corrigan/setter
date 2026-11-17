//! Setter: a cryptic crossword setter's desktop app.

mod answers;
mod components;
mod ds;
mod files;
mod gallery;
mod grid_edit;
mod grid_fill;
mod grid_view;
mod highlight;
mod history;
mod pages;
mod shell;
mod state;

use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;

const TOKENS_CSS: Asset = asset!("/assets/tokens.css");
const BRIDGE_CSS: Asset = asset!("/assets/dx-components-theme.css");
const COMPONENTS_CSS: Asset = asset!("/assets/components.css");
const NEWSREADER: Asset = asset!("/assets/fonts/newsreader-latin-opsz-normal.woff2");
const NEWSREADER_ITALIC: Asset = asset!("/assets/fonts/newsreader-latin-opsz-italic.woff2");
const INSTRUMENT_SANS: Asset = asset!("/assets/fonts/instrument-sans-latin-wght-normal.woff2");

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

fn main() {
    let (width, height) = window_size();
    let cfg = Config::new()
        .with_custom_head(files::theme_head(files::load_settings().theme))
        .with_menu(None)
        .with_window(
            WindowBuilder::new()
                .with_title("Setter")
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

fn font_faces() -> String {
    format!(
        "@font-face{{font-family:'Newsreader';font-style:normal;font-weight:200 800;src:url('{NEWSREADER}') format('woff2')}}\
         @font-face{{font-family:'Newsreader';font-style:italic;font-weight:200 800;src:url('{NEWSREADER_ITALIC}') format('woff2')}}\
         @font-face{{font-family:'Instrument Sans';font-style:normal;font-weight:400 700;src:url('{INSTRUMENT_SANS}') format('woff2')}}"
    )
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
    rsx! {
        Styles {}
        shell::Shell {}
    }
}

#[cfg(test)]
mod tests {
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
