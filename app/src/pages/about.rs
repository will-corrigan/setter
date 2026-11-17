//! Credits and licences for the bundled data, fonts and components.

use crate::ds::prose::Prose;
use dioxus::prelude::*;

/// The licence notices, compiled in so they ship with every build.
static NOTICE: &str = include_str!("../../../data/NOTICE.md");

#[component]
pub fn About() -> Element {
    rsx! {
        Prose { title: "Credits", text: NOTICE.trim_start_matches("# Credits and licences").trim_start() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    #[test]
    fn credits_cover_every_bundled_source() {
        let html = render(move || rsx! { About {} });
        for name in [
            "UKACD18",
            "Exet",
            "CMUdict",
            "Open English WordNet",
            "Princeton",
            "Moby",
            "Newsreader",
            "Instrument Sans",
            "DioxusLabs",
        ] {
            assert!(html.contains(name), "missing {name}");
        }
    }
}
