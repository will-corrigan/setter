//! Setter: a cryptic crossword setter's desktop app.

mod answers;
mod files;
mod highlight;

use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;

const MAIN_CSS: Asset = asset!("/assets/main.css");
const NEWSREADER: Asset = asset!("/assets/fonts/newsreader-latin-opsz-normal.woff2");
const NEWSREADER_ITALIC: Asset = asset!("/assets/fonts/newsreader-latin-opsz-italic.woff2");
const INSTRUMENT_SANS: Asset = asset!("/assets/fonts/instrument-sans-latin-wght-normal.woff2");

fn main() {
    let cfg = Config::new().with_menu(None).with_window(
        WindowBuilder::new()
            .with_title("Setter")
            .with_inner_size(LogicalSize::new(1440.0, 920.0)),
    );
    dioxus::LaunchBuilder::desktop().with_cfg(cfg).launch(App);
}

fn font_faces() -> String {
    format!(
        "@font-face{{font-family:'Newsreader';font-style:normal;font-weight:200 800;src:url('{NEWSREADER}') format('woff2')}}\
         @font-face{{font-family:'Newsreader';font-style:italic;font-weight:200 800;src:url('{NEWSREADER_ITALIC}') format('woff2')}}\
         @font-face{{font-family:'Instrument Sans';font-style:normal;font-weight:400 700;src:url('{INSTRUMENT_SANS}') format('woff2')}}"
    )
}

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: MAIN_CSS }
        style { {font_faces()} }
        div { class: "app",
            div { class: "top",
                span { class: "logo", "Setter" }
                span { class: "mode",
                    span { "Build grid" }
                    span { class: "on", "Write clues" }
                }
                span { class: "right",
                    button { class: "theme", "Theme" }
                }
            }
        }
    }
}
