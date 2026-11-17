//! Choosing a grid to start from: an overlay of thumbnails, one per
//! standard grid.

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
) -> Element {
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
                h3 { class: "template-picker__title", "{title}" }
                div { class: "template-picker__grids",
                    for (i , grid) in grids.into_iter().enumerate() {
                        button {
                            key: "{grid.name}",
                            class: "template-picker__thumb",
                            r#type: "button",
                            onclick: move |_| on_pick.call(i),
                            // Start on the first grid, so the keyboard works at once.
                            onmounted: move |e| async move {
                                if i == 0 {
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
        assert_eq!(html.matches("template-picker__thumb").count(), 2, "{html}");
        assert!(html.contains(">British 15 · A<") && html.contains(">British 13 · A<"));
        assert!(
            html.contains("--columns: 3") && html.contains("--columns: 2"),
            "{html}"
        );
        // 9 + 4 squares, 2 + 1 of them blocks.
        assert_eq!(html.matches("<span").count() - 2, 13, "{html}");
        assert_eq!(html.matches("is-block").count(), 3, "{html}");
    }
}
