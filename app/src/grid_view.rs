//! What the Build grid page shows, worked out from the puzzle: the squares,
//! the answer bar's pattern, the clue sheet's rows, the status line, and
//! where the words popover goes. Pure functions.

use crate::ds::clue_sheet::ClueRow;
use crate::ds::crossword_grid::{GridCell, Lit};
use crate::ds::status_line::{StatusAction, StatusItem, StatusTone};
use crate::ds::word_popover::{Placement, Side, WordEntry};
use crate::grid_edit::{self, Cursor};
use crate::grid_fill::split_of;
use xw_engine::fill::Suggestion;
use xw_engine::grid::{Dir, Light};
use xw_engine::puzzle::Puzzle;
use xw_engine::quality::Quality;
use xw_engine::rules::{light_name, GridStatus, RuleIssue};

/// The label on a non-word's action in the status line.
pub const ADD_WORD: &str = "Add to my words";

/// Every square as the grid draws it: letters, numbers, the selected answer
/// and square, and the word breaks from each answer's split.
pub fn grid_cells(p: &Puzzle, cursor: Cursor) -> Vec<GridCell> {
    let mut cells: Vec<GridCell> = p
        .grid
        .cells
        .iter()
        .map(|c| GridCell {
            block: c.block,
            letter: c.letter.filter(|_| !c.block),
            ..Default::default()
        })
        .collect();
    let lights = p.grid.lights();
    for l in &lights {
        cells[l.cells[0]].number = Some(l.number);
        for (cell, kind) in grid_edit::word_breaks(l, split_of(p, &l.id)) {
            if l.dir == Dir::Across {
                cells[cell].right = Some(kind);
            } else {
                cells[cell].below = Some(kind);
            }
        }
    }
    let on_square = cells.get(cursor.cell).is_some_and(|c| !c.block);
    if on_square {
        // The answer along the cursor's direction, else the one across it.
        let answer = lights
            .iter()
            .find(|l| l.dir == cursor.dir && l.cells.contains(&cursor.cell))
            .or_else(|| lights.iter().find(|l| l.cells.contains(&cursor.cell)));
        for &cell in answer.iter().flat_map(|l| &l.cells) {
            cells[cell].lit = Lit::Answer;
        }
        cells[cursor.cell].lit = Lit::Current;
    }
    cells
}

/// An answer's letters for the answer bar, in capitals with its word breaks
/// and a dot for each empty square: "·A·· ·E·T·".
pub fn shown_pattern(pattern: &str, split: &str) -> String {
    grid_edit::display_with_split(&pattern.to_uppercase(), split).replace('?', "·")
}

/// What the split box shows: the answer's split, or just its length.
pub fn split_text(p: &Puzzle, light: &Light) -> String {
    match split_of(p, &light.id) {
        "" => light.len().to_string(),
        split => split.to_string(),
    }
}

/// The clue sheet's rows: (across, down), in number order.
pub fn clue_rows(p: &Puzzle) -> (Vec<ClueRow>, Vec<ClueRow>) {
    let (mut across, mut down) = (vec![], vec![]);
    for l in p.grid.lights() {
        let clue = p
            .clues
            .get(&l.id)
            .map(|c| c.text.trim())
            .filter(|text| !text.is_empty())
            .map(str::to_string);
        let row = ClueRow {
            number: l.number,
            clue,
            enumeration: split_text(p, &l),
            key: l.id.clone(),
        };
        if l.dir == Dir::Across {
            across.push(row);
        } else {
            down.push(row);
        }
    }
    (across, down)
}

/// "3d" as the status line says it: "3 down".
fn spelt_out(label: &str) -> String {
    match label.strip_suffix('a') {
        Some(number) => format!("{number} across"),
        None => format!("{} down", label.trim_end_matches('d')),
    }
}

