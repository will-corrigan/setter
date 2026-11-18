//! Choosing a grid to start from: an overlay of thumbnails, one per
//! standard grid, a size at a time.

use crate::ds::segmented::Segmented;
use dioxus::prelude::*;

/// A grid to choose: its name and its rows ('#' is a block).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GridThumb {
    pub name: String,
    pub rows: Vec<String>,
}

#[component]
pub fn TemplatePicker(
    #[props(into)] title: String,
    grids: Vec<GridThumb>,
    /// A grid was chosen; this is its place in `grids`.
    on_pick: EventHandler<usize>,
    on_close: EventHandler<()>,
    /// The size to show first; the biggest if not given.
    #[props(default)]
    size: Option<usize>,
) -> Element {
    // The sizes on offer, smallest first; the biggest is shown to start with.
    let mut sizes: Vec<usize> = grids.iter().map(|g| g.rows.len()).collect();
    sizes.sort_unstable();
    sizes.dedup();
    let first = size.filter(|n| sizes.contains(n));
    let mut size = use_signal(|| first.or(sizes.last().copied()).unwrap_or(0));
    let shown: Vec<(usize, GridThumb)> = grids
        .into_iter()
        .enumerate()
        .filter(|(_, g)| g.rows.len() == size())
        .collect();
    let first = shown.first().map(|(i, _)| *i);
    let labels: Vec<String> = sizes.iter().map(|n| format!("{n}×{n}")).collect();
    let at = sizes.iter().position(|n| *n == size()).unwrap_or(0);
    rsx! {
        div {
            class: "template-picker",
            onclick: move |_| on_close.call(()),
            onkeydown: move |e| {
                if e.key() == Key::Escape {
                    on_close.call(());
                }
            },
            div {
                class: "template-picker__card",
                role: "dialog",
                "aria-modal": "true",
                "aria-label": "{title}",
                onclick: move |e| e.stop_propagation(),
                div { class: "template-picker__head",
                    h3 { class: "template-picker__title", "{title}" }
                    if sizes.len() > 1 {
                        Segmented {
                            label: "Grid size",
                            pills: true,
                            options: labels,
                            selected: at,
                            on_change: move |j: usize| {
                                if let Some(n) = sizes.get(j) {
                                    size.set(*n);
                                }
                            },
                        }
                    }
                }
                div { class: "template-picker__grids",
                    for (i , grid) in shown {
                        button {
                            key: "{grid.name}",
                            class: "template-picker__thumb",
                            r#type: "button",
                            onclick: move |_| on_pick.call(i),
                            // Start on the first grid, so the keyboard works at once.
                            onmounted: move |e| async move {
                                if Some(i) == first {
                                    let _ = e.set_focus(true).await;
                                }
                            },
                            div {
                                class: "template-picker__mini",
                                "aria-hidden": "true",
                                style: "--columns: {grid.rows.first().map_or(0, |r| r.chars().count())}",
                                for (j , ch) in grid.rows.concat().chars().enumerate() {
                                    span { key: "{j}", class: if ch == '#' { "is-block" } }
                                }
                            }
                            span { class: "template-picker__name", "{grid.name}" }
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
    fn shows_a_thumbnail_and_name_for_each_grid() {
        let grids = vec![
            GridThumb {
                name: "British 15 · A".into(),
                rows: vec!["..#".into(), "...".into(), "#..".into()],
            },
            GridThumb {
                name: "British 13 · A".into(),
                rows: vec!["..".into(), ".#".into()],
            },
        ];
        let html = render(move || {
            rsx! {
                TemplatePicker { title: "New puzzle", grids, on_pick: |_| {}, on_close: |_| {} }
            }
        });
        assert!(html.contains(r#"class="template-picker""#), "{html}");
        assert!(html.contains(r#"role="dialog""#), "{html}");
        assert!(html.contains(r#"aria-label="New puzzle""#), "{html}");
        assert!(html.contains(">New puzzle<"), "{html}");
        // A size at a time, the biggest first; a pill for each size.
        assert_eq!(html.matches("template-picker__thumb").count(), 1, "{html}");
        assert!(html.contains(">British 15 · A<") && !html.contains(">British 13 · A<"));
        assert!(html.contains("--columns: 3"), "{html}");
        assert!(html.contains(">2×2<") && html.contains(">3×3<"), "{html}");
        assert!(html.contains(r#"aria-label="Grid size""#), "{html}");
        // 9 squares, 2 of them blocks.
        assert_eq!(html.matches("is-block").count(), 2, "{html}");
    }
}
