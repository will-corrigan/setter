//! A whole puzzle: grid, clues and metadata. Saved as JSON (`.xword`);
//! exportable to ipuz.

use crate::grid::{Dir, Grid};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Clue {
    #[serde(default)]
    pub text: String,
    /// e.g. "5,3" or "4-4". Empty means "use the light length".
    #[serde(default)]
    pub enumeration: String,
    /// Setter's private explanation of the wordplay.
    #[serde(default)]
    pub annotation: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Symmetry {
    #[default]
    Rotational,
    Mirror,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Puzzle {
    #[serde(default = "one")]
    pub format: u32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub preamble: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub symmetry: Symmetry,
    pub grid: Grid,
    /// Keyed by light id ("r,c,a").
    #[serde(default)]
    pub clues: BTreeMap<String, Clue>,
}

fn one() -> u32 {
    1
}

impl Puzzle {
    pub fn new(width: usize, height: usize) -> Puzzle {
        Puzzle {
            format: 1,
            title: String::new(),
            author: String::new(),
            preamble: String::new(),
            notes: String::new(),
            symmetry: Symmetry::Rotational,
            grid: Grid::new(width, height),
            clues: BTreeMap::new(),
        }
    }

    pub fn to_ipuz(&self) -> Value {
        let g = &self.grid;
        let lights = g.lights();
        let mut numbers: BTreeMap<usize, u32> = BTreeMap::new();
        for l in &lights {
            numbers.insert(l.cells[0], l.number);
        }
        let mut puzzle = Vec::new();
        let mut solution = Vec::new();
        for r in 0..g.height {
            let mut prow = Vec::new();
            let mut srow = Vec::new();
            for c in 0..g.width {
                let i = g.idx(r, c);
                let cell = &g.cells[i];
                if cell.block {
                    prow.push(json!("#"));
                    srow.push(json!("#"));
                    continue;
                }
                let mut bars = String::new();
                if cell.bar_right {
                    bars.push('R');
                }
                if cell.bar_bottom {
                    bars.push('B');
                }
                let num = numbers.get(&i).map(|n| json!(n)).unwrap_or(json!(0));
                if bars.is_empty() && !cell.circled {
                    prow.push(num);
                } else {
                    let mut style = serde_json::Map::new();
                    if !bars.is_empty() {
                        style.insert("barred".into(), json!(bars));
                    }
                    if cell.circled {
                        style.insert("shapebg".into(), json!("circle"));
                    }
                    prow.push(json!({ "cell": num, "style": style }));
                }
                srow.push(
                    cell.letter
                        .map(|l| json!(l.to_string()))
                        .unwrap_or(json!(null)),
                );
            }
            puzzle.push(Value::Array(prow));
            solution.push(Value::Array(srow));
        }
        let mut across = Vec::new();
        let mut down = Vec::new();
        for l in &lights {
            let clue = self.clues.get(&l.id).cloned().unwrap_or_default();
            let enumeration = if clue.enumeration.trim().is_empty() {
                l.len().to_string()
            } else {
                clue.enumeration.trim().to_string()
            };
            let entry =
                json!({ "number": l.number, "clue": clue.text, "enumeration": enumeration });
            if l.dir == Dir::Across {
                across.push(entry)
            } else {
                down.push(entry)
            }
        }
        json!({
            "version": "http://ipuz.org/v2",
            "kind": ["http://ipuz.org/crossword#1"],
            "title": self.title,
            "author": self.author,
            "intro": self.preamble,
            "dimensions": { "width": g.width, "height": g.height },
            "block": "#",
            "empty": 0,
            "puzzle": puzzle,
            "solution": solution,
            "clues": { "Across": across, "Down": down },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_json() {
        let mut p = Puzzle::new(3, 3);
        p.grid.cells[4].block = true;
        p.clues.insert(
            "0,0,a".into(),
            Clue {
                text: "Feline".into(),
                enumeration: "3".into(),
                annotation: String::new(),
            },
        );
        let s = serde_json::to_string(&p).unwrap();
        let q: Puzzle = serde_json::from_str(&s).unwrap();
        assert_eq!(p, q);
    }

    #[test]
    fn ipuz_has_numbers_and_clues() {
        let mut p = Puzzle::new(3, 3);
        p.grid = Grid::from_rows(&["cat", "a#o", "bye"]);
        p.clues.insert(
            "0,0,a".into(),
            Clue {
                text: "Pet".into(),
                ..Default::default()
            },
        );
        let v = p.to_ipuz();
        assert_eq!(v["puzzle"][0][0], json!(1));
        assert_eq!(v["puzzle"][1][1], json!("#"));
        assert_eq!(v["solution"][0][0], json!("C"));
        assert_eq!(v["clues"]["Across"][0]["clue"], json!("Pet"));
        assert_eq!(v["clues"]["Across"][0]["enumeration"], json!("3"));
    }
}
