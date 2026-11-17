//! British blocked-grid rules and the grid status shown under the grid.

use crate::fill::FillIndex;
use crate::grid::{Dir, Grid, Light};
use crate::lexicon::Lexicon;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum RuleIssue {
    /// Fewer than half of the light's letters are checked by a crossing light.
    UnderChecked {
        light: String,
        checked: usize,
        len: usize,
    },
    /// Two unchecked squares next to each other within a light.
    DoubleUnch { light: String },
    /// Not every open square can be reached from every other.
    Disconnected,
}

/// "3 across" / "12 down".
pub fn light_name(l: &Light) -> String {
    format!(
        "{} {}",
        l.number,
        if l.dir == Dir::Across {
            "across"
        } else {
            "down"
        }
    )
}

/// British blocked-grid checks: half checking, no double unches, connected.
pub fn british_rules(grid: &Grid) -> Vec<RuleIssue> {
    let lights = grid.lights();
    let cell_lights = grid.cell_lights(&lights);
    let checked = |c: usize| cell_lights[c][0].is_some() && cell_lights[c][1].is_some();
    let mut out = vec![];
    for l in &lights {
        let n = l.cells.iter().filter(|&&c| checked(c)).count();
        if n * 2 < l.len() {
            out.push(RuleIssue::UnderChecked {
                light: l.label(),
                checked: n,
                len: l.len(),
            });
        }
        if l.cells.windows(2).any(|w| !checked(w[0]) && !checked(w[1])) {
            out.push(RuleIssue::DoubleUnch { light: l.label() });
        }
    }
    let open: Vec<usize> = (0..grid.cells.len())
        .filter(|&i| !grid.cells[i].block)
        .collect();
    if let Some(&start) = open.first() {
        let mut seen = HashSet::from([start]);
        let mut stack = vec![start];
        while let Some(i) = stack.pop() {
            let (r, c) = (i / grid.width, i % grid.width);
            let mut next = vec![];
            if r > 0 {
                next.push(i - grid.width);
            }
            if r + 1 < grid.height {
                next.push(i + grid.width);
            }
            if c > 0 {
                next.push(i - 1);
            }
            if c + 1 < grid.width {
                next.push(i + 1);
            }
            for j in next {
                if !grid.cells[j].block && seen.insert(j) {
                    stack.push(j);
                }
            }
        }
        if seen.len() < open.len() {
            out.push(RuleIssue::Disconnected);
        }
    }
    out
}

/// Everything the status line under the grid reports.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct GridStatus {
    pub filled: usize,
    pub total: usize,
    /// Complete answers that aren't in the word list ("3 across").
    pub non_words: Vec<String>,
    /// Incomplete answers that nothing in the word list fits.
    pub unfillable: Vec<String>,
    /// Words used more than once (upper case).
    pub repeats: Vec<String>,
    pub rules: Vec<RuleIssue>,
}

pub fn grid_status(lex: &Lexicon, index: &FillIndex, grid: &Grid) -> GridStatus {
    let lights = grid.lights();
    let mut s = GridStatus {
        total: lights.len(),
        rules: british_rules(grid),
        ..Default::default()
    };
    let mut seen: HashMap<String, usize> = HashMap::new();
    for l in &lights {
        let p = grid.pattern(l);
        if !p.contains('?') {
            s.filled += 1;
            if !lex.contains(&p) {
                s.non_words.push(light_name(l));
            }
            *seen.entry(p.to_uppercase()).or_default() += 1;
        } else if index.count(p.as_bytes()) == 0 {
            s.unfillable.push(light_name(l));
        }
    }
    let mut repeats: Vec<String> = seen
        .into_iter()
        .filter(|(_, n)| *n > 1)
        .map(|(w, _)| w)
        .collect();
    repeats.sort();
    s.repeats = repeats;
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fill::{FillIndex, FillOptions};
    use crate::wordplay::testutil::lex;

    #[test]
    fn open_grid_passes_and_unchecked_rows_fail() {
        assert!(british_rules(&Grid::from_rows(&["...", "...", "..."])).is_empty());
        let issues = british_rules(&Grid::from_rows(&["....", "####", "...."]));
        assert_eq!(
            issues
                .iter()
                .filter(|i| matches!(i, RuleIssue::UnderChecked { .. }))
                .count(),
            2,
            "{issues:?}"
        );
        assert!(issues
            .iter()
            .any(|i| matches!(i, RuleIssue::DoubleUnch { .. })));
    }

    #[test]
    fn disconnected_grid_is_reported() {
        let g = Grid::from_rows(&["..#..", "..#..", "#####", "..#..", "..#.."]);
        assert!(british_rules(&g).contains(&RuleIssue::Disconnected));
    }

    #[test]
    fn status_names_problems() {
        let idx = FillIndex::build(lex(), FillOptions::default());
        // 3x3 with a block in the middle: CAT/BYE across, CAB/TOE down when full.
        let g = Grid::from_rows(&["cat", "a#o", "byq"]);
        let s = grid_status(lex(), &idx, &g);
        assert_eq!((s.filled, s.total), (4, 4));
        assert!(s.non_words.contains(&"3 across".to_string()), "{s:?}");
        assert!(s.non_words.contains(&"2 down".to_string()), "{s:?}");
        let g = Grid::from_rows(&["cat", "a#a", "cat"]);
        let s = grid_status(lex(), &idx, &g);
        assert!(s.repeats.contains(&"CAT".to_string()), "{s:?}");
        let g = Grid::from_rows(&["xq.", "q#.", "..."]);
        let s = grid_status(lex(), &idx, &g);
        assert!(s.unfillable.contains(&"1 across".to_string()), "{s:?}");
        assert!(s.unfillable.contains(&"1 down".to_string()), "{s:?}");
        assert_eq!(s.filled, 0);
    }
}
