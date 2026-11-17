//! One column of the word bank: a titled, filterable, grouped list of words.

use dioxus::prelude::*;
use xw_engine::bank::Column;

/// Items whose text contains `query` (case-insensitive); empty groups dropped.
pub fn filter_groups(groups: &[(String, Vec<String>)], query: &str) -> Vec<(String, Vec<String>)> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return groups.to_vec();
    }
    groups
        .iter()
        .map(|(g, items)| {
            (
                g.clone(),
                items
                    .iter()
                    .filter(|i| i.to_lowercase().contains(&q))
                    .cloned()
                    .collect::<Vec<_>>(),
            )
        })
        .filter(|(_, items)| !items.is_empty())
        .collect()
}

/// Split `text` into (piece, highlighted) runs at the given byte ranges.
pub fn marked(text: &str, ranges: &[(usize, usize)]) -> Vec<(String, bool)> {
    let mut out = vec![];
    let mut pos = 0;
    let mut sorted = ranges.to_vec();
    sorted.sort_unstable();
    for (a, b) in sorted {
        let (a, b) = (a.max(pos).min(text.len()), b.min(text.len()));
        if a >= b || !text.is_char_boundary(a) || !text.is_char_boundary(b) {
            continue;
        }
        if a > pos {
            out.push((text[pos..a].to_string(), false));
        }
        out.push((text[a..b].to_string(), true));
        pos = b;
    }
    if pos < text.len() {
        out.push((text[pos..].to_string(), false));
    }
    out
}

#[component]
pub fn WordColumn(
    column: Column,
    selected: Option<String>,
    on_pick: EventHandler<String>,
) -> Element {
    let mut query = use_signal(String::new);
    let groups = filter_groups(&column.groups, &query.read());
    let total: usize = column.groups.iter().map(|(_, items)| items.len()).sum();
    let marks_for = |item: &str| -> Vec<(usize, usize)> {
        column
            .marks
            .iter()
            .find(|(t, _)| t == item)
            .map(|(_, r)| r.clone())
            .unwrap_or_default()
    };
    rsx! {
        section { class: "word-column", "data-tone": column.tone.class(), "data-wide": "{column.wide}",
            header { class: "word-column__head",
                span { "{column.title}" }
                span { class: "count", "{total}" }
            }
            input {
                class: "word-column__filter",
                placeholder: "Filter",
                "aria-label": "Filter {column.title}",
                value: "{query}",
                oninput: move |e| query.set(e.value()),
            }
            div { class: "word-column__list", role: "listbox", "aria-label": "{column.title}",
                if groups.is_empty() {
                    p { class: "word-column__empty", if total == 0 { "No words found" } else { "No matches" } }
                }
                for (gi, (group, items)) in groups.into_iter().enumerate() {
                    div { key: "g{gi}",
                        div { class: "word-column__group", "{group}" }
                        div { class: "word-column__items",
                            for (i, item) in items.into_iter().enumerate() {
                                button {
                                    key: "{i}",
                                    class: "word-column__item",
                                    role: "option",
                                    "aria-selected": "{selected.as_deref() == Some(item.as_str())}",
                                    onclick: {
                                        let item = item.clone();
                                        move |_| on_pick.call(item.clone())
                                    },
                                    for (k, (piece, on)) in marked(&item, &marks_for(&item)).into_iter().enumerate() {
                                        if on {
                                            mark { key: "{k}", "{piece}" }
                                        } else {
                                            span { key: "{k}", "{piece}" }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;
    use xw_engine::bank::Tone;

    fn groups() -> Vec<(String, Vec<String>)> {
        vec![
            ("Abbreviation".into(), vec!["gold".into(), "men".into()]),
            ("Synonyms".into(), vec!["coffer".into()]),
        ]
    }

    #[test]
    fn filter_keeps_matching_items_and_drops_empty_groups() {
        assert_eq!(
            filter_groups(&groups(), "CO"),
            vec![("Synonyms".to_string(), vec!["coffer".to_string()])]
        );
        assert_eq!(filter_groups(&groups(), "  "), groups());
    }

    #[test]
    fn marked_splits_text_on_ranges() {
        assert_eq!(
            marked("for chest", &[(1, 3), (4, 9)]),
            vec![
                ("f".to_string(), false),
                ("or".to_string(), true),
                (" ".to_string(), false),
                ("chest".to_string(), true)
            ]
        );
        assert_eq!(marked("box", &[]), vec![("box".to_string(), false)]);
    }

    #[test]
    fn column_renders_title_count_and_selection() {
        let column = Column {
            title: "CHEST".into(),
            tone: Tone::P2,
            wide: false,
            groups: vec![("Synonyms".into(), vec!["box".into(), "coffer".into()])],
            marks: vec![],
        };
        let html = render(
            move || rsx! { WordColumn { column, selected: Some("box".to_string()), on_pick: |_| {} } },
        );
        assert!(html.contains("CHEST"), "{html}");
        assert!(html.contains(">2<"), "{html}");
        assert!(html.contains(r#"aria-selected="true""#), "{html}");
        assert!(html.contains(r#"data-tone="p2""#), "{html}");
        assert!(
            html.contains("word-column__filter"),
            "every column has a filter: {html}"
        );
    }
}
