//! The line above the grid: which answer is selected, its letters so far,
//! and a box to set how it splits into words.

use dioxus::prelude::*;

#[component]
pub fn AnswerBar(
    /// "3 down".
    #[props(into)]
    label: String,
    /// The letters so far with their word breaks, e.g. "·A·· ·E·T·".
    #[props(into)]
    pattern: String,
    /// The split as it stands: "4,5", "3-4", or just the length.
    #[props(into)]
    split: String,
    /// The split box was changed to this text.
    on_split: EventHandler<String>,
) -> Element {
    // Goes up with every change, so the box is drawn afresh and always shows
    // the split that was kept: one that doesn't add up leaves no trace.
    let mut drawn = use_signal(|| 0u32);
    rsx! {
        div { class: "answer-bar",
            b { class: "answer-bar__label", "{label}" }
            span { class: "answer-bar__pattern", "{pattern}" }
            label {
                class: "answer-bar__split",
                title: "How the answer splits into words, e.g. 4,5 or 3-4",
                "("
                input {
                    key: "{drawn}",
                    class: "answer-bar__input",
                    value: "{split}",
                    spellcheck: "false",
                    autocomplete: "off",
                    "aria-label": "Word split",
                    onchange: move |e| {
                        on_split.call(e.value());
                        drawn += 1;
                    },
                }
                ")"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    #[test]
    fn shows_the_label_the_pattern_and_the_split_box() {
        let html = render(move || {
            rsx! {
                AnswerBar { label: "3 down", pattern: "·A·· ·E·T·", split: "4,5", on_split: |_| {} }
            }
        });
        assert!(html.contains(r#"class="answer-bar""#), "{html}");
        assert!(html.contains(">3 down<"), "{html}");
        assert!(html.contains(">·A·· ·E·T·<"), "{html}");
        assert!(html.contains(r#"value="4,5""#), "{html}");
        assert!(html.contains(r#"aria-label="Word split""#), "{html}");
        let input = html.find("<input").unwrap();
        let (open, close) = (html[..input].rfind('('), html[input..].find(')'));
        assert!(
            open.is_some() && close.is_some(),
            "the box sits in brackets: {html}"
        );
    }
}
