//! A small, plain picture of a grid: blocks and squares, optionally with
//! one answer lit and its first square marked. Thumbnails on Home and in
//! the template picker; the "where am I" grid on Write clues, where a click
//! on a square picks it.

use dioxus::prelude::*;

/// How large each square is drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MiniSize {
    /// A list thumbnail.
    #[default]
    Tiny,
    /// A tile or picker thumbnail.
    Small,
    /// Large enough to click a square.
    Medium,
}

impl MiniSize {
    fn class(self) -> &'static str {
        match self {
            MiniSize::Tiny => "mini-grid mini-grid--tiny",
            MiniSize::Small => "mini-grid mini-grid--small",
            MiniSize::Medium => "mini-grid mini-grid--medium",
        }
    }
}

#[component]
pub fn MiniGrid(
    columns: usize,
    /// One per square, row by row: true for a block.
    blocks: Vec<bool>,
    #[props(default)] size: MiniSize,
    /// Squares of the answer in view.
    #[props(default)]
    lit: Vec<usize>,
    /// The answer's first square.
    #[props(default)]
    current: Option<usize>,
    /// A square was clicked (only then are squares clickable).
    #[props(default)]
    on_cell: Option<EventHandler<usize>>,
) -> Element {
    let squares: Vec<(usize, &'static str, bool)> = blocks
        .iter()
        .enumerate()
        .map(|(i, &block)| {
            let class = if block {
                "is-block"
            } else if current == Some(i) {
                "is-current"
            } else if lit.contains(&i) {
                "is-lit"
            } else {
                ""
            };
            (i, class, block)
        })
        .collect();
    rsx! {
        div {
            class: size.class(),
            style: "--columns: {columns}",
            "aria-hidden": "true",
            for (i , class , block) in squares {
                span {
                    key: "{i}",
                    class,
                    onclick: move |_| {
                        if let (Some(h), false) = (on_cell, block) {
                            h.call(i);
                        }
                    },
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
    fn draws_blocks_lit_squares_and_the_current_one() {
        let html = render(move || {
            rsx! {
                MiniGrid {
                    columns: 3,
                    blocks: vec![false, false, false, false, true, false, false, false, false],
                    lit: vec![0, 1, 2],
                    current: Some(0),
                }
            }
        });
        assert!(html.contains("--columns: 3"), "{html}");
        assert_eq!(html.matches("<span").count(), 9, "{html}");
        assert_eq!(html.matches("is-block").count(), 1, "{html}");
        assert_eq!(html.matches("is-lit").count(), 2, "{html}");
        assert_eq!(html.matches("is-current").count(), 1, "{html}");
        assert!(html.contains("mini-grid--tiny"), "{html}");
    }
}
