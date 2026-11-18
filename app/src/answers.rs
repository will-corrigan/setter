//! The puzzle's answers in clue order, and moving between them.

use xw_engine::grid::Dir;
use xw_engine::puzzle::Puzzle;

#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    /// Stable light id, e.g. "0,0,a".
    pub light_id: String,
    /// Clue label, e.g. "10a".
    pub label: String,
    /// Lowercase letters, '?' for an empty square.
    pub pattern: String,
    /// The clue's enumeration if set, otherwise the length.
    pub enumeration: String,
}

/// All answers: acrosses then downs, each in number order.
pub fn answers(p: &Puzzle) -> Vec<Answer> {
    let lights = p.grid.lights();
    let mut out: Vec<Answer> = Vec::with_capacity(lights.len());
    for dir in [Dir::Across, Dir::Down] {
        for l in lights.iter().filter(|l| l.dir == dir) {
            let enumeration = p
                .clues
                .get(&l.id)
                .map(|c| c.enumeration.trim().to_string())
                .filter(|e| !e.is_empty())
                .unwrap_or_else(|| l.len().to_string());
            out.push(Answer {
                light_id: l.id.clone(),
                label: l.label(),
                pattern: p.grid.pattern(l),
                enumeration,
            });
        }
    }
    out
}

/// The light id `delta` places from `current`, wrapping around.
pub fn step(list: &[Answer], current: &str, delta: i32) -> Option<String> {
    if list.is_empty() {
        return None;
    }
    let n = list.len() as i32;
    let i = list
        .iter()
        .position(|a| a.light_id == current)
        .map(|i| i as i32)
        .unwrap_or(0);
    Some(list[(i + delta).rem_euclid(n) as usize].light_id.clone())
}

/// (lights with clue text, all lights)
pub fn clued_count(p: &Puzzle) -> (usize, usize) {
    let lights = p.grid.lights();
    let clued = lights
        .iter()
        .filter(|l| {
            p.clues
                .get(&l.id)
                .is_some_and(|c| !c.text.trim().is_empty())
        })
        .count();
    (clued, lights.len())
}

/// (lights with every square filled, all lights)
pub fn filled_count(p: &Puzzle) -> (usize, usize) {
    let lights = p.grid.lights();
    let filled = lights
        .iter()
        .filter(|l| l.cells.iter().all(|&c| p.grid.cells[c].letter.is_some()))
        .count();
    (filled, lights.len())
}

/// The next answer after `current` with no clue yet, wrapping round; `None`
/// if every answer has one.
pub fn next_unclued(list: &[Answer], p: &Puzzle, current: &str) -> Option<String> {
    let n = list.len();
    let start = list
        .iter()
        .position(|a| a.light_id == current)
        .map_or(0, |i| i + 1);
    (0..n)
        .map(|k| &list[(start + k) % n])
        .find(|a| {
            p.clues
                .get(&a.light_id)
                .is_none_or(|c| c.text.trim().is_empty())
        })
        .map(|a| a.light_id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use xw_engine::grid::Grid;
    use xw_engine::puzzle::Puzzle;

    fn puzzle() -> Puzzle {
        let mut p = Puzzle::new(3, 3);
        p.grid = Grid::from_rows(&["cat", "a#o", "bye"]);
        p
    }

    #[test]
    fn lists_acrosses_then_downs() {
        let list = answers(&puzzle());
        let labels: Vec<_> = list.iter().map(|a| a.label.as_str()).collect();
        assert_eq!(labels, ["1a", "3a", "1d", "2d"]);
        let patterns: Vec<_> = list.iter().map(|a| a.pattern.as_str()).collect();
        assert_eq!(patterns, ["cat", "bye", "cab", "toe"]);
        assert_eq!(list[0].enumeration, "3");
    }

    #[test]
    fn step_wraps_both_ways() {
        let list = answers(&puzzle());
        assert_eq!(step(&list, "0,0,a", -1), Some("0,2,d".to_string()));
        assert_eq!(step(&list, "0,2,d", 1), Some("0,0,a".to_string()));
    }

    #[test]
    fn counts_filled_lights() {
        assert_eq!(filled_count(&puzzle()), (4, 4));
        let mut p = puzzle();
        p.grid = Grid::from_rows(&["c.t", "a#o", "b.."]);
        // Only 1 down (CAB) is complete.
        assert_eq!(filled_count(&p), (1, 4));
        assert_eq!(filled_count(&Puzzle::new(0, 0)), (0, 0));
    }

    #[test]
    fn counts_clued_lights() {
        let mut p = puzzle();
        assert_eq!(clued_count(&p), (0, 4));
        p.clues.insert(
            "0,0,a".into(),
            xw_engine::puzzle::Clue {
                text: "Pet".into(),
                ..Default::default()
            },
        );
        p.clues.insert(
            "2,0,a".into(),
            xw_engine::puzzle::Clue {
                text: "  ".into(),
                ..Default::default()
            },
        );
        assert_eq!(clued_count(&p), (1, 4));
    }

    #[test]
    fn next_unclued_skips_clued_answers_and_wraps() {
        let mut p = Puzzle::new(3, 3);
        p.grid = xw_engine::grid::Grid::from_rows(&["cat", "a#o", "bye"]);
        let list = answers(&p);
        let ids: Vec<&str> = list.iter().map(|a| a.light_id.as_str()).collect();
        p.clues.entry(ids[1].into()).or_default().text = "Farewell".into();
        assert_eq!(next_unclued(&list, &p, ids[0]).as_deref(), Some(ids[2]));
        assert_eq!(
            next_unclued(&list, &p, ids[3]).as_deref(),
            Some(ids[0]),
            "wraps round"
        );
        for id in &ids {
            p.clues.entry((*id).into()).or_default().text = "x".into();
        }
        assert_eq!(next_unclued(&list, &p, ids[0]), None);
    }
}
