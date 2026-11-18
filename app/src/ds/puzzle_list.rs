//! Home's pieces: the tiles for starting a puzzle of each size, and the list
//! of puzzles with their progress.

use crate::ds::menu_button::{MenuButton, MenuItem};
use crate::ds::mini_grid::{MiniGrid, MiniSize};
use dioxus::prelude::*;

/// A size of grid to start from: its side and a sample grid's blocks.
#[derive(Clone, PartialEq, Debug)]
pub struct SizeChoice {
    pub side: usize,
    pub blocks: Vec<bool>,
}

#[component]
pub fn SizeTiles(sizes: Vec<SizeChoice>, on_pick: EventHandler<usize>) -> Element {
    rsx! {
        div { class: "size-tiles", role: "list",
            for (i , s) in sizes.into_iter().enumerate() {
                button {
                    key: "{s.side}",
                    class: "size-tiles__tile",
                    style: "--tile-side: {s.side}",
                    r#type: "button",
                    role: "listitem",
                    onclick: move |_| on_pick.call(i),
                    MiniGrid { columns: s.side, blocks: s.blocks.clone(), size: MiniSize::Small }
                    span { class: "size-tiles__label", "{s.side}×{s.side}" }
                }
            }
        }
    }
}

/// One puzzle's row.
#[derive(Clone, PartialEq, Debug)]
pub struct PuzzleRow {
    pub title: String,
    /// "Quixote · 15×15"
    pub detail: String,
    pub columns: usize,
    pub blocks: Vec<bool>,
    pub filled: (usize, usize),
    pub clued: (usize, usize),
    /// "2 minutes ago"
    pub edited: String,
}

/// What a row's "⋯" menu offers, in order.
pub const ROW_ACTIONS: [&str; 3] = ["Open", "Duplicate", "Show in folder"];

fn percent((done, all): (usize, usize)) -> usize {
    (done * 100).checked_div(all).unwrap_or(0)
}

#[component]
pub fn PuzzleList(
    rows: Vec<PuzzleRow>,
    on_open: EventHandler<usize>,
    /// (row, position in [`ROW_ACTIONS`])
    on_action: EventHandler<(usize, usize)>,
) -> Element {
    rsx! {
        div { class: "puzzle-list", role: "list",
            for (i , row) in rows.into_iter().enumerate() {
                div { key: "{i}-{row.title}", class: "puzzle-list__row", role: "listitem",
                    button {
                        class: "puzzle-list__open",
                        r#type: "button",
                        "aria-label": "Open {row.title}",
                        onclick: move |_| on_open.call(i),
                        MiniGrid { columns: row.columns, blocks: row.blocks.clone() }
                        span { class: "puzzle-list__name",
                            b { "{row.title}" }
                            span { "{row.detail}" }
                        }
                        span { class: "puzzle-list__progress",
                            "{row.filled.0} of {row.filled.1} filled"
                            span { class: "puzzle-list__bar",
                                i { style: "width: {percent(row.filled)}%" }
                            }
                        }
                        span { class: "puzzle-list__progress puzzle-list__progress--clued",
                            "{row.clued.0} of {row.clued.1} clued"
                            span { class: "puzzle-list__bar",
                                i { style: "width: {percent(row.clued)}%" }
                            }
                        }
                        span { class: "puzzle-list__edited", "{row.edited}" }
                    }
                    MenuButton {
                        label: "More for {row.title}",
                        icon: rsx! { span { class: "puzzle-list__more", "⋯" } },
                        items: ROW_ACTIONS.iter().map(|a| MenuItem::new(*a)).collect(),
                        on_pick: move |a: usize| on_action.call((i, a)),
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

    fn row(title: &str) -> PuzzleRow {
        PuzzleRow {
            title: title.into(),
            detail: "Quixote · 3×3".into(),
            columns: 3,
            blocks: vec![false, false, false, false, true, false, false, false, false],
            filled: (3, 4),
            clued: (1, 4),
            edited: "2 minutes ago".into(),
        }
    }

    #[test]
    fn rows_show_name_progress_and_when() {
        let html = render(move || {
            rsx! { PuzzleList { rows: vec![row("Prize 12"), row("Mini 7")], on_open: |_| {}, on_action: |_| {} } }
        });
        assert_eq!(html.matches("puzzle-list__row").count(), 2, "{html}");
        for piece in [
            ">Prize 12<",
            ">Quixote · 3×3<",
            "3 of 4 filled",
            "1 of 4 clued",
            ">2 minutes ago<",
            "width: 75%",
            "width: 25%",
        ] {
            assert!(html.contains(piece), "{piece}: {html}");
        }
        assert!(html.contains(r#"aria-label="Open Prize 12""#), "{html}");
        assert!(html.contains(r#"aria-label="More for Mini 7""#), "{html}");
    }

    #[test]
    fn size_tiles_name_each_size() {
        let sizes = vec![
            SizeChoice {
                side: 5,
                blocks: vec![false; 25],
            },
            SizeChoice {
                side: 7,
                blocks: vec![false; 49],
            },
        ];
        let html = render(move || rsx! { SizeTiles { sizes: sizes.clone(), on_pick: |_| {} } });
        assert!(html.contains(">5×5<") && html.contains(">7×7<"), "{html}");
        assert_eq!(html.matches("mini-grid--small").count(), 2, "{html}");
    }

    #[test]
    fn an_empty_puzzle_shows_no_progress_rather_than_dividing_by_zero() {
        assert_eq!(percent((0, 0)), 0);
        assert_eq!(percent((1, 2)), 50);
    }
}