/// The status line: what is filled, then each check as passed or as what is
/// wrong. An answer that isn't a word can be added to the user's words.
pub fn status_items(status: &GridStatus, p: &Puzzle) -> Vec<StatusItem> {
    let ok = |text: &str| StatusItem::new(StatusTone::Ok, text);
    let warn = |text: String| StatusItem::new(StatusTone::Warn, text);
    let mut items = vec![StatusItem::new(
        StatusTone::Plain,
        format!("{} of {} answers filled", status.filled, status.total),
    )];

    // Each complete answer that isn't a word, spelt with its split. The
    // status can be a moment behind the grid: an answer that has since lost
    // a letter is passed over.
    let lights = p.grid.lights();
    let non_words: Vec<StatusItem> = status
        .non_words
        .iter()
        .filter_map(|name| {
            let light = lights.iter().find(|l| light_name(l) == *name)?;
            let letters = p.grid.pattern(light);
            if letters.contains('?') {
                return None;
            }
            let word = grid_edit::display_with_split(&letters, split_of(p, &light.id));
            Some(StatusItem {
                tone: StatusTone::Warn,
                text: format!("{} ({name}) is not in the word list", word.to_uppercase()),
                action: Some(StatusAction {
                    label: ADD_WORD.to_string(),
                    key: light.id.clone(),
                }),
            })
        })
        .collect();
    if non_words.is_empty() {
        items.push(ok("every answer is a word"));
    } else {
        items.extend(non_words);
    }

    if status.unfillable.is_empty() {
        items.push(ok("everything still fillable"));
    } else {
        items.push(warn(format!(
            "nothing in the word list fits {}",
            status.unfillable.join(", ")
        )));
    }

    if status.repeats.is_empty() {
        items.push(ok("no repeats"));
    } else {
        items.push(warn(format!("repeated: {}", status.repeats.join(", "))));
    }

    if status.rules.is_empty() {
        items.push(ok("British checking rules met"));
    } else {
        let named = |pick: fn(&RuleIssue) -> Option<&str>| -> Vec<String> {
            status
                .rules
                .iter()
                .filter_map(pick)
                .map(spelt_out)
                .collect()
        };
        let few = named(|issue| match issue {
            RuleIssue::UnderChecked { light, .. } => Some(light),
            _ => None,
        });
        if !few.is_empty() {
            items.push(warn(format!(
                "too few crossing answers for {}",
                few.join(", ")
            )));
        }
        let runs = named(|issue| match issue {
            RuleIssue::DoubleUnch { light } => Some(light),
            _ => None,
        });
        if !runs.is_empty() {
            items.push(warn(format!(
                "two squares in a row cross nothing in {}",
                runs.join(", ")
            )));
        }
        if status.rules.contains(&RuleIssue::Disconnected) {
            items.push(warn("the grid is in separate pieces".to_string()));
        }
    }
    items
}

/// Room for the words popover, in grid squares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Room {
    /// From the grid's left edge to the window's right edge.
    pub columns: f32,
    /// The popover's width, with the gap between it and the answer.
    pub popover_columns: f32,
}

impl Default for Room {
    /// Until it has been measured: all the room in the world.
    fn default() -> Self {
        Room {
            columns: f32::INFINITY,
            popover_columns: 10.0,
        }
    }
}

/// Where the words popover goes: beside the answer's far end, level with
/// its first row. It flips to the answer's near side when there is no room
/// before the window's edge. (The styles slide it up if it would hang below
/// the grid, by as much as its height needs.)
pub fn place_popover(light: &Light, columns: usize, room: Room) -> Placement {
    let columns = columns.max(1);
    let (first, last) = (light.cells[0], light.cells[light.cells.len() - 1]);
    let (row, start, end) = (first / columns, first % columns, last % columns + 1);
    if end as f32 + room.popover_columns <= room.columns {
        Placement {
            side: Side::Right,
            column: end,
            row,
        }
    } else {
        Placement {
            side: Side::Left,
            column: start,
            row,
        }
    }
}

