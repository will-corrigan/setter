//! Editing the grid: selection, typing, moving, placing and clearing answers,
//! word splits. Pure functions over the puzzle, so they're easy to test.

use xw_engine::fill::shape_for;
use xw_engine::grid::{Dir, Grid, Light};
use xw_engine::puzzle::Puzzle;
use xw_engine::text;

/// The selected square and the direction being filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub cell: usize,
    pub dir: Dir,
}

fn other(dir: Dir) -> Dir {
    if dir == Dir::Across {
        Dir::Down
    } else {
        Dir::Across
    }
}

pub fn light_at(grid: &Grid, cell: usize, dir: Dir) -> Option<Light> {
    grid.lights()
        .into_iter()
        .find(|l| l.dir == dir && l.cells.contains(&cell))
}

/// The light being filled: the cursor's direction, else the other one.
pub fn current_light(grid: &Grid, c: Cursor) -> Option<Light> {
    light_at(grid, c.cell, c.dir).or_else(|| light_at(grid, c.cell, other(c.dir)))
}

fn find_light(p: &Puzzle, id: &str) -> Option<Light> {
    p.grid.lights().into_iter().find(|l| l.id == id)
}

/// Clicking a square: the same square switches direction, a new one keeps it
/// if it can. Blocks can't be selected.
pub fn click(grid: &Grid, c: Cursor, cell: usize) -> Cursor {
    if cell >= grid.cells.len() || grid.cells[cell].block {
        return c;
    }
    if cell == c.cell {
        return if light_at(grid, cell, other(c.dir)).is_some() {
            Cursor {
                cell,
                dir: other(c.dir),
            }
        } else {
            c
        };
    }
    let dir = if light_at(grid, cell, c.dir).is_some() {
        c.dir
    } else {
        other(c.dir)
    };
    Cursor { cell, dir }
}

fn step(grid: &Grid, c: Cursor, delta: isize) -> Cursor {
    let Some(l) = current_light(grid, c) else {
        return c;
    };
    let pos = l.cells.iter().position(|&x| x == c.cell).unwrap_or(0) as isize;
    let next = pos + delta;
    if next < 0 || next as usize >= l.cells.len() {
        return Cursor {
            cell: c.cell,
            dir: l.dir,
        };
    }
    Cursor {
        cell: l.cells[next as usize],
        dir: l.dir,
    }
}

/// Type a letter into the selected square and move along the answer.
pub fn type_letter(p: &mut Puzzle, c: Cursor, ch: char) -> Cursor {
    if p.grid.cells[c.cell].block || !ch.is_ascii_alphabetic() {
        return c;
    }
    p.grid.cells[c.cell].letter = Some(ch.to_ascii_uppercase());
    step(&p.grid, c, 1)
}

/// Delete the selected letter, or step back and delete if the square is empty.
pub fn backspace(p: &mut Puzzle, c: Cursor) -> Cursor {
    let c = if p.grid.cells[c.cell].letter.is_none() {
        step(&p.grid, c, -1)
    } else {
        c
    };
    p.grid.cells[c.cell].letter = None;
    c
}

/// Move with the arrow keys, skipping blocks; the axis sets the direction.
pub fn arrow(grid: &Grid, c: Cursor, dr: i32, dc: i32) -> Cursor {
    let dir = if dr != 0 { Dir::Down } else { Dir::Across };
    let (mut r, mut col) = ((c.cell / grid.width) as i32, (c.cell % grid.width) as i32);
    loop {
        r += dr;
        col += dc;
        if r < 0 || col < 0 || r >= grid.height as i32 || col >= grid.width as i32 {
            return Cursor { cell: c.cell, dir };
        }
        let i = r as usize * grid.width + col as usize;
        if !grid.cells[i].block {
            return Cursor { cell: i, dir };
        }
    }
}

/// Jump to the next (or previous) answer, at its first empty square.
pub fn next_light(grid: &Grid, c: Cursor, delta: i32) -> Cursor {
    let lights = grid.lights();
    if lights.is_empty() {
        return c;
    }
    let cur = current_light(grid, c).map(|l| l.id);
    let i = lights
        .iter()
        .position(|l| Some(&l.id) == cur.as_ref())
        .unwrap_or(0) as i32;
    let l = &lights[(i + delta).rem_euclid(lights.len() as i32) as usize];
    let cell = l
        .cells
        .iter()
        .copied()
        .find(|&x| grid.cells[x].letter.is_none())
        .unwrap_or(l.cells[0]);
    Cursor { cell, dir: l.dir }
}

