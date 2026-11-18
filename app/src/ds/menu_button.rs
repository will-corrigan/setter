//! A button that opens a short menu of actions (File, settings).

use dioxus::prelude::*;

/// One line of a menu.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct MenuItem {
    pub label: String,
    /// Shown greyed at the right: the keys that do the same ("⌘S").
    pub keys: Option<String>,
    /// Can't be used right now: shown greyed, not hidden.
    pub disabled: bool,
    /// For a choice among several (Light, Dark): a tick when chosen.
    pub checked: Option<bool>,
    /// A thin line above this item, starting a new group.
    pub separated: bool,
    /// A small heading instead of an action ("Recent").
    pub heading: bool,
}

impl MenuItem {
    pub fn new(label: impl Into<String>) -> MenuItem {
        MenuItem {
            label: label.into(),
            ..Default::default()
        }
    }
    pub fn keys(mut self, keys: impl Into<String>) -> MenuItem {
        self.keys = Some(keys.into());
        self
    }
    pub fn disabled(mut self, disabled: bool) -> MenuItem {
        self.disabled = disabled;
        self
    }
    pub fn checked(mut self, checked: bool) -> MenuItem {
        self.checked = Some(checked);
        self
    }
    pub fn separated(mut self) -> MenuItem {
        self.separated = true;
        self
    }
    pub fn heading(label: impl Into<String>) -> MenuItem {
        MenuItem {
            label: label.into(),
            heading: true,
            separated: true,
            ..Default::default()
        }
    }
}

#[component]
pub fn MenuButton(
    /// The button's text; with an `icon`, its tooltip and accessible name.
    #[props(into)]
    label: String,
    /// Shown instead of the text, e.g. a gear.
    #[props(default)]
    icon: Option<Element>,
    items: Vec<MenuItem>,
    #[props(default)] open: bool,
    /// The position of the chosen item in `items`.
    on_pick: EventHandler<usize>,
) -> Element {
    let mut is_open = use_signal(|| open);
    let has_icon = icon.is_some();
    rsx! {
        div { class: "menu-button",
            button {
                class: if has_icon { "menu-button__trigger menu-button__trigger--icon" } else { "menu-button__trigger" },
                r#type: "button",
                "aria-haspopup": "menu",
                "aria-expanded": "{is_open()}",
                "aria-label": "{label}",
                title: "{label}",
                onclick: move |_| is_open.toggle(),
                if let Some(icon) = icon {
                    {icon}
                } else {
                    "{label}"
                }
            }
            if is_open() {
                div { class: "menu-button__backdrop", onclick: move |_| is_open.set(false) }
                div { class: "menu-button__list", role: "menu", "aria-label": "{label}",
                    for (i, item) in items.iter().enumerate() {
                        if item.separated && i > 0 {
                            div { key: "sep{i}", class: "menu-button__sep", role: "separator" }
                        }
                        if item.heading {
                            div { key: "{i}", class: "menu-button__heading", "{item.label}" }
                        } else {
                            button {
                                key: "{i}",
                                class: "menu-button__item",
                                r#type: "button",
                                role: if item.checked.is_some() { "menuitemradio" } else { "menuitem" },
                                "aria-checked": item.checked.map(|c| c.to_string()),
                                disabled: item.disabled,
                                onclick: move |_| {
                                    is_open.set(false);
                                    on_pick.call(i);
                                },
                                if let Some(checked) = item.checked {
                                    span { class: "menu-button__check", if checked { "✓" } }
                                }
                                span { class: "menu-button__label", "{item.label}" }
                                if let Some(keys) = &item.keys {
                                    span { class: "menu-button__keys", "{keys}" }
                                }
                            }
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

    fn items() -> Vec<MenuItem> {
        vec![
            MenuItem::new("Open…").keys("Ctrl+O"),
            MenuItem::new("Save as…").keys("Ctrl+Shift+S"),
            MenuItem::new("Export web page…").separated(),
            MenuItem::new("Show in folder").disabled(true),
            MenuItem::heading("Recent"),
            MenuItem::new("Prize puzzle 12"),
        ]
    }

    #[test]
    fn closed_menu_shows_only_the_button() {
        let html =
            render(move || rsx! { MenuButton { label: "File", items: items(), on_pick: |_| {} } });
        assert!(
            html.contains(">File<") && html.contains(r#"aria-haspopup="menu""#),
            "{html}"
        );
        assert!(html.contains(r#"aria-expanded="false""#), "{html}");
        assert!(
            !html.contains("Save as"),
            "items only render when open: {html}"
        );
    }

    #[test]
    fn open_menu_has_keys_groups_headings_and_greyed_items() {
        let html = render(
            move || rsx! { MenuButton { label: "File", open: true, items: items(), on_pick: |_| {} } },
        );
        assert_eq!(html.matches(r#"role="menuitem""#).count(), 5, "{html}");
        assert!(
            html.contains(r#"<span class="menu-button__keys">Ctrl+Shift+S</span>"#),
            "{html}"
        );
        assert_eq!(html.matches("menu-button__sep").count(), 2, "{html}");
        assert!(
            html.contains(r#"<div class="menu-button__heading">Recent</div>"#),
            "{html}"
        );
        let folder = html.find("Show in folder").unwrap();
        let tag = &html[html[..folder].rfind("<button").unwrap()..folder];
        assert!(tag.contains("disabled"), "{tag}");
    }

    #[test]
    fn choices_show_a_tick_and_an_icon_can_stand_for_the_label() {
        let theme = vec![
            MenuItem::new("Light").checked(true),
            MenuItem::new("Dark").checked(false),
        ];
        let html = render(move || {
            rsx! {
                MenuButton { label: "Settings", icon: rsx! { svg { class: "icon" } }, open: true, items: theme.clone(), on_pick: |_| {} }
            }
        });
        assert!(
            html.contains(r#"aria-label="Settings""#) && html.contains("<svg"),
            "{html}"
        );
        assert!(
            !html.contains(">Settings<"),
            "the icon replaces the text: {html}"
        );
        assert_eq!(html.matches(r#"role="menuitemradio""#).count(), 2, "{html}");
        assert!(
            html.contains(r#"aria-checked="true""#) && html.contains("✓"),
            "{html}"
        );
    }
}
