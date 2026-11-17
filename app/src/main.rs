//! Setter: a cryptic crossword setter's desktop app.

mod answers;
mod components;
mod ds;
mod files;
mod gallery;
mod highlight;
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

fn main() {
    let cfg = Config::new()
        .with_custom_head(files::theme_head(files::load_settings().theme))
        .with_menu(None)
        .with_window(
            WindowBuilder::new()
                .with_title("Setter")
                .with_inner_size(LogicalSize::new(1440.0, 920.0)),
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
