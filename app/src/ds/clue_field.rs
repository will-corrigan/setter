//! The clue editor: a textarea over a mirror that colours the picked words.

use dioxus::prelude::*;
use xw_engine::bank::Tone;

#[component]
pub fn ClueField(
    text: String,
    picks: Vec<(String, Tone)>,
    enumeration: String,
    #[props(default = "Write the clue…".to_string(), into)] placeholder: String,
    on_input: EventHandler<String>,
) -> Element {
    let spans = crate::highlight::spans(&text, &picks);
    rsx! {
        div { class: "clue-field",
            div { class: "clue-field__box",
                div { class: "clue-field__mirror", "aria-hidden": "true",
                    for (i, (piece, tone)) in spans.into_iter().enumerate() {
                        if let Some(t) = tone {
                            span { key: "{i}", class: "w", "data-tone": t.class(), "{piece}" }
                        } else {
                            span { key: "{i}", "{piece}" }
                        }
                    }
                    "\u{200b}"
                }
                textarea {
                    class: "clue-field__input",
                    value: "{text}",
                    placeholder: "{placeholder}",
                    spellcheck: "false",
                    "aria-label": "Clue",
                    oninput: move |e| on_input.call(e.value()),
                }
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
    fn mirror_colours_picks() {
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
        assert!(
            html.contains(r#"<span class="w" data-tone="p1">Gold</span>"#),
            "{html}"
        );
        assert!(html.contains(r#"data-tone="def""#), "{html}");
        assert!(html.contains("(9)"), "{html}");
    }
}
