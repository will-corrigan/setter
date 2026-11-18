//! Settings' pieces: the sidebar of sections, a titled page, groups of rows
//! (a name, perhaps a line under it, and its control on the right) and keys
//! drawn as chips.

use dioxus::prelude::*;

/// One section in the sidebar.
#[derive(Clone, PartialEq, Debug)]
pub struct SettingsNavItem {
    pub key: String,
    pub label: String,
    /// How many things the setter has added there, if it's a list.
    pub count: Option<usize>,
}

#[component]
pub fn SettingsNav(
    /// "Back to Prize puzzle 12"
    #[props(into)]
    back: String,
    items: Vec<SettingsNavItem>,
    #[props(into)] current: String,
    #[props(into)] query: String,
    on_query: EventHandler<String>,
    on_select: EventHandler<String>,
    on_back: EventHandler<()>,
) -> Element {
    rsx! {
        nav { class: "settings-nav", "aria-label": "Settings",
            button { class: "settings-nav__back", r#type: "button", onclick: move |_| on_back.call(()),
                span { "aria-hidden": "true", "‹" }
                "{back}"
            }
            input {
                class: "settings-nav__find",
                r#type: "search",
                placeholder: "Find a setting",
                "aria-label": "Find a setting",
                value: "{query}",
                oninput: move |e| on_query.call(e.value()),
            }
            for item in items {
                button {
                    key: "{item.key}",
                    class: "settings-nav__item",
                    r#type: "button",
                    "aria-current": (item.key == current).then_some("page"),
                    onclick: {
                        let k = item.key.clone();
                        move |_| on_select.call(k.clone())
                    },
                    span { "{item.label}" }
                    if let Some(n) = item.count {
                        span { class: "settings-nav__count", "{n}" }
                    }
                }
            }
        }
    }
}

/// A section's page: its title and what's in it.
#[component]
pub fn SettingsPage(#[props(into)] title: String, children: Element) -> Element {
    rsx! {
        div { class: "settings-page",
            h1 { class: "settings-page__title", "{title}" }
            {children}
        }
    }
}

/// A titled card of rows.
#[component]
pub fn SettingsGroup(#[props(into, default)] title: String, children: Element) -> Element {
    rsx! {
        section { class: "settings-group",
            if !title.is_empty() {
                h2 { class: "settings-group__title", "{title}" }
            }
            div { class: "settings-group__card", {children} }
        }
    }
}

/// One setting: its name, perhaps a line under it, and its control.
#[component]
pub fn SettingsRow(
    #[props(into)] label: String,
    #[props(into, default)] detail: String,
    children: Element,
) -> Element {
    rsx! {
        div { class: "settings-row",
            div { class: "settings-row__label",
                b { "{label}" }
                if !detail.is_empty() {
                    span { "{detail}" }
                }
            }
            div { class: "settings-row__control", {children} }
        }
    }
}

/// Keys drawn as chips: "Ctrl+Shift+S" → [Ctrl] [Shift] [S]; on a Mac the
/// symbols (⌘ ⇧ S). Alternatives are separated by " · ".
#[component]
pub fn KeyChips(#[props(into)] keys: String) -> Element {
    let combos: Vec<Vec<String>> = keys.split(" · ").map(crate::keys::parts).collect();
    rsx! {
        span { class: "key-chips",
            for (i , combo) in combos.into_iter().enumerate() {
                if i > 0 {
                    span { class: "key-chips__or", "·" }
                }
                for (j , k) in combo.into_iter().enumerate() {
                    kbd { key: "{i}-{j}", class: "key-chips__key", "{k}" }
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
    fn the_nav_marks_the_current_section_and_shows_counts() {
        let items = vec![
            SettingsNavItem {
                key: "general".into(),
                label: "General".into(),
                count: None,
            },
            SettingsNavItem {
                key: "words".into(),
                label: "Word lists".into(),
                count: Some(15),
            },
        ];
        let html = render(move || {
            rsx! {
                SettingsNav { back: "Back to Prize 12", items: items.clone(), current: "words", query: "", on_query: |_| {}, on_select: |_| {}, on_back: |_| {} }
            }
        });
        assert!(html.contains(">Back to Prize 12<"), "{html}");
        assert!(html.contains(r#"placeholder="Find a setting""#), "{html}");
        assert_eq!(html.matches(r#"aria-current="page""#).count(), 1, "{html}");
        let current = html.split(r#"aria-current="page""#).nth(1).unwrap();
        assert!(current.starts_with("><span>Word lists</span>"), "{html}");
        assert!(
            html.contains(r#"<span class="settings-nav__count">15</span>"#),
            "{html}"
        );
    }

    #[test]
    fn rows_have_a_name_a_detail_and_a_control() {
        let html = render(move || {
            rsx! {
                SettingsPage { title: "General",
                    SettingsGroup { title: "You",
                        SettingsRow { label: "Setter name", detail: "On new puzzles", button { "x" } }
                        SettingsRow { label: "Theme", button { "y" } }
                    }
                }
            }
        });
        assert!(
            html.contains(r#"<h1 class="settings-page__title">General</h1>"#),
            "{html}"
        );
        assert!(
            html.contains(">You<")
                && html.contains("<b>Setter name</b><span>On new puzzles</span>"),
            "{html}"
        );
        assert!(
            html.contains("<b>Theme</b></div>"),
            "no empty detail line: {html}"
        );
        assert_eq!(html.matches("settings-row__control").count(), 2, "{html}");
    }

    #[test]
    fn keys_are_chips_with_alternatives() {
        let html = render(move || rsx! { KeyChips { keys: "Ctrl+Z · Ctrl+Shift+Z" } });
        assert_eq!(html.matches("key-chips__key").count(), 5, "{html}");
        assert_eq!(html.matches("key-chips__or").count(), 1, "{html}");
    }
}
