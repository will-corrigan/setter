//! Grouped navigation list with counts (the device list).

use dioxus::prelude::*;

#[derive(Clone, PartialEq, Debug)]
pub struct NavItem {
    pub key: String,
    pub label: String,
    pub count: usize,
    pub group: String,
}

#[component]
pub fn SideNav(
    items: Vec<NavItem>,
    current: String,
    #[props(default = "Devices".to_string(), into)] label: String,
    /// The counts aren't known yet: show the names only.
    #[props(default)]
    busy: bool,
    on_select: EventHandler<String>,
) -> Element {
    let mut groups: Vec<(String, Vec<NavItem>)> = vec![];
    for item in items {
        match groups.last_mut() {
            Some((g, list)) if *g == item.group => list.push(item),
            _ => groups.push((item.group.clone(), vec![item])),
        }
    }
    rsx! {
        nav { class: "side-nav", "aria-label": "{label}", "aria-busy": busy.then_some("true"),
            for (group, list) in groups {
                div { key: "{group}", role: "group", "aria-label": "{group}",
                    h2 { class: "side-nav__group", "{group}" }
                    for item in list {
                        button {
                            key: "{item.key}",
                            class: if item.count == 0 && !busy { "side-nav__item is-empty" } else { "side-nav__item" },
                            "aria-current": (item.key == current).then_some("true"),
                            onclick: {
                                let k = item.key.clone();
                                move |_| on_select.call(k.clone())
                            },
                            span { "{item.label}" }
                            if !busy {
                                span { class: "side-nav__count", "{item.count}" }
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

    #[test]
    fn groups_items_and_marks_current_and_empty() {
        let items = vec![
            NavItem {
                key: "a".into(),
                label: "Anagram".into(),
                count: 412,
                group: "Wordplay".into(),
            },
            NavItem {
                key: "c".into(),
                label: "Container".into(),
                count: 0,
                group: "Wordplay".into(),
            },
            NavItem {
                key: "d".into(),
                label: "Double definition".into(),
                count: 6,
                group: "Meaning only".into(),
            },
        ];
        let html =
            render(move || rsx! { SideNav { items, current: "a".to_string(), on_select: |_| {} } });
        assert_eq!(html.matches("side-nav__group").count(), 2, "{html}");
        assert!(html.contains(r#"aria-current="true""#), "{html}");
        assert!(html.contains("is-empty"), "{html}");
        assert!(html.contains("412"), "{html}");
        assert!(!html.contains("aria-busy"), "{html}");
    }

    #[test]
    fn busy_nav_hides_counts_and_greys_nothing() {
        let items = vec![
            NavItem {
                key: "a".into(),
                label: "Anagram".into(),
                count: 0,
                group: "Wordplay".into(),
            },
            NavItem {
                key: "c".into(),
                label: "Container".into(),
                count: 0,
                group: "Wordplay".into(),
            },
        ];
        let html = render(
            move || rsx! { SideNav { items, current: String::new(), busy: true, on_select: |_| {} } },
        );
        assert!(html.contains(r#"aria-busy="true""#), "{html}");
        assert!(
            html.contains("Anagram") && html.contains("Container"),
            "{html}"
        );
        assert!(!html.contains("side-nav__count"), "{html}");
        assert!(!html.contains("is-empty"), "{html}");
    }
}
