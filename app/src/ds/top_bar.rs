//! The app's top bar: brand on the left, then `start` items, then `end` items
//! pushed to the right.

use dioxus::prelude::*;

#[component]
pub fn TopBar(#[props(into)] brand: String, start: Element, end: Element) -> Element {
    rsx! {
        header { class: "topbar",
            span { class: "topbar__logo", "{brand}" }
            {start}
            span { class: "topbar__spacer" }
            {end}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    #[test]
    fn brand_start_then_end_on_the_right() {
        let html = render(move || {
            rsx! {
                TopBar { brand: "Setter", start: rsx! { span { "mode" } }, end: rsx! { span { "save" } } }
            }
        });
        let (b, m, sp, s) = (
            html.find("Setter").unwrap(),
            html.find("mode").unwrap(),
            html.find("topbar__spacer").unwrap(),
            html.find("save").unwrap(),
        );
        assert!(b < m && m < sp && sp < s, "{html}");
        assert!(html.contains("<header class=\"topbar\""), "{html}");
    }
}
