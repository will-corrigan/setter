//! The words that fit an answer, in a popover beside it: a header with the
//! answer's name and two icon buttons, a search box, word-count chips and
//! the list. Picking a word places it.

use crate::components::button::{Button, ButtonSize, ButtonVariant};
use crate::ds::icon::{Icon, IconName};
use crate::ds::segmented::Segmented;
use dioxus::prelude::*;

/// Which side of the answer the popover opens on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Right,
    Left,
}

/// Where the popover goes, in grid squares from the grid's top-left corner.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Placement {
    pub side: Side,
    /// On the right: the column it starts at. On the left: the column it ends before.
    pub column: usize,
    /// The row it starts level with. It slides up from there if it would
    /// hang below the grid.
    pub row: usize,
}

/// One word or phrase in the list.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct WordEntry {
    /// As it is written: "cart horse".
    pub text: String,
    /// A phrase's split, e.g. "4,5". `None` for a single word.
    pub split: Option<String>,
}

/// The word-count chips, in order.
pub const WORD_COUNTS: [&str; 4] = ["Any", "1 word", "2 words", "3+ words"];

#[component]
pub fn WordPopover(
    /// The answer's name: "3 down".
    #[props(into)]
    label: String,
    /// Its letters so far, e.g. "·A·· ·E·T·".
    #[props(into)]
    pattern: String,
    at: Placement,
    /// What is in the search box.
    query: String,
    /// The chosen chip, as an index into `WORD_COUNTS`.
    words: usize,
    /// The words that fit; `None` while they are being found.
    entries: Option<Vec<WordEntry>>,
    /// The icon buttons can't be used just now.
    #[props(default)]
    busy: bool,
    on_query: EventHandler<String>,
    on_words: EventHandler<usize>,
    /// A word was picked; this is its text.
    on_place: EventHandler<String>,
    on_fill: EventHandler<()>,
    on_clear: EventHandler<()>,
    on_close: EventHandler<()>,
) -> Element {
    // The entry the arrow keys are on.
    let mut active = use_signal(|| 0usize);
    let count = entries.as_ref().map_or(0, Vec::len);
    let chosen = active().min(count.saturating_sub(1));
    let under_keys = entries.as_ref().and_then(|list| list.get(chosen)).cloned();
    // Keep the entry under the arrow keys in view.
    use_effect(move || {
        active();
        document::eval(
            "document.querySelector('.word-popover__item[aria-selected=\"true\"]')?.scrollIntoView({ block: 'nearest' })",
        );
    });
    let dialog = rsx! {
        div {
            class: "word-popover",
            role: "dialog",
            "aria-label": "Words for {label}",
            // Clicks in here are not clicks elsewhere.
            onclick: move |e| e.stop_propagation(),
            oncontextmenu: move |e| e.stop_propagation(),
            onkeydown: move |e| {
                if e.key() == Key::Escape {
                    e.stop_propagation();
                    on_close.call(());
                }
            },
            header { class: "word-popover__head",
                b { class: "word-popover__label", "{label}" }
                Button {
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::IconSm,
                    disabled: busy,
                    title: "Fill just this answer",
                    aria_label: "Fill just this answer",
                    onclick: move |_| on_fill.call(()),
                    Icon { name: IconName::Wand }
                }
                Button {
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::IconSm,
                    disabled: busy,
                    title: "Clear this answer",
                    aria_label: "Clear this answer",
                    onclick: move |_| on_clear.call(()),
                    Icon { name: IconName::Eraser }
                }
            }
            input {
                class: "word-popover__search",
                value: "{query}",
                placeholder: "Search {pattern}",
                spellcheck: "false",
                autocomplete: "off",
                "aria-label": "Search the words for {label}",
                onmounted: move |e| async move {
                    let _ = e.set_focus(true).await;
                },
                oninput: move |e| {
                    active.set(0);
                    on_query.call(e.value());
                },
                onkeydown: move |e| match e.key() {
                    Key::ArrowDown => {
                        e.prevent_default();
                        active.set((chosen + 1).min(count.saturating_sub(1)));
                    }
                    Key::ArrowUp => {
                        e.prevent_default();
                        active.set(chosen.saturating_sub(1));
                    }
                    Key::Enter => {
                        if let Some(entry) = &under_keys {
                            e.prevent_default();
                            on_place.call(entry.text.clone());
                        }
                    }
                    _ => {}
                },
            }
            Segmented {
                label: "Words",
                pills: true,
                options: WORD_COUNTS.map(str::to_string).to_vec(),
                selected: words,
                on_change: move |i| {
                    active.set(0);
                    on_words.call(i);
                },
            }
            div { class: "word-popover__list", role: "listbox", "aria-label": "Words that fit",
                {
                    match entries {
                        None => rsx! {
                            p { class: "word-popover__note is-pending", "Finding words…" }
                        },
                        Some(list) if list.is_empty() => rsx! {
                            p { class: "word-popover__note", "Nothing fits {pattern}" }
                        },
                        Some(list) => rsx! {
                            for (i , entry) in list.into_iter().enumerate() {
                                button {
                                    key: "{entry.text}",
                                    class: "word-popover__item",
                                    r#type: "button",
                                    role: "option",
                                    "aria-selected": "{i == chosen}",
                                    onclick: {
                                        let text = entry.text.clone();
                                        move |_| on_place.call(text.clone())
                                    },
                                    "{entry.text}"
                                    if let Some(split) = entry.split {
                                        small { class: "word-popover__split", "({split})" }
                                    }
                                }
                            }
                        },
                    }
                }
            }
        }
    };
    // The anchor puts the popover beside the answer, level with its row, and
    // lets it slide up when it would hang below the grid.
    rsx! {
        div {
            class: "word-popover-anchor",
            "data-side": if at.side == Side::Left { "left" } else { "right" },
            style: "--at-column: {at.column}; --at-row: {at.row}",
            {dialog}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::test_render as render;

    fn entry(text: &str, split: Option<&str>) -> WordEntry {
        WordEntry {
            text: text.into(),
            split: split.map(str::to_string),
        }
    }

    fn popover(entries: Option<Vec<WordEntry>>, at: Placement) -> String {
        render(move || {
            rsx! {
                WordPopover {
                    label: "3 down",
                    pattern: "·A·· ·E·T·",
                    at,
                    query: "ca".to_string(),
                    words: 2,
                    entries,
                    on_query: |_| {},
                    on_words: |_| {},
                    on_place: |_| {},
                    on_fill: |_| {},
                    on_clear: |_| {},
                    on_close: |_| {},
                }
            }
        })
    }

    fn right() -> Placement {
        Placement {
            side: Side::Right,
            column: 9,
            row: 2,
        }
    }

    #[test]
    fn header_has_the_answer_and_two_named_icon_buttons() {
        let html = popover(Some(vec![]), right());
        assert!(html.contains(r#"class="word-popover""#), "{html}");
        assert!(html.contains(">3 down<"), "{html}");
        for name in ["Fill just this answer", "Clear this answer"] {
            assert!(html.contains(&format!(r#"aria-label="{name}""#)), "{html}");
            assert!(html.contains(&format!(r#"title="{name}""#)), "{html}");
            // The name is only on the button, never spelt out beside it.
            assert!(!html.contains(&format!(">{name}<")), "{html}");
        }
        assert_eq!(html.matches("dx-button").count(), 2, "{html}");
        assert!(html.contains(r#"data-icon="wand""#), "{html}");
        assert!(html.contains(r#"data-icon="eraser""#), "{html}");
    }

    #[test]
    fn search_box_and_word_count_chips() {
        let html = popover(Some(vec![]), right());
        assert!(html.contains("word-popover__search"), "{html}");
        assert!(html.contains(r#"value="ca""#), "{html}");
        assert!(
            html.contains(r#"placeholder="Search ·A·· ·E·T·""#),
            "{html}"
        );
        assert!(html.contains(r#"data-variant="pills""#), "{html}");
        let mut from = 0;
        for chip in WORD_COUNTS {
            let at = html[from..]
                .find(&format!(">{chip}<"))
                .unwrap_or_else(|| panic!("{chip} missing or out of order: {html}"));
            from += at;
        }
        assert_eq!(html.matches(r#"data-state="on""#).count(), 1, "{html}");
        let on = &html[html.find(r#"data-state="on""#).unwrap()..];
        let chip = &on[..on.find("</button>").unwrap()];
        assert!(chip.ends_with(">2 words"), "{chip}");
    }

    #[test]
    fn lists_words_with_a_phrases_split_and_no_notes() {
        let html = popover(
            Some(vec![
                entry("cart horse", Some("4,5")),
                entry("orchestra", None),
            ]),
            right(),
        );
        assert_eq!(html.matches("word-popover__item").count(), 2, "{html}");
        assert!(
            html.contains(r#"cart horse<small class="word-popover__split">(4,5)</small>"#),
            "{html}"
        );
        assert!(html.contains(">orchestra</button>"), "{html}");
        assert_eq!(html.matches("<small").count(), 1, "{html}");
        assert_eq!(html.matches(r#"aria-selected="true""#).count(), 1, "{html}");
        assert!(!html.contains("word-popover__note"), "{html}");
    }

    #[test]
    fn says_when_nothing_fits_or_words_are_on_their_way() {
        let none = popover(Some(vec![]), right());
        assert!(none.contains("Nothing fits ·A·· ·E·T·"), "{none}");
        assert!(!none.contains("is-pending"), "{none}");
        let pending = popover(None, right());
        assert!(pending.contains("Finding words…"), "{pending}");
        assert!(pending.contains("is-pending"), "{pending}");
        assert!(!pending.contains("word-popover__item"), "{pending}");
    }

    #[test]
    fn is_placed_by_column_row_and_side() {
        let html = popover(Some(vec![]), right());
        // An anchor carries the place; the popover itself sits inside it.
        let anchor = html
            .find(r#"class="word-popover-anchor""#)
            .expect("an anchor");
        let dialog = html.find(r#"class="word-popover""#).expect("the popover");
        assert!(anchor < dialog, "{html}");
        let tag = &html[anchor..anchor + html[anchor..].find('>').unwrap()];
        assert!(tag.contains(r#"data-side="right""#), "{tag}");
        assert!(tag.contains("--at-column: 9; --at-row: 2"), "{tag}");
        let flipped = popover(
            Some(vec![]),
            Placement {
                side: Side::Left,
                column: 6,
                row: 14,
            },
        );
        assert!(flipped.contains(r#"data-side="left""#), "{flipped}");
        assert!(
            flipped.contains("--at-column: 6; --at-row: 14"),
            "{flipped}"
        );
    }
}
