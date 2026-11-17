//! The clues set out like a printed puzzle: Across and Down side by side,
//! each row a number, its clue and its enumeration. Answers are never shown.

use dioxus::prelude::*;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ClueRow {
    /// What `on_select` is given for this row (the light id).
    pub key: String,
    pub number: u32,
    /// `None` until a clue has been written.
    pub clue: Option<String>,
    /// "5", "4,5", "3-4".
    pub enumeration: String,
}

#[component]
fn Column(
    title: &'static str,
    rows: Vec<ClueRow>,
    current: Option<String>,
    on_select: EventHandler<String>,
) -> Element {
    rsx! {
        section { class: "clue-sheet__column", "aria-label": title,
            h3 { class: "clue-sheet__title", "{title}" }
            for row in rows {
                button {
                    key: "{row.key}",
                    class: "clue-sheet__row",
                    r#type: "button",
                    "aria-current": (current.as_deref() == Some(row.key.as_str())).then_some("true"),
                    onclick: {
                        let key = row.key.clone();
                        move |_| on_select.call(key.clone())
                    },
                    b { class: "clue-sheet__number", "{row.number}" }
                    if let Some(clue) = row.clue {
                        span { class: "clue-sheet__clue",
                            "{clue} "
                            span { class: "clue-sheet__enum", "({row.enumeration})" }
                        }
                    } else {
                        span { class: "clue-sheet__missing", "no clue" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ClueSheet(
    across: Vec<ClueRow>,
    down: Vec<ClueRow>,
    /// The key of the selected answer's row.
    current: Option<String>,
    on_select: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "clue-sheet",
            Column { title: "Across", rows: across, current: current.clone(), on_select }
            Column { title: "Down", rows: down, current, on_select }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    fn row(key: &str, number: u32, clue: Option<&str>, enumeration: &str) -> ClueRow {
        ClueRow {
            key: key.into(),
            number,
            clue: clue.map(str::to_string),
            enumeration: enumeration.into(),
        }
    }

    fn sheet() -> String {
        render(move || {
            rsx! {
                ClueSheet {
                    across: vec![
                        row("0,0,a", 1, Some("Reset badly for some woodland"), "5"),
                        row("2,0,a", 9, None, "9"),
                    ],
                    down: vec![row("0,0,d", 1, Some("Bill with the French put forward"), "2,3")],
                    current: Some("0,0,d".to_string()),
                    on_select: |_| {},
                }
            }
        })
    }

    #[test]
    fn two_columns_of_numbered_clues_with_enumerations() {
        let html = sheet();
        assert!(html.contains(r#"class="clue-sheet""#), "{html}");
        let (across, down) = (html.find(">Across<").unwrap(), html.find(">Down<").unwrap());
        assert!(across < down, "{html}");
        assert_eq!(html.matches("clue-sheet__column").count(), 2, "{html}");
        assert_eq!(html.matches("clue-sheet__row").count(), 3, "{html}");
        assert!(
            html.contains(r#"<b class="clue-sheet__number">1</b>"#),
            "{html}"
        );
        assert!(html.contains("Reset badly for some woodland"), "{html}");
        assert!(
            html.contains(r#"<span class="clue-sheet__enum">(5)</span>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<span class="clue-sheet__enum">(2,3)</span>"#),
            "{html}"
        );
    }

    #[test]
    fn a_row_with_no_clue_shows_its_number_and_a_tag() {
        let html = sheet();
        assert_eq!(html.matches("clue-sheet__missing").count(), 1, "{html}");
        let tag = html.find(">no clue<").expect("the tag");
        let number = html.find(r#"<b class="clue-sheet__number">9</b>"#).unwrap();
        assert!(number < tag, "{html}");
        // No enumeration beside the tag: just the number and the tag.
        assert!(!html.contains("(9)"), "{html}");
    }

    #[test]
    fn the_selected_answers_row_is_marked() {
        let html = sheet();
        assert_eq!(html.matches(r#"aria-current="true""#).count(), 1, "{html}");
        let marked = &html[html.find(r#"aria-current="true""#).unwrap()..];
        assert!(marked.starts_with(r#"aria-current="true""#));
        let end = marked.find("</button>").unwrap();
        assert!(marked[..end].contains("Bill with the French"), "{html}");
    }
}
