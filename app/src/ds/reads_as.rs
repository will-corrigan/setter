//! The quiet line under the clue: how it reads, plus passing checks only.

use crate::highlight::Checks;
use dioxus::prelude::*;

#[component]
pub fn ReadsAs(text: String, checks: Checks) -> Element {
    rsx! {
        div { class: "reads-as",
            if !text.is_empty() {
                span { "Reads as: {text}" }
            }
            if checks.adds_up {
                span { class: "check", "adds up" }
            }
            if checks.definition_at_end {
                span { class: "check", "definition at an end" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    #[test]
    fn shows_only_passing_checks() {
        let html = render(move || {
            rsx! {
                ReadsAs { text: "gold + box".to_string(), checks: Checks { adds_up: true, definition_at_end: false } }
            }
        });
        assert!(
            html.contains("gold + box") && html.contains("adds up"),
            "{html}"
        );
        assert!(!html.contains("definition at an end"), "{html}");
    }
}