/// Write an entry ("cart horse") into a light and record its split.
pub fn place(p: &mut Puzzle, light_id: &str, display: &str) {
    let Some(l) = find_light(p, light_id) else {
        return;
    };
    let key = text::key(display).to_uppercase();
    for (cell, ch) in l.cells.iter().zip(key.chars()) {
        p.grid.cells[*cell].letter = Some(ch);
    }
    let e = text::enumeration(display);
    let split = if e.contains([',', '-']) {
        e
    } else {
        String::new()
    };
    if !split.is_empty() || p.clues.contains_key(light_id) {
        p.clues.entry(light_id.to_string()).or_default().enumeration = split;
    }
}

/// Clear an answer, keeping letters that belong to a complete crossing answer.
pub fn clear_light(p: &mut Puzzle, light_id: &str) {
    let Some(l) = find_light(p, light_id) else {
        return;
    };
    for &cell in &l.cells {
        let keep = light_at(&p.grid, cell, other(l.dir))
            .is_some_and(|o| !p.grid.pattern(&o).contains('?'));
        if !keep {
            p.grid.cells[cell].letter = None;
        }
    }
}

/// Set how an answer splits into words. A plain length clears the split; a
/// split that doesn't add up is refused (returns false, nothing changes).
pub fn set_split(p: &mut Puzzle, light_id: &str, input: &str) -> bool {
    let Some(l) = find_light(p, light_id) else {
        return false;
    };
    let value = if input.trim() == l.len().to_string() {
        String::new()
    } else {
        match shape_for(input, l.len()) {
            Some(e) => e,
            None => return false,
        }
    };
    if !value.is_empty() || p.clues.contains_key(light_id) {
        p.clues.entry(light_id.to_string()).or_default().enumeration = value;
    }
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Break {
    Word,
    Hyphen,
}

/// The squares after which a word break falls, for drawing bars and dashes.
pub fn word_breaks(light: &Light, enumeration: &str) -> Vec<(usize, Break)> {
    let Some(e) = shape_for(enumeration, light.len()) else {
        return vec![];
    };
    let mut out = vec![];
    let mut at = 0;
    let mut num = String::new();
    for ch in e.chars() {
        if ch.is_ascii_digit() {
            num.push(ch);
        } else {
            at += num.parse::<usize>().unwrap_or(0);
            num.clear();
            out.push((
                light.cells[at - 1],
                if ch == '-' {
                    Break::Hyphen
                } else {
                    Break::Word
                },
            ));
        }
    }
    out
}

/// Letters spelt out with their split: ("zobrist", "3,4") → "zob rist".
pub fn display_with_split(letters: &str, enumeration: &str) -> String {
    let Some(e) = shape_for(enumeration, letters.len()) else {
        return letters.to_string();
    };
    let mut out = String::new();
    let mut at = 0;
    let mut num = String::new();
    for ch in e.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_digit() {
            num.push(ch);
            continue;
        }
        let n = num.parse::<usize>().unwrap_or(0);
        out.push_str(&letters[at..at + n]);
        at += n;
        num.clear();
        if at < letters.len() {
            out.push(if ch == '-' { '-' } else { ' ' });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use xw_engine::grid::{Dir, Grid};
    use xw_engine::puzzle::Puzzle;

    fn puzzle(rows: &[&str]) -> Puzzle {
        let mut p = Puzzle::new(rows[0].len(), rows.len());
        p.grid = Grid::from_rows(rows);
        p
    }
    fn across(cell: usize) -> Cursor {
        Cursor {
            cell,
            dir: Dir::Across,
        }
    }

    #[test]
    fn click_toggles_on_same_cell_and_picks_a_valid_direction() {
        let p = puzzle(&["cat", "a#o", "bye"]);
        assert_eq!(click(&p.grid, across(0), 0).dir, Dir::Down);
        let c = click(
            &p.grid,
            Cursor {
                cell: 0,
                dir: Dir::Down,
            },
            1,
        );
        assert_eq!((c.cell, c.dir), (1, Dir::Across));
        assert_eq!(
            click(&p.grid, across(0), 4).cell,
            0,
            "blocks can't be selected"
        );
    }

    #[test]
    fn typing_fills_and_advances_and_shares_letters() {
        let mut p = puzzle(&["...", ".#.", "..."]);
        let c = type_letter(&mut p, across(0), 'x');
        assert_eq!(c.cell, 1);
        assert_eq!(p.grid.cells[0].letter, Some('X'));
        let lights = p.grid.lights();
        let down = lights.iter().find(|l| l.id == "0,0,d").unwrap();
        assert_eq!(p.grid.pattern(down), "x??");
        let end = type_letter(&mut p, across(2), 'y');
        assert_eq!(end.cell, 2, "stays on the last square");
    }

    #[test]
    fn backspace_steps_back_over_empty_squares() {
        let mut p = puzzle(&["...", ".#.", "..."]);
        p.grid.cells[0].letter = Some('A');
        let c = backspace(&mut p, across(1));
        assert_eq!(c.cell, 0);
        assert_eq!(p.grid.cells[0].letter, None);
    }

    #[test]
    fn arrows_skip_blocks_and_set_direction() {
        let p = puzzle(&["...", ".#.", "..."]);
        assert_eq!(arrow(&p.grid, across(0), 0, 1), across(1));
        let c = arrow(&p.grid, across(1), 1, 0);
        assert_eq!((c.cell, c.dir), (7, Dir::Down));
        assert_eq!(
            arrow(&p.grid, across(2), 0, 1),
            across(2),
            "stops at the edge"
        );
    }

    #[test]
    fn next_light_wraps_to_first_empty_square() {
        let mut p = puzzle(&["...", ".#.", "..."]);
        p.grid.cells[6].letter = Some('Z');
        let c = next_light(&p.grid, across(0), 1);
        assert_eq!((c.cell, c.dir), (0, Dir::Down));
        let back = next_light(&p.grid, across(0), -1);
        assert_eq!(
            back,
            across(7),
            "wraps to the last light (3 across), first empty square"
        );
    }

    #[test]
    fn placing_a_phrase_sets_its_split() {
        let mut p = puzzle(&["........."]);
        let id = p.grid.lights()[0].id.clone();
        place(&mut p, &id, "cart horse");
        assert_eq!(p.grid.pattern(&p.grid.lights()[0]), "carthorse");
        assert_eq!(p.clues[&id].enumeration, "4,5");
        place(&mut p, &id, "carthorse");
        assert_eq!(p.clues[&id].enumeration, "");
    }

    #[test]
    fn clearing_keeps_letters_of_complete_crossings() {
        let mut p = puzzle(&["cat", "a#o", "bye"]);
        clear_light(&mut p, "0,0,a");
        let row: String = (0..3)
            .map(|i| p.grid.cells[i].letter.unwrap_or('?'))
            .collect();
        assert_eq!(row, "C?T");
    }

    #[test]
    fn splits_must_add_up() {
        let mut p = puzzle(&["........."]);
        let id = p.grid.lights()[0].id.clone();
        assert!(set_split(&mut p, &id, "4,5"));
        assert!(!set_split(&mut p, &id, "4,4"));
        assert_eq!(p.clues[&id].enumeration, "4,5");
        assert!(set_split(&mut p, &id, "9"));
        assert_eq!(p.clues[&id].enumeration, "");
    }

    #[test]
    fn word_breaks_fall_after_each_word() {
        let p = puzzle(&["........."]);
        let l = &p.grid.lights()[0];
        assert_eq!(word_breaks(l, "4,5"), vec![(l.cells[3], Break::Word)]);
        assert_eq!(
            word_breaks(l, "2,1,6"),
            vec![(l.cells[1], Break::Word), (l.cells[2], Break::Word)]
        );
        assert_eq!(word_breaks(l, "3-6"), vec![(l.cells[2], Break::Hyphen)]);
        assert!(word_breaks(l, "9").is_empty());
        assert!(word_breaks(l, "4,4").is_empty());
    }

    #[test]
    fn display_with_split_spells_out_the_words() {
        assert_eq!(display_with_split("zobrist", "3,4"), "zob rist");
        assert_eq!(display_with_split("zobrist", "3-4"), "zob-rist");
        assert_eq!(display_with_split("zobrist", "7"), "zobrist");
        assert_eq!(display_with_split("zobrist", "2,2"), "zobrist");
    }
}
