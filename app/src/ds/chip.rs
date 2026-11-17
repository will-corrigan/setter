//! A word you can pick: rounded pill, filled in its tone when pressed.

use dioxus::prelude::*;
use xw_engine::bank::Tone;

#[component]
pub fn Chip(
    #[props(into)] label: String,
    #[props(default)] tone: Option<Tone>,
    #[props(default)] pressed: bool,
    onclick: Option<EventHandler<()>>,
) -> Element {
    rsx! {
        button {
            class: "chip",
            r#type: "button",
            "data-tone": tone.map(Tone::class),
            "aria-pressed": "{pressed}",
            onclick: move |_| {
                if let Some(h) = onclick {
                    h.call(())
                }
            },
            "{label}"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    #[test]
    fn pressed_chip_takes_its_tone() {
        let html =
            render(move || rsx! { Chip { label: "box", tone: Some(Tone::P2), pressed: true } });
        assert!(html.contains(r#"data-tone="p2""#), "{html}");
        assert!(html.contains(r#"aria-pressed="true""#), "{html}");
        assert!(html.contains(">box<"), "{html}");
    }
}
