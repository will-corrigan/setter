//! A titled panel of long-form text (credits, licences), kept readable.

use dioxus::prelude::*;

#[component]
pub fn Prose(#[props(into)] title: String, #[props(into)] text: String) -> Element {
    rsx! {
        article { class: "prose",
            h1 { class: "prose__title", "{title}" }
            div { class: "prose__body", "{text}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    #[test]
    fn renders_title_and_text() {
        let html = render(move || rsx! { Prose { title: "Credits", text: "Line one\nLine two" } });
        assert!(
            html.contains("prose") && html.contains("Credits") && html.contains("Line two"),
            "{html}"
        );
    }
}
