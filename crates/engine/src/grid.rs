//! Grid model: blocked (British-style) and barred grids, numbering and lights.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Cell {
    #[serde(default)]
    pub block: bool,
    /// Uppercase A–Z, or None if empty.
    #[serde(default)]
    pub letter: Option<char>,
    /// Bar on the right-hand edge (barred grids).
    #[serde(default)]
    pub bar_right: bool,
    /// Bar on the bottom edge (barred grids).
    #[serde(default)]
    pub bar_bottom: bool,
    #[serde(default)]
    pub circled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Cell>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Dir {
    Across,
    Down,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Light {
    pub number: u32,
    pub dir: Dir,
    pub row: usize,
    pub col: usize,
    /// Cell indices, in reading order.
    pub cells: Vec<usize>,
    /// Stable id: "r,c,a" / "r,c,d" (start cell + direction) — survives renumbering.
    pub id: String,
}

impl Light {
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn label(&self) -> String {
        format!(
            "{}{}",
            self.number,
            if self.dir == Dir::Across { "a" } else { "d" }
        )
    }
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Grid {
        Grid {
            width,
            height,
            cells: vec![Cell::default(); width * height],
        }
    }

    /// Build from rows of text: '#' = block, '.' or ' ' = empty, letters = filled.
    pub fn from_rows(rows: &[&str]) -> Grid {
        let height = rows.len();
        let width = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
        let mut g = Grid::new(width, height);
        for (r, row) in rows.iter().enumerate() {
            for (c, ch) in row.chars().enumerate() {
                let cell = &mut g.cells[r * width + c];
                match ch {
                    '#' => cell.block = true,
                    '.' | ' ' | '?' => {}
                    ch if ch.is_ascii_alphabetic() => cell.letter = Some(ch.to_ascii_uppercase()),
                    _ => {}
                }
            }
        }
        g
    }

    pub fn idx(&self, row: usize, col: usize) -> usize {
        row * self.width + col
    }

    fn open(&self, row: usize, col: usize) -> bool {
        !self.cells[self.idx(row, col)].block
    }

    fn joins_right(&self, row: usize, col: usize) -> bool {
        col + 1 < self.width
            && self.open(row, col)
            && self.open(row, col + 1)
            && !self.cells[self.idx(row, col)].bar_right
    }

    fn joins_down(&self, row: usize, col: usize) -> bool {
        row + 1 < self.height
            && self.open(row, col)
            && self.open(row + 1, col)
            && !self.cells[self.idx(row, col)].bar_bottom
    }

    /// All lights (entries of 2+ cells), numbered in the standard way.
    pub fn lights(&self) -> Vec<Light> {
        let mut out = Vec::new();
        let mut number = 0;
        for row in 0..self.height {
            for col in 0..self.width {
                if !self.open(row, col) {
                    continue;
                }
                let starts_across =
                    (col == 0 || !self.joins_right(row, col - 1)) && self.joins_right(row, col);
                let starts_down =
                    (row == 0 || !self.joins_down(row - 1, col)) && self.joins_down(row, col);
                if !(starts_across || starts_down) {
                    continue;
                }
                number += 1;
                if starts_across {
                    let mut cells = vec![self.idx(row, col)];
                    let mut c = col;
                    while self.joins_right(row, c) {
                        c += 1;
                        cells.push(self.idx(row, c));
                    }
                    out.push(Light {
                        number,
                        dir: Dir::Across,
                        row,
                        col,
                        cells,
                        id: format!("{row},{col},a"),
                    });
                }
                if starts_down {
                    let mut cells = vec![self.idx(row, col)];
                    let mut r = row;
                    while self.joins_down(r, col) {
                        r += 1;
                        cells.push(self.idx(r, col));
                    }
                    out.push(Light {
                        number,
                        dir: Dir::Down,
                        row,
                        col,
                        cells,
                        id: format!("{row},{col},d"),
                    });
                }
            }
        }
        out
    }

    /// Pattern for a light: letters lowercase, '?' for empty cells.
    pub fn pattern(&self, light: &Light) -> String {
        light
            .cells
            .iter()
            .map(|&i| {
                self.cells[i]
                    .letter
                    .map(|c| c.to_ascii_lowercase())
                    .unwrap_or('?')
            })
            .collect()
    }

    /// For each cell, the indices of the lights through it (across, down).
    pub fn cell_lights(&self, lights: &[Light]) -> Vec<[Option<usize>; 2]> {
        let mut out = vec![[None, None]; self.cells.len()];
        for (li, l) in lights.iter().enumerate() {
            let slot = if l.dir == Dir::Across { 0 } else { 1 };
            for &c in &l.cells {
                out[c][slot] = Some(li);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbering_blocked_grid() {
        let g = Grid::from_rows(&["...#", ".#..", "....", "#..."]);
        let labels: Vec<String> = g
            .lights()
            .iter()
            .map(|l| format!("{}{}", l.label(), l.len()))
            .collect();
        assert_eq!(
            labels,
            vec!["1a3", "1d3", "2d4", "3a2", "4d3", "5a4", "6d2", "7a3"]
        );
    }

    #[test]
    fn bars_split_lights() {
        let mut g = Grid::from_rows(&["....", "....", "....", "...."]);
        g.cells[1].bar_right = true; // bar after (0,1)
        let lights = g.lights();
        let row0: Vec<usize> = lights
            .iter()
            .filter(|l| l.row == 0 && l.dir == Dir::Across)
            .map(|l| l.len())
            .collect();
        assert_eq!(row0, vec![2, 2]);
    }

    #[test]
    fn patterns() {
        let g = Grid::from_rows(&["c.t"]);
        let l = &g.lights()[0];
        assert_eq!(g.pattern(l), "c?t");
    }
}
