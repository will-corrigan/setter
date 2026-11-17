//! One line of news under the top bar, shown until it is dismissed.

use crate::components::button::{Button, ButtonSize, ButtonVariant};
use dioxus::prelude::*;

#[component]
pub fn Notice(#[props(into)] message: String, on_dismiss: EventHandler<()>) -> Element {
    rsx! {
        div { class: "notice", role: "status",
            span { class: "notice__text", "{message}" }
            Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Xs,
                onclick: move |_| on_dismiss.call(()),
                "Dismiss"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    #[test]
    fn shows_the_message_and_a_dismiss_button() {
        let html = render(
            move || rsx! { Notice { message: "abbreviations.toml: bad line", on_dismiss: |_| {} } },
        );
        assert!(html.contains(r#"class="notice""#), "{html}");
        assert!(html.contains(r#"role="status""#), "{html}");
        assert!(html.contains("abbreviations.toml: bad line"), "{html}");
        assert!(
            html.contains("dx-button") && html.contains("Dismiss"),
            "{html}"
        );
    }
}
