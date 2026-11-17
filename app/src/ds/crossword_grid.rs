//! The crossword grid: squares to click and type in, the selected answer lit
//! up, and word breaks drawn between squares. Its colours are the same in
//! light and dark (the grid tokens), like a printed puzzle.

use crate::grid_edit::Break;
use dioxus::prelude::*;

/// How a square is lit.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Lit {
    #[default]
    Off,
    /// Part of the selected answer.
    Answer,
    /// The selected square.
    Current,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct GridCell {
    pub block: bool,
    /// A capital letter, if the square is filled.
    pub letter: Option<char>,
    /// The clue number, where an answer starts.
    pub number: Option<u32>,
    pub lit: Lit,
    /// A word break after this square, along its across answer.
    pub right: Option<Break>,
    /// A word break after this square, along its down answer.
    pub below: Option<Break>,
}

/// What a key press in the grid means.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GridKey {
    /// Put this capital letter in the square.
    Letter(char),
    Backspace,
    /// Move by (rows, columns).
    Move(i32, i32),
    /// Switch between across and down.
    Turn,
    NextAnswer,
    PreviousAnswer,
    Escape,
}

/// The grid's meaning for a key, named as the browser names it ("a",
/// "ArrowLeft", " "). `None` for keys the grid leaves alone.
pub fn grid_key(key: &str, shift: bool) -> Option<GridKey> {
    let mut chars = key.chars();
    if let (Some(ch), None) = (chars.next(), chars.next()) {
        if ch.is_ascii_alphabetic() {
            return Some(GridKey::Letter(ch.to_ascii_uppercase()));
        }
    }
    Some(match key {
        "Backspace" => GridKey::Backspace,
        "ArrowLeft" => GridKey::Move(0, -1),
        "ArrowRight" => GridKey::Move(0, 1),
        "ArrowUp" => GridKey::Move(-1, 0),
        "ArrowDown" => GridKey::Move(1, 0),
        " " => GridKey::Turn,
        "Tab" if shift => GridKey::PreviousAnswer,
        "Tab" => GridKey::NextAnswer,
        "Escape" => GridKey::Escape,
        _ => return None,
    })
}

fn break_name(b: Option<Break>) -> Option<&'static str> {
    b.map(|b| match b {
        Break::Word => "word",
        Break::Hyphen => "hyphen",
    })
}

#[component]
pub fn CrosswordGrid(
    /// Every square, row by row.
    cells: Vec<GridCell>,
    columns: usize,
    #[props(default = "Grid".to_string(), into)] label: String,
    /// A square was clicked.
    on_select: EventHandler<usize>,
    /// A square was right-clicked: its words are wanted.
    on_words: EventHandler<usize>,
    on_key: EventHandler<GridKey>,
) -> Element {
    rsx! {
        div {
            class: "xw-grid",
            role: "group",
            tabindex: "0",
            "aria-label": "{label}",
            style: "--columns: {columns}",
            onkeydown: move |e| {
                let held = e.modifiers();
                if held.intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::META) {
                    return;
                }
                if let Some(key) = grid_key(&e.key().to_string(), held.contains(Modifiers::SHIFT)) {
                    e.prevent_default();
                    on_key.call(key);
                }
            },
            for (i, cell) in cells.into_iter().enumerate() {
                if cell.block {
                    div { key: "{i}", class: "xw-grid__cell is-block" }
                } else {
                    div {
                        key: "{i}",
                        class: match cell.lit {
                            Lit::Off => "xw-grid__cell",
                            Lit::Answer => "xw-grid__cell is-answer",
                            Lit::Current => "xw-grid__cell is-current",
                        },
                        "data-cell": "{i}",
                        "data-break-right": break_name(cell.right),
                        "data-break-below": break_name(cell.below),
                        onclick: move |_| on_select.call(i),
                        oncontextmenu: move |e| {
                            e.prevent_default();
                            on_words.call(i);
                        },
                        if let Some(n) = cell.number {
                            span { class: "xw-grid__number", "{n}" }
                        }
                        if let Some(letter) = cell.letter {
                            "{letter}"
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
    fn keys_mean_letters_moves_turns_and_answers() {
        assert_eq!(grid_key("a", false), Some(GridKey::Letter('A')));
        assert_eq!(grid_key("Q", true), Some(GridKey::Letter('Q')));
        assert_eq!(grid_key("Backspace", false), Some(GridKey::Backspace));
        assert_eq!(grid_key("ArrowLeft", false), Some(GridKey::Move(0, -1)));
        assert_eq!(grid_key("ArrowRight", false), Some(GridKey::Move(0, 1)));
        assert_eq!(grid_key("ArrowUp", false), Some(GridKey::Move(-1, 0)));
        assert_eq!(grid_key("ArrowDown", false), Some(GridKey::Move(1, 0)));
        assert_eq!(grid_key(" ", false), Some(GridKey::Turn));
        assert_eq!(grid_key("Tab", false), Some(GridKey::NextAnswer));
        assert_eq!(grid_key("Tab", true), Some(GridKey::PreviousAnswer));
        assert_eq!(grid_key("Escape", false), Some(GridKey::Escape));
        for other in ["1", "é", "Enter", "F5", "Shift", ""] {
            assert_eq!(grid_key(other, false), None, "{other}");
        }
    }

    fn cells() -> Vec<GridCell> {
        vec![
            GridCell {
                letter: Some('C'),
                number: Some(1),
                lit: Lit::Answer,
                right: Some(Break::Word),
                ..Default::default()
            },
            GridCell {
                lit: Lit::Current,
                below: Some(Break::Hyphen),
                ..Default::default()
            },
            GridCell {
                block: true,
                ..Default::default()
            },
            GridCell {
                letter: Some('T'),
                number: Some(12),
                ..Default::default()
            },
        ]
    }

    #[test]
    fn draws_squares_blocks_numbers_letters_and_breaks() {
        let html = render(move || {
            rsx! {
                CrosswordGrid { cells: cells(), columns: 2, on_select: |_| {}, on_words: |_| {}, on_key: |_| {} }
            }
        });
        assert!(html.contains(r#"class="xw-grid""#), "{html}");
        assert!(html.contains("--columns: 2"), "{html}");
        assert!(html.contains(r#"tabindex="0""#), "{html}");
        assert_eq!(html.matches("xw-grid__cell").count(), 4, "{html}");
        assert_eq!(html.matches("is-block").count(), 1, "{html}");
        assert_eq!(html.matches("is-answer").count(), 1, "{html}");
        assert_eq!(html.matches("is-current").count(), 1, "{html}");
        assert!(
            html.contains(r#"<span class="xw-grid__number">1</span>C"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<span class="xw-grid__number">12</span>T"#),
            "{html}"
        );
        assert_eq!(
            html.matches(r#"data-break-right="word""#).count(),
            1,
            "{html}"
        );
        assert_eq!(
            html.matches(r#"data-break-below="hyphen""#).count(),
            1,
            "{html}"
        );
        // Blocks can't be chosen.
        assert_eq!(html.matches("data-cell=").count(), 3, "{html}");
    }
}
