//! The answer as letter tiles, grouped and coloured by wordplay part.

use dioxus::prelude::*;
use xw_engine::bank::{Tile, Tone};

/// A tile that isn't letters (e.g. "← DESSERTS") is shown as a quiet note.
fn is_note(t: &Tile) -> bool {
    !t.letters
        .chars()
        .all(|c| c.is_ascii_alphabetic() || c == '?')
}

#[component]
pub fn Tiles(tiles: Vec<Tile>, #[props(default = 28)] size: u32) -> Element {
    rsx! {
        div { class: "tiles", style: "--tile: {size}px",
            for (i, t) in tiles.iter().enumerate() {
                if is_note(t) {
                    span { key: "{i}", class: "tiles__note", "{t.letters}" }
                } else {
                    div { key: "{i}", class: "tiles__group", "data-tone": t.tone.map(Tone::class),
                        for (j, ch) in t.letters.chars().enumerate() {
                            span {
                                key: "{j}",
                                class: if ch == '?' { "tiles__tile is-blank" } else if t.struck { "tiles__tile is-struck" } else { "tiles__tile" },
                                if ch != '?' { "{ch}" }
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

    #[test]
    fn tiles_render_tones_struck_and_blanks() {
        let tiles = vec![
            Tile {
                letters: "OR".into(),
                tone: Some(Tone::P1),
                struck: false,
            },
            Tile {
                letters: "L".into(),
                tone: None,
                struck: true,
            },
            Tile {
                letters: "?".into(),
                tone: None,
                struck: false,
            },
        ];
        let html = render(move || rsx! { Tiles { tiles, size: 28 } });
        assert_eq!(html.matches("tiles__tile").count(), 4, "{html}");
        assert!(html.contains(r#"data-tone="p1""#), "{html}");
        assert!(
            html.contains("is-struck") && html.contains("is-blank"),
            "{html}"
        );
    }
}
