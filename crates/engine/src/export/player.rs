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
static FONT_SERIF: &[u8] =
    include_bytes!("../../../../app/assets/fonts/newsreader-latin-opsz-normal.woff2");
static FONT_SANS: &[u8] =
    include_bytes!("../../../../app/assets/fonts/instrument-sans-latin-wght-normal.woff2");

pub(crate) fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16
            | (*chunk.get(1).unwrap_or(&0) as u32) << 8
            | *chunk.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
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
    let mut h: u64 = 0xcbf29ce484222325;
    for b in p.title.bytes().chain(cells.concat().bytes()) {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
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
    // JSON inside <script>: "</" can't appear, or the script would end early.
    let json = data(p).to_string().replace("</", "<\\/");
    TEMPLATE
        .replace("{{TITLE}}", &html_text(&title))
        .replace("{{BY}}", &by)
        .replace("{{NOTES}}", &notes)
        .replace("{{FONT_SERIF}}", &base64(FONT_SERIF))
        .replace("{{FONT_SANS}}", &base64(FONT_SANS))
        .replace("{{CSS}}", CSS)
        .replace("{{JS}}", JS)
        .replace("{{DATA}}", &json)
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

    #[test]
    fn base64_matches_known_values() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }
}
