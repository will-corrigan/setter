//! Step through the puzzle's answers: ‹ WORD label ›.

use crate::components::button::{Button, ButtonSize, ButtonVariant};
use dioxus::prelude::*;

#[component]
pub fn AnswerNav(
    /// The answer in capitals, e.g. "ORCHESTRA".
    #[props(into)]
    word: String,
    /// Its clue number and enumeration, e.g. "10a · 9".
    #[props(into)]
    label: String,
    /// -1 for the previous answer, 1 for the next.
    on_step: EventHandler<i32>,
) -> Element {
    rsx! {
        div { class: "answer-nav", role: "group", aria_label: "Answer",
            Button {
                variant: ButtonVariant::Outline,
                size: ButtonSize::IconXs,
                aria_label: "Previous answer",
                title: "Previous answer (Alt+←)",
                onclick: move |_| on_step.call(-1),
                "‹"
            }
            span { class: "answer-nav__text",
                span { class: "answer-nav__word", "{word}" }
                span { class: "answer-nav__label", "{label}" }
            }
            Button {
                variant: ButtonVariant::Outline,
                size: ButtonSize::IconXs,
                aria_label: "Next answer",
                title: "Next answer (Alt+→)",
                onclick: move |_| on_step.call(1),
                "›"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    #[test]
    fn shows_the_answer_between_two_named_buttons() {
        let html = render(
            move || rsx! { AnswerNav { word: "OR·HESTRA", label: "10a · 9", on_step: |_| {} } },
        );
        assert!(html.contains("answer-nav"), "{html}");
        assert!(html.contains(">OR·HESTRA<"), "{html}");
        assert!(html.contains(">10a · 9<"), "{html}");
        assert!(html.contains(r#"aria-label="Previous answer""#), "{html}");
        assert!(html.contains(r#"aria-label="Next answer""#), "{html}");
        assert_eq!(html.matches("dx-button").count(), 2, "{html}");
    }
}
