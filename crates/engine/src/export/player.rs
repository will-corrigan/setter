//! A self-contained web page for solving the puzzle in any browser: Setter's
//! own player (assets/player), with check and reveal, a timer, an anagram
//! helper and a celebration when it's solved. No network needed.

use crate::fill::shape_for;
use crate::grid::Dir;
use crate::puzzle::Puzzle;
use serde_json::{json, Value};

static TEMPLATE: &str = include_str!("../../assets/player/player.html");
static CSS: &str = include_str!("../../assets/player/player.css");
static JS: &str = include_str!("../../assets/player/player.js");
use crate::fonts;

/// The fonts the page uses, inlined: the sans and the upright serif.
fn player_faces() -> String {
    fonts::face(fonts::SANS_FAMILY, &fonts::SANS) + &fonts::face(fonts::SERIF_FAMILY, &fonts::SERIF)
}

/// The puzzle as the player's JSON: grid, numbers, word breaks and clues.
pub fn data(p: &Puzzle) -> Value {
    let g = &p.grid;
    let has_solution = g.cells.iter().all(|c| c.block || c.letter.is_some());
    let cells: Vec<String> = g
        .cells
        .iter()
        .map(|c| {
            if c.block {
                "#".into()
            } else {
                c.letter
                    .map(|l| l.to_ascii_uppercase().to_string())
                    .unwrap_or_default()
            }
        })
        .collect();
    let lights = g.lights();
    let mut numbers = vec![0u32; g.cells.len()];
    let mut breaks: Vec<Vec<&str>> = vec![vec![]; g.cells.len()];
    let mut clues = vec![];
    for dir in [Dir::Across, Dir::Down] {
        for l in lights.iter().filter(|l| l.dir == dir) {
            numbers[l.cells[0]] = l.number;
            let clue = p.clues.get(&l.id).cloned().unwrap_or_default();
            let split = shape_for(&clue.enumeration, l.len());
            let en = split.clone().unwrap_or_else(|| l.len().to_string());
            if let Some(e) = split {
                let mut at = 0;
                let mut num = String::new();
                for ch in e.chars() {
                    if ch.is_ascii_digit() {
                        num.push(ch);
                        continue;
                    }
                    at += num.parse::<usize>().unwrap_or(0);
                    num.clear();
                    let side = if dir == Dir::Across { "r" } else { "b" };
                    breaks[l.cells[at - 1]].push(if ch == '-' {
                        if side == "r" {
                            "hy-r"
                        } else {
                            "hy-b"
                        }
                    } else if side == "r" {
                        "bar-r"
                    } else {
                        "bar-b"
                    });
                }
            }
            clues.push(json!({
                "n": l.number,
                "dir": if dir == Dir::Across { "a" } else { "d" },
                "cells": l.cells,
                "text": clue.text.split_whitespace().collect::<Vec<_>>().join(" "),
                "en": en,
                "anno": clue.annotation.split_whitespace().collect::<Vec<_>>().join(" "),
            }));
        }
    }
    // The id keys the solver's saved progress, so it covers everything that
    // makes this a different puzzle: title, shape, blocks, letters and clues.
    // Each field ends with a separator byte so adjacent fields can't run
    // together ("AB"+"" vs "A"+"B").
    let mut h: u64 = 0xcbf29ce484222325;
    let mut feed = |s: &str| {
        for b in s.bytes().chain([0x1f]) {
            h = (h ^ b as u64).wrapping_mul(0x100000001b3);
        }
    };
    feed(&p.title);
    feed(&g.width.to_string());
    feed(&g.height.to_string());
    for c in &cells {
        feed(c);
    }
    for c in &clues {
        for f in ["dir", "n", "text", "en"] {
            feed(&c[f].to_string());
        }
    }
    json!({
        "id": format!("{h:016x}"),
        "title": p.title,
        "author": p.author,
        "w": g.width,
        "h": g.height,
        "cells": cells,
        "hasSolution": has_solution,
        "numbers": numbers,
        "breaks": breaks.iter().map(|b| b.join(" ")).collect::<Vec<_>>(),
        "clues": clues,
    })
}