/// The engine's suggestions as the popover lists them: phrases show their split.
pub fn word_entries(found: Vec<(Suggestion, Quality)>) -> Vec<WordEntry> {
    found
        .into_iter()
        .map(|(s, quality)| WordEntry {
            rare: quality == Quality::Rare,
            split: s.enumeration.contains([',', '-']).then_some(s.enumeration),
            text: s.text,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds::crossword_grid::Lit;
    use crate::ds::status_line::{StatusAction, StatusTone};
    use crate::ds::word_popover::Side;
    use crate::grid_edit::{self, Break};
    use xw_engine::grid::{Dir, Grid};
    use xw_engine::rules::RuleIssue;

    fn puzzle(rows: &[&str]) -> Puzzle {
        let mut p = Puzzle::new(rows[0].len(), rows.len());
        p.grid = Grid::from_rows(rows);
        p
    }

    fn at(cell: usize, dir: Dir) -> Cursor {
        Cursor { cell, dir }
    }

    // ---------- the squares ----------

    #[test]
    fn squares_carry_letters_numbers_and_the_selection() {
        let p = puzzle(&["c.t", "a#o", "bye"]);
        let cells = grid_cells(&p, at(1, Dir::Across));
        assert_eq!(cells.len(), 9);
        assert_eq!(
            cells[0],
            GridCell {
                letter: Some('C'),
                number: Some(1),
                lit: Lit::Answer,
                ..Default::default()
            }
        );
        assert_eq!(
            cells[1],
            GridCell {
                lit: Lit::Current,
                ..Default::default()
            }
        );
        assert_eq!((cells[2].number, cells[2].lit), (Some(2), Lit::Answer));
        assert!(cells[4].block);
        assert_eq!((cells[6].number, cells[6].lit), (Some(3), Lit::Off));
        assert_eq!(cells[3].number, None);
        let numbers: Vec<u32> = cells.iter().filter_map(|c| c.number).collect();
        assert_eq!(numbers, [1, 2, 3]);
    }

    #[test]
    fn the_down_answer_lights_up_when_the_cursor_points_down() {
        let p = puzzle(&["c.t", "a#o", "bye"]);
        let cells = grid_cells(&p, at(0, Dir::Down));
        let lit: Vec<usize> = (0..9).filter(|&i| cells[i].lit != Lit::Off).collect();
        assert_eq!(lit, [0, 3, 6]);
        assert_eq!(cells[0].lit, Lit::Current);
    }

    #[test]
    fn splits_draw_breaks_along_their_own_answer() {
        let mut p = puzzle(&["c.t", "a#o", "bye"]);
        assert!(grid_edit::set_split(&mut p, "0,0,a", "1,2"));
        assert!(grid_edit::set_split(&mut p, "0,0,d", "2-1"));
        let cells = grid_cells(&p, at(8, Dir::Across));
        assert_eq!(cells[0].right, Some(Break::Word));
        assert_eq!(cells[0].below, None);
        assert_eq!(cells[3].below, Some(Break::Hyphen));
        assert_eq!(cells[3].right, None);
        let breaks = cells
            .iter()
            .filter(|c| c.right.is_some() || c.below.is_some())
            .count();
        assert_eq!(breaks, 2);
    }

    // ---------- the answer bar ----------

    #[test]
    fn patterns_show_dots_for_gaps_and_the_word_breaks() {
        assert_eq!(shown_pattern("?a???e?t?", "4,5"), "·A·· ·E·T·");
        assert_eq!(shown_pattern("zob????", "3-4"), "ZOB-····");
        assert_eq!(shown_pattern("c?t", ""), "C·T");
        assert_eq!(
            shown_pattern("c?t", "2,2"),
            "C·T",
            "a split that doesn't add up"
        );
        assert_eq!(shown_pattern("ab?de?", "2,1,3"), "AB · DE·");
    }

    #[test]
    fn the_split_box_shows_the_split_or_the_length() {
        let mut p = puzzle(&["........."]);
        let light = p.grid.lights()[0].clone();
        assert_eq!(split_text(&p, &light), "9");
        grid_edit::set_split(&mut p, &light.id, "4,5");
        assert_eq!(split_text(&p, &light), "4,5");
    }

    // ---------- the clue sheet ----------

    #[test]
    fn clue_rows_have_number_clue_and_enumeration_but_never_the_answer() {
        let mut p = puzzle(&["cat", "a#o", "bye"]);
        p.clues.entry("0,0,a".into()).or_default().text = " Pet ".into();
        p.clues.entry("2,0,a".into()).or_default().text = "  ".into();
        grid_edit::set_split(&mut p, "0,0,d", "1,2");
        let (across, down) = clue_rows(&p);
        assert_eq!(
            across,
            vec![
                ClueRow {
                    key: "0,0,a".into(),
                    number: 1,
                    clue: Some("Pet".into()),
                    enumeration: "3".into(),
                },
                ClueRow {
                    key: "2,0,a".into(),
                    number: 3,
                    clue: None,
                    enumeration: "3".into(),
                },
            ]
        );
        assert_eq!(
            down.iter()
                .map(|r| (r.number, r.enumeration.as_str(), r.clue.is_some()))
                .collect::<Vec<_>>(),
            [(1, "1,2", false), (2, "3", false)]
        );
    }

    // ---------- the status line ----------

    fn texts(items: &[StatusItem]) -> Vec<(StatusTone, &str)> {
        items.iter().map(|i| (i.tone, i.text.as_str())).collect()
    }

    #[test]
    fn a_sound_grid_passes_every_check() {
        let p = puzzle(&["cat", "a#o", "bye"]);
        let status = GridStatus {
            filled: 4,
            total: 4,
            ..Default::default()
        };
        let items = status_items(&status, &p);
        assert_eq!(
            texts(&items),
            [
                (StatusTone::Plain, "4 of 4 answers filled"),
                (StatusTone::Ok, "every answer is a word"),
                (StatusTone::Ok, "everything still fillable"),
                (StatusTone::Ok, "no repeats"),
                (StatusTone::Ok, "British checking rules met"),
            ]
        );
        assert!(items.iter().all(|i| i.action.is_none()));
    }

    #[test]
    fn problems_are_named_and_non_words_can_be_added() {
        let mut p = puzzle(&["cat", "a#o", "byq"]);
        grid_edit::set_split(&mut p, "2,0,a", "2,1");
        let status = GridStatus {
            filled: 4,
            total: 4,
            non_words: vec!["3 across".into(), "2 down".into()],
            unfillable: vec!["1 down".into(), "2 down".into()],
            repeats: vec!["CAT".into(), "TOE".into()],
            rules: vec![
                RuleIssue::UnderChecked {
                    light: "3a".into(),
                    checked: 1,
                    len: 3,
                },
                RuleIssue::UnderChecked {
                    light: "12d".into(),
                    checked: 1,
                    len: 3,
                },
                RuleIssue::DoubleUnch { light: "3a".into() },
                RuleIssue::Disconnected,
            ],
        };
        let items = status_items(&status, &p);
        assert_eq!(
            texts(&items),
            [
                (StatusTone::Plain, "4 of 4 answers filled"),
                (StatusTone::Warn, "BY Q (3 across) is not in the word list"),
                (StatusTone::Warn, "TOQ (2 down) is not in the word list"),
                (
                    StatusTone::Warn,
                    "nothing in the word list fits 1 down, 2 down"
                ),
                (StatusTone::Warn, "repeated: CAT, TOE"),
                (
                    StatusTone::Warn,
                    "too few crossing answers for 3 across, 12 down"
                ),
                (
                    StatusTone::Warn,
                    "two squares in a row cross nothing in 3 across"
                ),
                (StatusTone::Warn, "the grid is in separate pieces"),
            ]
        );
        assert_eq!(
            items[1].action,
            Some(StatusAction {
                label: "Add to my words".into(),
                key: "2,0,a".into(),
            })
        );
        assert_eq!(items[2].action.as_ref().unwrap().key, "0,2,d");
        assert!(items[3..].iter().all(|i| i.action.is_none()));
    }

    // ---------- where the popover goes ----------

    fn light(p: &Puzzle, id: &str) -> Light {
        p.grid.lights().into_iter().find(|l| l.id == id).unwrap()
    }

    fn fifteen() -> Puzzle {
        let rows = xw_engine::templates::TEMPLATES[0].rows;
        puzzle(rows)
    }

    #[test]
    fn the_popover_sits_past_the_end_of_an_across_answer() {
        let p = fifteen();
        // Row 2 starts with a nine-letter answer: columns 0 to 8.
        let nine = light(&p, "2,0,a");
        assert_eq!(
            place_popover(&nine, 15, Room::default()),
            Placement {
                side: Side::Right,
                column: 9,
                row: 2,
            }
        );
    }

    #[test]
    fn the_popover_sits_beside_a_down_answer_level_with_its_top() {
        let p = fifteen();
        let down = light(&p, "0,4,d");
        assert_eq!(
            place_popover(&down, 15, Room::default()),
            Placement {
                side: Side::Right,
                column: 5,
                row: 0,
            }
        );
    }

    #[test]
    fn the_popover_flips_when_the_window_edge_is_too_near() {
        let p = fifteen();
        // Row 0's second answer runs from column 6 to the last column.
        let last = light(&p, "0,6,a");
        let narrow = Room {
            columns: 20.0,
            popover_columns: 8.0,
        };
        assert_eq!(
            place_popover(&last, 15, narrow),
            Placement {
                side: Side::Left,
                column: 6,
                row: 0,
            }
        );
        // An answer that ends early enough still opens to the right.
        let first = light(&p, "0,0,a");
        assert_eq!(place_popover(&first, 15, narrow).side, Side::Right);
        // Exactly enough room is enough.
        let snug = Room {
            columns: 23.0,
            ..narrow
        };
        assert_eq!(place_popover(&last, 15, snug).side, Side::Right);
    }

    #[test]
    fn the_popover_is_level_with_the_answer_however_low_it_is() {
        // The styles slide it up as far as its height needs; the row it asks
        // for is always the answer's own.
        let p = fifteen();
        let bottom = light(&p, "14,0,a");
        assert_eq!(place_popover(&bottom, 15, Room::default()).row, 14);
        let down = light(&p, "10,14,d");
        assert_eq!(place_popover(&down, 15, Room::default()).row, 10);
    }

    // ---------- the list of words ----------

    #[test]
    fn phrases_show_their_split_and_single_words_do_not() {
        let found = vec![
            Suggestion {
                text: "cart horse".into(),
                key: "carthorse".into(),
                rank: 9,
                enumeration: "4,5".into(),
                viable: true,
                min_crossing: 3,
            },
            Suggestion {
                text: "orchestra".into(),
                key: "orchestra".into(),
                rank: 2,
                enumeration: "9".into(),
                viable: true,
                min_crossing: 3,
            },
            Suggestion {
                text: "well-to-do".into(),
                key: "welltodo".into(),
                rank: 30,
                enumeration: "4-2-2".into(),
                viable: true,
                min_crossing: 3,
            },
        ];
        let found = found.into_iter().map(|s| (s, Quality::Good)).collect();
        assert_eq!(
            word_entries(found),
            vec![
                WordEntry {
                    text: "cart horse".into(),
                    split: Some("4,5".into()),
                    rare: false,
                },
                WordEntry {
                    text: "orchestra".into(),
                    split: None,
                    rare: false,
                },
                WordEntry {
                    text: "well-to-do".into(),
                    split: Some("4-2-2".into()),
                    rare: false,
                },
            ]
        );
    }
}
