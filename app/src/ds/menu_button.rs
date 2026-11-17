//! A button that opens a short list of actions (e.g. Export formats).

use dioxus::prelude::*;

#[component]
pub fn MenuButton(
    #[props(into)] label: String,
    items: Vec<String>,
    #[props(default)] open: bool,
    on_pick: EventHandler<usize>,
) -> Element {
    let mut is_open = use_signal(|| open);
    rsx! {
        div { class: "menu-button",
            button {
                class: "menu-button__trigger",
                r#type: "button",
                "aria-haspopup": "menu",
                "aria-expanded": "{is_open()}",
                onclick: move |_| is_open.toggle(),
                "{label}"
            }
            if is_open() {
                div { class: "menu-button__backdrop", onclick: move |_| is_open.set(false) }
                div { class: "menu-button__list", role: "menu", "aria-label": "{label}",
                    for (i, item) in items.iter().enumerate() {
                        button {
                            key: "{i}",
                            class: "menu-button__item",
                            r#type: "button",
                            role: "menuitem",
                            onclick: move |_| {
                                is_open.set(false);
                                on_pick.call(i);
                            },
                            "{item}"
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
    fn closed_menu_shows_only_the_button() {
        let html = render(move || {
            rsx! {
                MenuButton { label: "Export", items: vec!["Web page (.html)".to_string(), "Across Lite (.puz)".to_string()], on_pick: |_| {} }
            }
        });
        assert!(
            html.contains(">Export<") && html.contains(r#"aria-haspopup="menu""#),
            "{html}"
        );
        assert!(html.contains(r#"aria-expanded="false""#), "{html}");
        assert!(
            !html.contains("Across Lite"),
            "items only render when open: {html}"
        );
    }

    #[test]
    fn open_menu_lists_items() {
        let html = render(move || {
            rsx! {
                MenuButton { label: "Export", open: true, items: vec!["Web page (.html)".to_string(), "Across Lite (.puz)".to_string()], on_pick: |_| {} }
            }
        });
        assert_eq!(html.matches(r#"role="menuitem""#).count(), 2, "{html}");
        assert!(html.contains(r#"aria-expanded="true""#), "{html}");
    }
}
