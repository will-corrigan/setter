//! The clue editor. The picked words show as coloured pieces that can be
//! dragged to another gap; everything else is plain text to type freely.
//!
//! Rust never fills the editor itself: it sets `data-segments` (the clue as
//! pieces) and `clue_field.js` draws them and keeps the caret. Typing comes
//! back through the hidden sink input, a drag through the arrange input.

use dioxus::prelude::*;
use xw_engine::bank::Tone;

const SCRIPT: &str = include_str!("clue_field.js");

/// The clue as pieces for the editor: `[{"t": text, "tone": "p1" | null}]`.
pub fn segments(text: &str, picks: &[(String, Tone)]) -> String {
    let pieces: Vec<serde_json::Value> = crate::highlight::spans(text, picks)
        .into_iter()
        .map(|(t, tone)| serde_json::json!({ "t": t, "tone": tone.map(Tone::class) }))
        .collect();
    serde_json::Value::Array(pieces).to_string()
}

#[component]
pub fn ClueField(
    text: String,
    picks: Vec<(String, Tone)>,
    enumeration: String,
    #[props(default = "Write the clue…".to_string(), into)] placeholder: String,
    on_input: EventHandler<String>,
    /// The pieces were dragged into a new order.
    #[props(default)]
    on_arrange: Option<EventHandler<String>>,
) -> Element {
    use_effect(|| {
        document::eval(SCRIPT);
    });
    let segments = segments(&text, &picks);
    rsx! {
        div { class: "clue-field",
            div {
                class: "clue-field__editor",
                contenteditable: "true",
                role: "textbox",
                "aria-label": "Clue",
                spellcheck: "false",
                "data-placeholder": "{placeholder}",
                "data-segments": "{segments}",
            }
            input {
                class: "clue-field__sink",
                r#type: "text",
                tabindex: "-1",
                "aria-hidden": "true",
                value: "{text}",
                oninput: move |e| on_input.call(e.value()),
            }
            input {
                class: "clue-field__arrange",
                r#type: "text",
                tabindex: "-1",
                "aria-hidden": "true",
                oninput: move |e| match on_arrange {
                    Some(h) => h.call(e.value()),
                    None => on_input.call(e.value()),
                },
            }
            span { class: "clue-field__enum", "({enumeration})" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    #[test]
    fn the_editor_gets_the_clue_and_its_pieces() {
        let html = render(move || {
            rsx! {
                ClueField {
                    text: "Gold box for band".to_string(),
                    picks: vec![("gold".to_string(), Tone::P1), ("band".to_string(), Tone::Definition)],
                    enumeration: "9".to_string(),
                    on_input: |_| {},
                }
            }
        });
        // The editor carries the clue as pieces for its script to draw.
        assert!(html.contains(r#"contenteditable="true""#), "{html}");
        assert!(html.contains("data-segments="), "{html}");
        assert!(html.contains("(9)"), "{html}");
        assert!(html.contains(r#"value="Gold box for band""#), "{html}");
    }
}

#[cfg(test)]
mod segment_tests {
    use super::*;

    #[test]
    fn segments_mark_the_picked_words() {
        let json = segments(
            "Gold box for band",
            &[("gold".into(), Tone::P1), ("band".into(), Tone::Definition)],
        );
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v[0]["t"], "Gold");
        assert_eq!(v[0]["tone"], "p1");
        assert_eq!(v[1]["t"], " box for ");
        assert!(v[1]["tone"].is_null());
        assert_eq!(v[2]["tone"], "def");
        let joined: String = v
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["t"].as_str().unwrap())
            .collect();
        assert_eq!(joined, "Gold box for band");
    }
}
