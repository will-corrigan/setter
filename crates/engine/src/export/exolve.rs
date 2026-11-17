//! Exolve: a self-contained, playable HTML page (https://github.com/viresh-ratnakar/exolve).

use crate::grid::Dir;
use crate::puzzle::Puzzle;

static EXOLVE_JS: &str = include_str!("../../../../data/vendor/exolve/exolve-m.js");
static EXOLVE_CSS: &str = include_str!("../../../../data/vendor/exolve/exolve-m.css");

fn grid_rows(p: &Puzzle) -> Vec<String> {
    let g = &p.grid;
    let any_letter = g.cells.iter().any(|c| !c.block && c.letter.is_some());
    (0..g.height)
        .map(|r| {
            (0..g.width)
                .map(|c| {
                    let cell = &g.cells[g.idx(r, c)];
                    match (cell.block, cell.letter) {
                        (true, _) => '.',
                        (false, Some(l)) => l.to_ascii_uppercase(),
                        (false, None) if any_letter => '?',
                        (false, None) => '0',
                    }
                })
                .collect()
        })
        .collect()
}

/// A stable id so the solver's progress is saved per puzzle in the browser.
fn puzzle_id(p: &Puzzle) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in p.title.bytes().chain(grid_rows(p).concat().bytes()) {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    format!("setter-{h:016x}")
}

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The puzzle in Exolve's plain-text format.
pub fn spec(p: &Puzzle) -> String {
    let mut s = String::from("exolve-begin\n");
    s += &format!("  exolve-id: {}\n", puzzle_id(p));
    if !p.title.trim().is_empty() {
        s += &format!("  exolve-title: {}\n", one_line(&p.title));
    }
    if !p.author.trim().is_empty() {
        s += &format!("  exolve-setter: {}\n", one_line(&p.author));
    }
    if !p.preamble.trim().is_empty() {
        s += "  exolve-preamble:\n";
        for line in p.preamble.lines() {
            s += &format!("    {line}\n");
        }
    }
    s += &format!(
        "  exolve-width: {}\n  exolve-height: {}\n  exolve-grid:\n",
        p.grid.width, p.grid.height
    );
    for row in grid_rows(p) {
        s += &format!("    {row}\n");
    }
    let lights = p.grid.lights();
    for (dir, name) in [(Dir::Across, "across"), (Dir::Down, "down")] {
        s += &format!("  exolve-{name}:\n");
        for l in lights.iter().filter(|l| l.dir == dir) {
            let clue = p.clues.get(&l.id).cloned().unwrap_or_default();
            let enumeration = if clue.enumeration.trim().is_empty() {
                l.len().to_string()
            } else {
                clue.enumeration.trim().to_string()
            };
            let text = one_line(&clue.text);
            let mut line = if text.is_empty() {
                format!("{} ({enumeration})", l.number)
            } else {
                format!("{} {text} ({enumeration})", l.number)
            };
            let note = one_line(&clue.annotation);
            if !note.is_empty() {
                line += &format!(" {note}");
            }
            s += &format!("    {line}\n");
        }
    }
    s + "exolve-end\n"
}

/// Text safe inside a JavaScript template literal inside a <script> element.
fn js_template(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${")
        .replace("</", "<\\/")
}

fn html_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// A single self-contained page anyone can open in a browser to solve the
/// puzzle (check, reveal, timer, progress saved locally, print). Works offline.
pub fn html(p: &Puzzle) -> String {
    let title = if p.title.trim().is_empty() {
        "Crossword".to_string()
    } else {
        one_line(&p.title)
    };
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{}</title>\n\
         <!-- Made with Setter. Player: Exolve (https://github.com/viresh-ratnakar/exolve), MIT License, Copyright (c) 2019 Viresh Ratnakar. -->\n\
         <style>\n{EXOLVE_CSS}\n</style>\n<script>\n{EXOLVE_JS}\n</script>\n</head>\n<body>\n\
         <div id=\"exolve\"></div>\n<script>\ncreateExolve(`\n{}`);\n</script>\n</body>\n</html>\n",
        html_text(&title),
        js_template(&spec(p))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::Grid;
    use crate::puzzle::{Clue, Puzzle};

    fn sample() -> Puzzle {
        let mut p = Puzzle::new(3, 3);
        p.title = "Tiny `test` ${x}".into();
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
            "2,0,a".into(),
            Clue {
                text: "Farewell".into(),
                enumeration: "3".into(),
                annotation: "".into(),
            },
        );
        p
    }

    #[test]
    fn spec_has_grid_clues_and_enumerations() {
        let s = spec(&sample());
        assert!(s.contains("exolve-begin") && s.contains("exolve-end"));
        assert!(s.contains("exolve-width: 3") && s.contains("exolve-height: 3"));
        assert!(
            s.contains("exolve-grid:\n    CAT\n    A.O\n    BYE\n"),
            "{s}"
        );
        assert!(
            s.contains("exolve-across:\n    1 Pet </script> feline (3) C+AT\n    3 Farewell (3)\n"),
            "{s}"
        );
        assert!(
            s.contains("exolve-down:\n    1 (3)\n    2 (3)\n"),
            "unclued lights still listed: {s}"
        );
        assert!(s.contains("exolve-setter: Setter"));
    }

    #[test]
    fn unfilled_squares_are_question_marks_and_empty_grid_is_zeros() {
        let mut p = sample();
        p.grid.cells[8].letter = None;
        assert!(spec(&p).contains("    BY?\n"));
        for c in &mut p.grid.cells {
            c.letter = None;
        }
        assert!(spec(&p).contains("    000\n    0.0\n    000\n"));
    }

    #[test]
    fn html_is_self_contained_and_safe() {
        let h = html(&sample());
        assert!(h.starts_with("<!DOCTYPE html>"));
        assert!(h.contains("createExolve(") && h.contains("function createExolve"));
        assert!(
            !h.contains("<script src=") && !h.contains("<link "),
            "nothing loaded from the network"
        );
        assert!(
            !h.contains("Pet </script>"),
            "script end tags in clues are escaped"
        );
        assert!(
            h.contains("\\`test\\`") && h.contains("\\${x}"),
            "template-literal syntax escaped"
        );
        assert!(h.contains("<title>Tiny `test` ${x}</title>"));
    }
}
