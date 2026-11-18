//! Single-choice segmented control (or a row of pills), built on the official
//! Dioxus ToggleGroup so keyboard and screen-reader behaviour come for free.

use crate::components::toggle_group::{ToggleGroup, ToggleItem};
use dioxus::prelude::*;
use std::collections::HashSet;

#[component]
pub fn Segmented(
    #[props(into)] label: String,
    options: Vec<String>,
    selected: usize,
    #[props(default)] pills: bool,
    on_change: EventHandler<usize>,
) -> Element {
    rsx! {
        ToggleGroup {
            horizontal: true,
            pressed: Some(HashSet::from([selected])),
            on_pressed_change: move |set: HashSet<usize>| {
                // Clicking the current option would un-press it; keep one chosen.
                if let Some(&i) = set.iter().next() {
                    on_change.call(i);
                }
            },
            "aria-label": "{label}",
            "data-variant": if pills { "pills" } else { "segmented" },
            // Where the sliding bubble sits: one of `--count` equal slots.
            style: "--count: {options.len()}; --at: {selected}",
            for (i, text) in options.into_iter().enumerate() {
                ToggleItem { key: "{i}", index: i, "{text}" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    #[test]
    fn renders_options_with_one_on() {
        let html = render(move || {
            rsx! {
                Segmented {
                    label: "Mode",
                    options: vec!["Build grid".to_string(), "Write clues".to_string()],
                    selected: 1,
                    on_change: |_| {},
                }
            }
        });
        assert!(
            html.contains("Build grid") && html.contains("Write clues"),
            "{html}"
        );
        assert_eq!(html.matches(r#"data-state="on""#).count(), 1, "{html}");
        assert!(html.contains(r#"aria-label="Mode""#), "{html}");
    }

    #[test]
    fn pills_variant_is_marked() {
        let html = render(move || {
            rsx! {
                Segmented { label: "Options", options: vec!["A".to_string()], selected: 0, pills: true, on_change: |_| {} }
            }
        });
        assert!(html.contains(r#"data-variant="pills""#), "{html}");
    }
}