fn html_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// One self-contained page that solves the puzzle in any browser, offline.
pub fn html(p: &Puzzle) -> String {
    let title = if p.title.trim().is_empty() {
        "Crossword".to_string()
    } else {
        p.title.trim().to_string()
    };
    let by = if p.author.trim().is_empty() {
        String::new()
    } else {
        format!(
            "<span class=\"by\">by {}</span>",
            html_text(p.author.trim())
        )
    };
    let notes = if p.notes.trim().is_empty() {
        String::new()
    } else {
        format!("{} ", html_text(p.notes.trim()))
    };
    // JSON inside <script>: no "<", ">" or "&" at all, so neither "</script>"
    // nor "<!--<script>" can end or derail the block. They only occur inside
    // JSON strings, where the \u escapes decode back to the same text.
    let json = data(p)
        .to_string()
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026");
    fill(
        TEMPLATE,
        &[
            ("TITLE", &html_text(&title)),
            ("BY", &by),
            ("NOTES", &notes),
            ("FONTS", &player_faces()),
            ("CSS", CSS),
            ("JS", JS),
            ("DATA", &json),
        ],
    )
}

/// Replaces each `{{NAME}}` in the template with its value, in one pass.
/// Inserted text is never rescanned, so a title or clue that happens to say
/// `{{DATA}}` stays as written. Unknown names are left alone.
fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let value = after.find("}}").and_then(|close| {
            let name = &after[..close];
            values
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| (*v, close))
        });
        match value {
            Some((v, close)) => {
                out.push_str(v);
                rest = &after[close + 2..];
            }
            None => {
                out.push_str("{{");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::Grid;
    use crate::puzzle::{Clue, Puzzle};

    fn sample() -> Puzzle {
        let mut p = Puzzle::new(3, 3);
        p.title = "Tiny <test>".into();
        p.author = "Setter".into();
        p.grid = Grid::from_rows(&["cat", "a#o", "bye"]);
        p.clues.insert(
            "0,0,a".into(),
            Clue {
                text: "Pet </script> feline".into(),
                enumeration: "".into(),
                annotation: "C+AT".into(),
            },
        );
        p.clues.insert(
            "0,0,d".into(),
            Clue {
                text: "Taxi".into(),
                enumeration: "1-2".into(),
                annotation: "".into(),
            },
        );
        p
    }

    #[test]
    fn data_describes_grid_and_clues() {
        let d = data(&sample());
        assert_eq!((d["w"].as_u64(), d["h"].as_u64()), (Some(3), Some(3)));
        assert_eq!(d["cells"][0], "C");
        assert_eq!(d["cells"][4], "#");
        assert_eq!(d["hasSolution"], true);
        assert_eq!(d["numbers"][0], 1);
        assert_eq!(d["numbers"][2], 2);
        let clues = d["clues"].as_array().unwrap();
        assert_eq!(clues.len(), 4);
        assert_eq!(clues[0]["n"], 1);
        assert_eq!(clues[0]["dir"], "a");
        assert_eq!(clues[0]["cells"], serde_json::json!([0, 1, 2]));
        assert_eq!(clues[0]["en"], "3");
        assert_eq!(clues[0]["anno"], "C+AT");
        assert_eq!(clues[2]["dir"], "d", "acrosses first, then downs");
        assert_eq!(
            d["breaks"][0], "hy-b",
            "1-2 down puts a hyphen under the first square"
        );
    }

    #[test]
    fn unfinished_grid_has_no_solution() {
        let mut p = sample();
        p.grid.cells[8].letter = None;
        let d = data(&p);
        assert_eq!(d["hasSolution"], false);
        assert_eq!(d["cells"][8], "");
    }

    #[test]
    fn html_is_self_contained_and_safe() {
        let h = html(&sample());
        assert!(h.starts_with("<!DOCTYPE html>"));
        assert!(
            !h.contains("<script src=") && !h.contains("<link "),
            "nothing loaded from the network"
        );
        assert!(h.contains("data:font/woff2;base64,"), "fonts embedded");
        assert!(
            !h.contains("Pet </script>"),
            "script end tags in clues are escaped"
        );
        assert!(h.contains("<title>Tiny &lt;test&gt;</title>"));
        assert!(!h.contains("{{"), "every placeholder filled");
        assert!(h.contains("setter-data") && h.contains("window.setterPlayer"));
    }

    /// The JSON in the page's data block, parsed back.
    fn embedded_data(h: &str) -> Value {
        let open = "<script type=\"application/json\" id=\"setter-data\">";
        let start = h.find(open).expect("data block") + open.len();
        let end = start + h[start..].find("</script>").expect("data block ends");
        serde_json::from_str(&h[start..end]).expect("data block is valid JSON")
    }

    #[test]
    fn user_text_cannot_inject_placeholders_or_markup() {
        let mut p = sample();
        p.title = "{{JS}} <!--<script>".into();
        p.author = "{{DATA}} </script>".into();
        p.notes = "{{CSS}} {{NOTES}}".into();
        let evil = "{{DATA}} </script><img src=x onerror=alert(1)> <!--<script> {{JS}} & >";
        p.clues.get_mut("0,0,a").unwrap().text = evil.into();
        let h = html(&p);
        let d = embedded_data(&h);
        assert_eq!(d["title"], "{{JS}} <!--<script>");
        assert_eq!(d["author"], "{{DATA}} </script>");
        assert_eq!(d["clues"][0]["text"], evil);
        assert_eq!(d, data(&p), "data block round-trips exactly");

        // User text lands as text, never expanded: the player code, styles,
        // data and fonts each appear exactly once.
        assert_eq!(h.matches("window.setterPlayer").count(), 1, "JS once");
        assert_eq!(h.matches(".keys {").count(), 1, "CSS once");
        assert_eq!(h.matches("id=\"setter-data\"").count(), 1, "data once");
        assert_eq!(
            h.matches(&fonts::base64(fonts::SANS.woff2)).count(),
            1,
            "font once"
        );
        assert!(h.contains("<title>{{JS}} &lt;!--&lt;script&gt;</title>"));
        assert!(h.contains("<h1>{{JS}} &lt;!--&lt;script&gt;</h1>"));
        assert!(h.contains("by {{DATA}} &lt;/script&gt;</span>"));
        assert!(h.contains("{{CSS}} {{NOTES}} Made with Setter."));
        // Only the two scripts the template has: the data block and the player.
        assert_eq!(h.matches("<script").count(), 2);
        assert!(!h.contains("<img"), "no markup from clues reaches the page");
        assert!(!h.contains("<!--<"), "no comment-script trick in the data");
    }

    #[test]
    fn data_block_escapes_angle_brackets_and_ampersands() {
        let mut p = sample();
        p.clues.get_mut("0,0,a").unwrap().text = "a<b>c&d".into();
        let h = html(&p);
        let open = "id=\"setter-data\">";
        let start = h.find(open).unwrap() + open.len();
        let block = &h[start..start + h[start..].find("</script>").unwrap()];
        assert!(!block.contains('<') && !block.contains('>') && !block.contains('&'));
        assert!(block.contains("a\\u003cb\\u003ec\\u0026d"));
    }

    #[test]
    fn fill_replaces_each_placeholder_once_without_rescanning() {
        let out = fill(
            "<{{A}}|{{B}}|{{A}}|{{UNKNOWN}}|{{",
            &[("A", "{{B}}"), ("B", "x")],
        );
        assert_eq!(out, "<{{B}}|x|{{B}}|{{UNKNOWN}}|{{");
    }

    #[test]
    fn progress_id_depends_on_shape_blocks_and_clues() {
        let a = sample();
        let id = |p: &Puzzle| data(p)["id"].as_str().unwrap().to_string();
        assert_eq!(id(&a), id(&sample()), "deterministic");

        let mut clue = sample();
        clue.clues.get_mut("0,0,d").unwrap().text = "Cab".into();
        assert_ne!(id(&a), id(&clue), "a different clue is a different puzzle");

        let mut en = sample();
        en.clues.get_mut("0,0,d").unwrap().enumeration = "".into();
        assert_ne!(id(&a), id(&en), "a different enumeration too");

        // Same title, no letters, different shape or blocks.
        let blank = |w, h, blocks: &[usize]| {
            let mut p = Puzzle::new(w, h);
            p.title = "Same".into();
            for &i in blocks {
                p.grid.cells[i].block = true;
            }
            p
        };
        assert_ne!(id(&blank(3, 3, &[])), id(&blank(3, 3, &[4])));
        assert_ne!(id(&blank(2, 3, &[])), id(&blank(3, 2, &[])));
    }
}
