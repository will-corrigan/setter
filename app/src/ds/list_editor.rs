//! A list the setter keeps: add to it, filter it, remove from it. Used for
//! their words, the words never to suggest, abbreviations and indicators.

use dioxus::prelude::*;

#[derive(Clone, PartialEq, Debug)]
pub struct ListRow {
    /// The word, phrase or letters.
    pub text: String,
    /// Beside it, quieter: an enumeration ("6,7") or meanings.
    pub detail: String,
    /// At the right: when it was added ("added today"), if known.
    pub when: String,
}

/// Keep rows whose text or detail contains `query` (any case).
pub fn filtered(rows: &[ListRow], query: &str) -> Vec<ListRow> {
    let q = query.trim().to_lowercase();
    rows.iter()
        .filter(|r| {
            q.is_empty()
                || r.text.to_lowercase().contains(&q)
                || r.detail.to_lowercase().contains(&q)
        })
        .cloned()
        .collect()
}

#[component]
pub fn ListEditor(
    rows: Vec<ListRow>,
    /// Tabs above the list, each with its count; none for a single list.
    #[props(default)]
    tabs: Vec<(String, usize)>,
    #[props(default)] tab: usize,
    #[props(default)] on_tab: Option<EventHandler<usize>>,
    #[props(into)] placeholder: String,
    /// A second box beside the first (abbreviations: their meanings).
    #[props(default)]
    second_placeholder: Option<String>,
    /// What was typed: (first box, second box).
    on_add: EventHandler<(String, String)>,
    on_remove: EventHandler<String>,
    #[props(default)] on_import: Option<EventHandler<()>>,
    #[props(default)] on_export: Option<EventHandler<()>>,
) -> Element {
    let mut first = use_signal(String::new);
    let mut second = use_signal(String::new);
    let mut query = use_signal(String::new);
    let shown = filtered(&rows, &query());
    let mut add = move || {
        let a = first.peek().trim().to_string();
        if a.is_empty() {
            return;
        }
        on_add.call((a, second.peek().trim().to_string()));
        first.set(String::new());
        second.set(String::new());
    };
    let total = rows.len();
    rsx! {
        div { class: "list-editor",
            if !tabs.is_empty() {
                div { class: "list-editor__tabs", role: "tablist",
                    for (i , (name , n)) in tabs.into_iter().enumerate() {
                        button {
                            key: "{i}",
                            class: "list-editor__tab",
                            r#type: "button",
                            role: "tab",
                            "aria-selected": "{i == tab}",
                            onclick: move |_| {
                                if let Some(h) = on_tab {
                                    h.call(i);
                                }
                            },
                            "{name}"
                            span { class: "list-editor__tab-count", "{n}" }
                        }
                    }
                }
            }
            form {
                class: "list-editor__add",
                onsubmit: move |e| {
                    e.prevent_default();
                    add();
                },
                input {
                    class: "list-editor__input",
                    r#type: "text",
                    placeholder: "{placeholder}",
                    "aria-label": "{placeholder}",
                    value: "{first}",
                    oninput: move |e| first.set(e.value()),
                }
                if let Some(p) = &second_placeholder {
                    input {
                        class: "list-editor__input list-editor__input--wide",
                        r#type: "text",
                        placeholder: "{p}",
                        "aria-label": "{p}",
                        value: "{second}",
                        oninput: move |e| second.set(e.value()),
                    }
                }
                button { class: "list-editor__add-button", r#type: "submit", "Add" }
            }
            if total > 0 || on_import.is_some() {
                div { class: "list-editor__tools",
                    if total > 0 {
                        input {
                            class: "list-editor__filter",
                            r#type: "search",
                            placeholder: "Filter",
                            "aria-label": "Filter the list",
                            value: "{query}",
                            oninput: move |e| query.set(e.value()),
                        }
                    }
                    span { class: "list-editor__spacer" }
                    if let Some(h) = on_import {
                        button { class: "list-editor__link", r#type: "button", onclick: move |_| h.call(()), "Import…" }
                    }
                    if let (Some(h), true) = (on_export, total > 0) {
                        button { class: "list-editor__link", r#type: "button", onclick: move |_| h.call(()), "Export…" }
                    }
                }
            }
            div { class: "list-editor__rows", role: "list",
                if total == 0 {
                    p { class: "list-editor__empty", "Nothing yet" }
                } else if shown.is_empty() {
                    p { class: "list-editor__empty", "No matches" }
                }
                for row in shown {
                    div { key: "{row.text}", class: "list-editor__row", role: "listitem",
                        b { class: "list-editor__text", "{row.text}" }
                        if !row.detail.is_empty() {
                            span { class: "list-editor__detail", "{row.detail}" }
                        }
                        span { class: "list-editor__when", "{row.when}" }
                        button {
                            class: "list-editor__remove",
                            r#type: "button",
                            "aria-label": "Remove {row.text}",
                            title: "Remove",
                            onclick: {
                                let t = row.text.clone();
                                move |_| on_remove.call(t.clone())
                            },
                            "×"
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

    fn rows() -> Vec<ListRow> {
        vec![
            ListRow {
                text: "minute cryptic".into(),
                detail: "6,7".into(),
                when: "added today".into(),
            },
            ListRow {
                text: "quixote".into(),
                detail: "7".into(),
                when: String::new(),
            },
        ]
    }

    #[test]
    fn a_list_with_tabs_an_adder_tools_and_rows() {
        let html = render(move || {
            rsx! {
                ListEditor {
                    rows: rows(),
                    tabs: vec![("Your words".to_string(), 2), ("Never suggest".to_string(), 0)],
                    tab: 0,
                    on_tab: |_| {},
                    placeholder: "Add a word or phrase",
                    on_add: |_| {},
                    on_remove: |_| {},
                    on_import: |_| {},
                    on_export: |_| {},
                }
            }
        });
        assert!(
            html.contains(r#"aria-selected="true""#) && html.contains(">Your words<"),
            "{html}"
        );
        assert!(
            html.contains(r#"placeholder="Add a word or phrase""#),
            "{html}"
        );
        assert!(
            html.contains(">Import…<") && html.contains(">Export…<"),
            "{html}"
        );
        assert!(
            html.contains(">minute cryptic<")
                && html.contains(">6,7<")
                && html.contains(">added today<"),
            "{html}"
        );
        assert!(html.contains(r#"aria-label="Remove quixote""#), "{html}");
    }

    #[test]
    fn an_empty_list_says_so_and_has_nothing_to_export() {
        let html = render(move || {
            rsx! {
                ListEditor {
                    rows: vec![],
                    placeholder: "Letters",
                    second_placeholder: "What they stand for".to_string(),
                    on_add: |_| {},
                    on_remove: |_| {},
                }
            }
        });
        assert!(html.contains(">Nothing yet<"), "{html}");
        assert!(
            html.contains(r#"placeholder="What they stand for""#),
            "{html}"
        );
        assert!(
            !html.contains("Export") && !html.contains("Filter"),
            "{html}"
        );
    }

    #[test]
    fn the_filter_matches_text_or_detail() {
        assert_eq!(filtered(&rows(), "CRYP").len(), 1);
        assert_eq!(filtered(&rows(), "7").len(), 2);
        assert_eq!(filtered(&rows(), " ").len(), 2);
    }
}
