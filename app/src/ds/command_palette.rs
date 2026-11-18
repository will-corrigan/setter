//! The command palette: one box to run any command, jump to any answer or
//! open a recent puzzle. A combobox: ↑/↓ move, Enter runs, Esc closes.

use dioxus::prelude::*;

#[derive(Clone, PartialEq, Debug, Default)]
pub struct PaletteItem {
    pub label: String,
    /// Byte ranges of `label` to show bold (what matched).
    pub marks: Vec<(usize, usize)>,
    pub detail: String,
    pub keys: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct PaletteGroup {
    pub title: String,
    pub items: Vec<PaletteItem>,
}

/// A label cut into (text, bold) pieces.
pub fn pieces(label: &str, marks: &[(usize, usize)]) -> Vec<(String, bool)> {
    let mut out = vec![];
    let mut at = 0;
    for &(a, b) in marks {
        if a < at || b > label.len() || !label.is_char_boundary(a) || !label.is_char_boundary(b) {
            continue;
        }
        if a > at {
            out.push((label[at..a].to_string(), false));
        }
        out.push((label[a..b].to_string(), true));
        at = b;
    }
    if at < label.len() {
        out.push((label[at..].to_string(), false));
    }
    out
}

#[component]
pub fn CommandPalette(
    query: String,
    groups: Vec<PaletteGroup>,
    /// The highlighted item, counted across groups.
    active: usize,
    on_query: EventHandler<String>,
    /// Move the highlight by this many items.
    on_move: EventHandler<i32>,
    /// Run the item at this position (counted across groups).
    on_run: EventHandler<usize>,
    on_close: EventHandler<()>,
) -> Element {
    let total: usize = groups.iter().map(|g| g.items.len()).sum();
    use_effect(move || {
        let _ = active;
        document::eval(
            "document.querySelector('.palette__item[aria-selected=\"true\"]')?.scrollIntoView({ block: 'nearest' })",
        );
    });
    let mut n = 0usize;
    let numbered: Vec<(String, Vec<(usize, PaletteItem)>)> = groups
        .into_iter()
        .map(|g| {
            let items = g
                .items
                .into_iter()
                .map(|item| {
                    let i = n;
                    n += 1;
                    (i, item)
                })
                .collect();
            (g.title, items)
        })
        .collect();
    rsx! {
        div { class: "palette-scrim", onclick: move |_| on_close.call(()),
            div {
                class: "palette",
                role: "dialog",
                "aria-label": "Search",
                onclick: move |e| e.stop_propagation(),
                div { class: "palette__search",
                    input {
                        class: "palette__input",
                        r#type: "text",
                        role: "combobox",
                        "aria-label": "Search commands, answers and puzzles",
                        "aria-expanded": "true",
                        "aria-controls": "palette-list",
                        "aria-activedescendant": "palette-item-{active}",
                        spellcheck: "false",
                        autocomplete: "off",
                        value: "{query}",
                        onmounted: move |e| async move {
                            let _ = e.set_focus(true).await;
                        },
                        oninput: move |e| on_query.call(e.value()),
                        onkeydown: move |e: KeyboardEvent| match e.key() {
                            Key::ArrowDown => {
                                e.prevent_default();
                                on_move.call(1);
                            }
                            Key::ArrowUp => {
                                e.prevent_default();
                                on_move.call(-1);
                            }
                            Key::Enter => {
                                e.prevent_default();
                                if total > 0 {
                                    on_run.call(active);
                                }
                            }
                            Key::Escape => on_close.call(()),
                            _ => {}
                        },
                    }
                }
                div { class: "palette__list", id: "palette-list", role: "listbox",
                    if total == 0 {
                        p { class: "palette__empty", "No matches for “{query.trim()}”" }
                    }
                    for (title , items) in numbered {
                        div { key: "{title}", class: "palette__group", role: "group", "aria-label": "{title}",
                            h4 { class: "palette__title", "{title}" }
                            for (i , item) in items {
                                div {
                                    key: "{i}",
                                    id: "palette-item-{i}",
                                    class: "palette__item",
                                    role: "option",
                                    "aria-selected": "{i == active}",
                                    onclick: move |_| on_run.call(i),
                                    span { class: "palette__label",
                                        for (j , (text , bold)) in pieces(&item.label, &item.marks).into_iter().enumerate() {
                                            if bold {
                                                b { key: "{j}", "{text}" }
                                            } else {
                                                span { key: "{j}", "{text}" }
                                            }
                                        }
                                    }
                                    if !item.detail.is_empty() {
                                        span { class: "palette__detail", "{item.detail}" }
                                    }
                                    if let Some(keys) = &item.keys {
                                        span { class: "palette__keys", "{keys}" }
                                    }
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

    fn groups() -> Vec<PaletteGroup> {
        vec![
            PaletteGroup {
                title: "Commands".into(),
                items: vec![
                    PaletteItem {
                        label: "Export web page…".into(),
                        marks: vec![(0, 2)],
                        detail: String::new(),
                        keys: Some("Ctrl+E".into()),
                    },
                    PaletteItem {
                        label: "Next answer".into(),
                        marks: vec![],
                        detail: String::new(),
                        keys: None,
                    },
                ],
            },
            PaletteGroup {
                title: "Answers".into(),
                items: vec![PaletteItem {
                    label: "23 across · EXCESSIVE".into(),
                    marks: vec![(13, 15)],
                    detail: "(9) · no clue".into(),
                    keys: None,
                }],
            },
        ]
    }

    #[test]
    fn groups_items_bold_matches_and_keys() {
        let html = render(move || {
            rsx! {
                CommandPalette { query: "ex", groups: groups(), active: 2, on_query: |_| {}, on_move: |_| {}, on_run: |_| {}, on_close: |_| {} }
            }
        });
        assert!(
            html.contains(r#"role="combobox""#)
                && html.contains(r#"aria-activedescendant="palette-item-2""#),
            "{html}"
        );
        assert!(
            html.contains(">Commands<") && html.contains(">Answers<"),
            "{html}"
        );
        assert!(
            html.contains("<b>Ex</b>") && html.contains("<b>EX</b>"),
            "{html}"
        );
        assert!(
            html.contains(r#"<span class="palette__keys">Ctrl+E</span>"#),
            "{html}"
        );
        assert!(html.contains(">(9) · no clue<"), "{html}");
        // Numbered across groups: the answer is the third item.
        let third = html.find(r#"id="palette-item-2""#).unwrap();
        assert!(html[third..].contains("CESSIVE"), "{html}");
        assert_eq!(html.matches(r#"aria-selected="true""#).count(), 1, "{html}");
    }

    #[test]
    fn no_matches_says_so() {
        let html = render(move || {
            rsx! {
                CommandPalette { query: "zzz", groups: vec![], active: 0, on_query: |_| {}, on_move: |_| {}, on_run: |_| {}, on_close: |_| {} }
            }
        });
        assert!(html.contains("No matches for “zzz”"), "{html}");
    }

    #[test]
    fn labels_are_cut_at_their_marks() {
        assert_eq!(
            pieces("Export", &[(0, 2)]),
            vec![("Ex".into(), true), ("port".into(), false)]
        );
        assert_eq!(pieces("Open", &[]), vec![("Open".into(), false)]);
        assert_eq!(
            pieces("TEXAS", &[(1, 3)]),
            vec![
                ("T".into(), false),
                ("EX".into(), true),
                ("AS".into(), false)
            ]
        );
    }
}
