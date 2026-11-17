//! Text that becomes an input when clicked (puzzle title, setter name).

use dioxus::prelude::*;

#[component]
pub fn InlineEdit(
    #[props(into)] value: String,
    #[props(into)] placeholder: String,
    #[props(into)] label: String,
    #[props(default)] editing: bool,
    on_change: EventHandler<String>,
) -> Element {
    let mut is_editing = use_signal(|| editing);
    let mut draft = use_signal(|| value.clone());
    let shown = if value.trim().is_empty() {
        placeholder.clone()
    } else {
        value.clone()
    };
    let mut commit = move || {
        is_editing.set(false);
        on_change.call(draft.peek().trim().to_string());
    };
    rsx! {
        if is_editing() {
            input {
                class: "inline-edit inline-edit--input",
                "aria-label": "{label}",
                value: "{draft}",
                placeholder: "{placeholder}",
                autofocus: true,
                oninput: move |e| draft.set(e.value()),
                onblur: move |_| commit(),
                onkeydown: move |e: KeyboardEvent| match e.key() {
                    Key::Enter => commit(),
                    Key::Escape => is_editing.set(false),
                    _ => {}
                },
            }
        } else {
            button {
                class: if value.trim().is_empty() { "inline-edit is-placeholder" } else { "inline-edit" },
                r#type: "button",
                "aria-label": "{label}",
                title: "{label}",
                onclick: {
                    let value = value.clone();
                    move |_| {
                        draft.set(value.clone());
                        is_editing.set(true);
                    }
                },
                "{shown}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    #[test]
    fn shows_value_or_placeholder() {
        let html = render(
            move || rsx! { InlineEdit { value: "Prize 12", placeholder: "Untitled", label: "Puzzle title", on_change: |_| {} } },
        );
        assert!(
            html.contains("Prize 12") && html.contains(r#"aria-label="Puzzle title""#),
            "{html}"
        );
        let html = render(
            move || rsx! { InlineEdit { value: "", placeholder: "Untitled", label: "Puzzle title", on_change: |_| {} } },
        );
        assert!(
            html.contains("Untitled") && html.contains("is-placeholder"),
            "{html}"
        );
    }

    #[test]
    fn editing_shows_an_input() {
        let html = render(
            move || rsx! { InlineEdit { value: "Prize 12", placeholder: "Untitled", label: "Puzzle title", editing: true, on_change: |_| {} } },
        );
        assert!(
            html.contains("<input") && html.contains(r#"value="Prize 12""#),
            "{html}"
        );
    }
}
