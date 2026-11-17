//! A centred one-line message for empty or not-yet-available areas.

use dioxus::prelude::*;

#[component]
pub fn EmptyState(#[props(into)] message: String) -> Element {
    rsx! {
        div { class: "empty-state", role: "status", "{message}" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    #[test]
    fn renders_message() {
        let html = render(move || rsx! { EmptyState { message: "Nothing here" } });
        assert!(
            html.contains("empty-state") && html.contains("Nothing here"),
            "{html}"
        );
    }
}
