//! Quiet inline text: counts, small status lines.

use dioxus::prelude::*;

#[component]
pub fn Note(#[props(into)] text: String) -> Element {
    rsx! {
        span { class: "note", "{text}" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    #[test]
    fn renders_muted_text() {
        let html = render(move || rsx! { Note { text: "3 of 28 clued" } });
        assert!(
            html.contains(r#"class="note""#) && html.contains("3 of 28 clued"),
            "{html}"
        );
    }
}
