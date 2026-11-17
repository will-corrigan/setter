//! The line of checks under the grid: what passes, what is wrong, and an
//! action where something can be done about it.

use crate::components::button::{Button, ButtonSize, ButtonVariant};
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StatusTone {
    /// A plain fact, such as a count.
    Plain,
    /// A check that passes.
    Ok,
    /// Something wrong.
    Warn,
}

impl StatusTone {
    fn id(self) -> Option<&'static str> {
        match self {
            StatusTone::Plain => None,
            StatusTone::Ok => Some("ok"),
            StatusTone::Warn => Some("warn"),
        }
    }
}

/// A button beside a status item. `key` says what it acts on.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StatusAction {
    pub label: String,
    pub key: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StatusItem {
    pub tone: StatusTone,
    pub text: String,
    pub action: Option<StatusAction>,
}

impl StatusItem {
    pub fn new(tone: StatusTone, text: impl Into<String>) -> StatusItem {
        StatusItem {
            tone,
            text: text.into(),
            action: None,
        }
    }
}

#[component]
pub fn StatusLine(
    items: Vec<StatusItem>,
    /// An item's action was chosen; this is its key.
    on_action: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "status-line", role: "status",
            for (i, item) in items.into_iter().enumerate() {
                span { key: "{i}", class: "status-line__item", "data-tone": item.tone.id(),
                    span { "{item.text}" }
                    if let Some(action) = item.action {
                        Button {
                            variant: ButtonVariant::Outline,
                            size: ButtonSize::Xs,
                            onclick: move |_| on_action.call(action.key.clone()),
                            "{action.label}"
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    #[test]
    fn items_take_their_tone_and_actions_are_buttons() {
        let items = vec![
            StatusItem::new(StatusTone::Plain, "3 of 30 answers filled"),
            StatusItem::new(StatusTone::Ok, "no repeats"),
            StatusItem {
                tone: StatusTone::Warn,
                text: "ZOB RIST (3 across) is not in the word list".into(),
                action: Some(StatusAction {
                    label: "Add to my words".into(),
                    key: "2,0,a".into(),
                }),
            },
        ];
        let html = render(move || rsx! { StatusLine { items, on_action: |_| {} } });
        assert!(html.contains(r#"class="status-line""#), "{html}");
        assert!(html.contains(r#"role="status""#), "{html}");
        assert_eq!(html.matches("status-line__item").count(), 3, "{html}");
        assert_eq!(html.matches(r#"data-tone="ok""#).count(), 1, "{html}");
        assert_eq!(html.matches(r#"data-tone="warn""#).count(), 1, "{html}");
        assert!(html.contains("3 of 30 answers filled"), "{html}");
        assert_eq!(html.matches("dx-button").count(), 1, "{html}");
        assert!(html.contains("Add to my words"), "{html}");
    }

    #[test]
    fn an_empty_line_still_holds_its_place() {
        let html = render(move || rsx! { StatusLine { items: vec![], on_action: |_| {} } });
        assert!(html.contains(r#"class="status-line""#), "{html}");
        assert!(!html.contains("status-line__item"), "{html}");
    }
}
