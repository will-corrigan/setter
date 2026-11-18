//! The app's top bar: brand and `start` items on the left, `middle` centred,
//! `end` items on the right.

use dioxus::prelude::*;

#[component]
pub fn TopBar(
    #[props(into)] brand: String,
    start: Element,
    #[props(default)] middle: Option<Element>,
    end: Element,
    /// The brand was clicked (it goes home).
    #[props(default)]
    on_brand: Option<EventHandler<()>>,
) -> Element {
    rsx! {
        header { class: "topbar",
            div { class: "topbar__start",
                if let Some(go) = on_brand {
                    button {
                        class: "topbar__logo topbar__logo--button",
                        r#type: "button",
                        title: "Home",
                        onclick: move |_| go.call(()),
                        "{brand}"
                    }
                } else {
                    span { class: "topbar__logo", "{brand}" }
                }
                {start}
            }
            div { class: "topbar__middle",
                if let Some(m) = middle {
                    {m}
                }
            }
            div { class: "topbar__end", {end} }
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
            html.find("topbar__end").unwrap(),
            html.find("save").unwrap(),
        );
        assert!(b < m && m < sp && sp < s, "{html}");
        assert!(html.contains("<header class=\"topbar\""), "{html}");
    }
}
