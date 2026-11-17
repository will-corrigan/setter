//! Filling the grid with the engine: new puzzles, undo snapshots, the words
//! that fit an answer, "Fill the rest", "Fill just this answer" and adding
//! the user's own words. Pure functions; the page runs the slow ones off the
//! UI thread.

use crate::grid_edit::{self, Cursor};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
use xw_engine::fill::{
    autofill_shaped, safe_suggestions, shape_for, AutofillResult, FillIndex, Shape, Suggestion,
    WordCount,
};
use xw_engine::grid::{Dir, Grid, Light};
use xw_engine::puzzle::Puzzle;
use xw_engine::rules::light_name;
use xw_engine::Lexicon;

/// How many changes can be undone.
pub const UNDO_STEPS: usize = 200;
/// How many words the popover lists.
pub const WORD_LIMIT: usize = 120;
/// How long "Fill the rest" may take.
pub const FILL_TIME: Duration = Duration::from_secs(20);
/// How long "Fill just this answer" may take.
pub const ANSWER_TIME: Duration = Duration::from_secs(3);
/// How much chance goes into a fill, so filling again gives other words.
const VARIETY: f32 = 0.4;
/// How long the first attempt at a fill gets before starting again.
const FIRST_SPELL: Duration = Duration::from_millis(50);

fn across_from(dir: Dir) -> Dir {
    if dir == Dir::Across {
        Dir::Down
    } else {
        Dir::Across
    }
}

fn find_light(grid: &Grid, light_id: &str) -> Option<Light> {
    grid.lights().into_iter().find(|l| l.id == light_id)
}

fn first_empty(grid: &Grid, light: &Light) -> Option<usize> {
    light
        .cells
        .iter()
        .copied()
        .find(|&c| grid.cells[c].letter.is_none())
}

/// An empty puzzle on a template's grid.
pub fn new_puzzle(rows: &[&str]) -> Puzzle {
    let grid = Grid::from_rows(rows);
    let mut p = Puzzle::new(grid.width, grid.height);
    p.grid = grid;
    p
}

/// Where the cursor starts: the first answer's first square.
pub fn start_cursor(grid: &Grid) -> Cursor {
    match grid.lights().first() {
        Some(l) => Cursor {
            cell: l.cells[0],
            dir: l.dir,
        },
        None => Cursor {
            cell: 0,
            dir: Dir::Across,
        },
    }
}

/// The cursor for an answer: on its first empty square, or its first.
pub fn cursor_on(grid: &Grid, light_id: &str) -> Option<Cursor> {
    let light = find_light(grid, light_id)?;
    Some(Cursor {
        cell: first_empty(grid, &light).unwrap_or(light.cells[0]),
        dir: light.dir,
    })
}

/// What grid editing can change, kept for undo: the letters, each answer's
/// split, and where the cursor was. Clue text is not part of it, so undoing
/// a grid change never loses a clue written since.
#[derive(Debug, Clone, PartialEq)]
pub struct Snap {
    grid: Grid,
    splits: BTreeMap<String, String>,
    pub cursor: Cursor,
}

pub fn snapshot(p: &Puzzle, cursor: Cursor) -> Snap {
    Snap {
        grid: p.grid.clone(),
        splits: p
            .clues
            .iter()
            .filter(|(_, clue)| !clue.enumeration.is_empty())
            .map(|(id, clue)| (id.clone(), clue.enumeration.clone()))
            .collect(),
        cursor,
    }
}

/// Put a snapshot's letters and splits back, leaving clue text alone.
pub fn restore(p: &mut Puzzle, snap: &Snap) {
    p.grid = snap.grid.clone();
    for (id, clue) in p.clues.iter_mut() {
        clue.enumeration = snap.splits.get(id).cloned().unwrap_or_default();
    }
    for (id, split) in &snap.splits {
        if !p.clues.contains_key(id) {
            p.clues.entry(id.clone()).or_default().enumeration = split.clone();
        }
    }
}

/// An answer's split as stored ("4,5"), or "" if it has none.
pub fn split_of<'a>(p: &'a Puzzle, light_id: &str) -> &'a str {
    p.clues
        .get(light_id)
        .map_or("", |clue| clue.enumeration.trim())
}

/// The shape every answer with a split must keep when it is filled.
pub fn shapes(p: &Puzzle) -> HashMap<String, Shape> {
    p.grid
        .lights()
        .into_iter()
        .filter_map(|l| {
            let enumeration = shape_for(split_of(p, &l.id), l.len())?;
            Some((
                l.id,
                Shape {
                    enumeration: Some(enumeration),
                    words: WordCount::Any,
                },
            ))
        })
        .collect()
}

/// The words that fit an answer: safe for its crossings, of the wanted
/// shape, starting with the letters typed in the search box, and not
/// already used elsewhere in the grid.
#[allow(clippy::too_many_arguments)]
pub fn suggest(
    lex: &Lexicon,
    index: &FillIndex,
    grid: &Grid,
    light_id: &str,
    split: &str,
    words: WordCount,
    prefix: &str,
    limit: usize,
) -> Vec<Suggestion> {
    let used: HashSet<String> = grid
        .lights()
        .iter()
        .filter(|l| l.id != light_id)
        .map(|l| grid.pattern(l).to_lowercase())
        .filter(|p| !p.contains('?'))
        .collect();
    let mut found = suggest_any(
        lex,
        index,
        grid,
        light_id,
        split,
        words,
        prefix,
        limit + used.len(),
    );
    found.retain(|s| !used.contains(&s.key));
    found.truncate(limit);
    found
}

#[allow(clippy::too_many_arguments)]
fn suggest_any(
    lex: &Lexicon,
    index: &FillIndex,
    grid: &Grid,
    light_id: &str,
    split: &str,
    words: WordCount,
    prefix: &str,
    limit: usize,
) -> Vec<Suggestion> {
    let Some(light) = find_light(grid, light_id) else {
        return vec![];
    };
    let shape = Shape {
        enumeration: shape_for(split, light.len()),
        words,
    };
    let prefix = xw_engine::text::key(prefix);
    if prefix.is_empty() {
        return safe_suggestions(lex, index, grid, light_id, &shape, limit);
    }
    if prefix.len() > light.len() {
        return vec![];
    }
    // Search with the typed letters written into a copy of the grid, so the
    // whole word list is searched, not just the words already listed. The
    // engine only checks crossings through empty squares, so the crossings
    // of the typed letters are checked here: each depends on its one letter.
    let mut typed = grid.clone();
    for (&cell, ch) in light.cells.iter().zip(prefix.chars()) {
        match typed.cells[cell].letter {
            Some(there) if there.to_ascii_lowercase() != ch => return vec![],
            Some(_) => {}
            None => {
                typed.cells[cell].letter = Some(ch.to_ascii_uppercase());
                let crossing = grid_edit::light_at(&typed, cell, across_from(light.dir));
                if crossing.is_some_and(|o| index.count(typed.pattern(&o).as_bytes()) == 0) {
                    return vec![];
                }
            }
        }
    }
    safe_suggestions(lex, index, &typed, light_id, &shape, limit)
}

/// The next answer after the cursor's that still has empty squares, at its
/// first empty square. The cursor stays put if every answer is full.
pub fn next_open(grid: &Grid, c: Cursor) -> Cursor {
    let lights = grid.lights();
    let here = grid_edit::current_light(grid, c).map(|l| l.id);
    let from = lights
        .iter()
        .position(|l| Some(&l.id) == here.as_ref())
        .unwrap_or(0);
    (1..=lights.len())
        .map(|step| &lights[(from + step) % lights.len()])
        .find_map(|l| first_empty(grid, l).map(|cell| Cursor { cell, dir: l.dir }))
        .unwrap_or(c)
}

/// Autofill, starting again when a search gets stuck. A search either
/// finishes in a few milliseconds or, on an unlucky first choice, goes round
/// in circles until its time is up (splits make that likelier). So it runs
/// in short spells that double in length, each a fresh start, until one
/// comes through or `limit` is used up.
fn autofill_patiently(
    lex: &Lexicon,
    index: &FillIndex,
    grid: &Grid,
    shapes: &HashMap<String, Shape>,
    limit: Duration,
    cancel: &AtomicBool,
) -> AutofillResult {
    let deadline = Instant::now() + limit;
    let mut spell = FIRST_SPELL;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let result = autofill_shaped(lex, index, grid, shapes, spell.min(left), VARIETY, cancel);
        let out_of_time = !result.ok && result.message.contains("time limit");
        if !out_of_time || left <= spell {
            return result;
        }
        spell *= 2;
    }
}

/// Why a fill failed, as a line for the notice.
fn why_not(message: &str) -> String {
    let why = if message.contains("time limit") {
        "ran out of time"
    } else if message.starts_with("No fill exists") {
        "no words fit the letters already in the grid"
    } else {
        "stopped"
    };
    format!("Couldn't fill the rest: {why}.")
}

/// Fill every empty square, keeping the letters and splits already there.
/// Gives the filled puzzle, or why it couldn't be done.
pub fn fill_rest(
    lex: &Lexicon,
    index: &FillIndex,
    p: &Puzzle,
    limit: Duration,
    cancel: &AtomicBool,
) -> Result<Puzzle, String> {
    let result = autofill_patiently(lex, index, &p.grid, &shapes(p), limit, cancel);
    if !result.ok {
        return Err(why_not(&result.message));
    }
    let mut filled = p.clone();
    filled.grid = result.grid;
    // An answer that came out as a phrase gets that phrase's split.
    for l in p.grid.lights() {
        if !p.grid.pattern(&l).contains('?') {
            continue;
        }
        let entry = lex.lookup(&filled.grid.pattern(&l)).map(|id| lex.word(id));
        if let Some(split) = entry.map(|w| w.enumeration()) {
            if split.contains([',', '-']) {
                filled.clues.entry(l.id).or_default().enumeration = split;
            }
        }
    }
    Ok(filled)
}

/// A copy of the grid in which autofill can only reach one answer and the
/// answers crossing its empty squares. Every other empty square is blocked,
/// and the crossing answers are cut off from the rest of the grid, so the
/// answer gets a word its crossings can live with and nothing else is
/// touched. Answer ids stay the same.
pub fn only_this_answer(grid: &Grid, light_id: &str) -> Grid {
    let mut out = grid.clone();
    let Some(light) = find_light(grid, light_id) else {
        return out;
    };
    let empty = |cell: usize| grid.cells[cell].letter.is_none();
    // The answers crossing this one's empty squares: what goes here decides
    // whether they can still be filled.
    let crossings: Vec<Light> = light
        .cells
        .iter()
        .filter(|&&cell| empty(cell))
        .filter_map(|&cell| grid_edit::light_at(grid, cell, across_from(light.dir)))
        .collect();
    let mut kept: HashSet<usize> = light.cells.iter().copied().collect();
    kept.extend(crossings.iter().flat_map(|l| l.cells.iter().copied()));
    for (cell, square) in out.cells.iter_mut().enumerate() {
        if !square.block && empty(cell) && !kept.contains(&cell) {
            square.block = true;
        }
    }
    // Cut each crossing answer's other squares off from their neighbours
    // along this answer's direction, so no stray answers run through them.
    for cell in crossings.iter().flat_map(|l| l.cells.iter().copied()) {
        if light.cells.contains(&cell) {
            continue;
        }
        if light.dir == Dir::Across {
            out.cells[cell].bar_right = true;
            if cell % grid.width > 0 {
                out.cells[cell - 1].bar_right = true;
            }
        } else {
            out.cells[cell].bar_bottom = true;
            if cell >= grid.width {
                out.cells[cell - grid.width].bar_bottom = true;
            }
        }
    }
    out
}

/// Fill one answer with a word that leaves its crossings fillable. Only that
/// answer's squares change.
pub fn fill_answer(
    lex: &Lexicon,
    index: &FillIndex,
    p: &Puzzle,
    light_id: &str,
    limit: Duration,
    cancel: &AtomicBool,
) -> Result<Puzzle, String> {
    let Some(light) = find_light(&p.grid, light_id) else {
        return Ok(p.clone());
    };
    if !p.grid.pattern(&light).contains('?') {
        return Ok(p.clone());
    }
    let prepared = only_this_answer(&p.grid, light_id);
    let result = autofill_patiently(lex, index, &prepared, &shapes(p), limit, cancel);
    let word = result
        .entries
        .into_iter()
        .find(|(id, _)| id == light_id)
        .map(|(_, word)| word);
    let Some(word) = word.filter(|_| result.ok) else {
        return Err(format!(
            "Nothing in the word list fits {}.",
            light_name(&light)
        ));
    };
    let mut filled = p.clone();
    grid_edit::place(&mut filled, light_id, &word);
    Ok(filled)
}

/// What "Add to my words" adds for a complete answer: its letters, shaped by
/// its split ("zob rist"). `None` while the answer has empty squares.
pub fn own_word(p: &Puzzle, light_id: &str) -> Option<String> {
    let light = find_light(&p.grid, light_id)?;
    let letters = p.grid.pattern(&light);
    (!letters.contains('?')).then(|| grid_edit::display_with_split(&letters, split_of(p, light_id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files;
    use crate::state::engine;
    use xw_engine::fill::FillOptions;
    use xw_engine::rules::grid_status;
    use xw_engine::templates::TEMPLATES;

    fn puzzle(rows: &[&str]) -> Puzzle {
        let mut p = Puzzle::new(rows[0].len(), rows.len());
        p.grid = Grid::from_rows(rows);
        p
    }

    fn at(cell: usize, dir: Dir) -> Cursor {
        Cursor { cell, dir }
    }

    fn row(p: &Puzzle, light_id: &str) -> String {
        let l = p.grid.lights().into_iter().find(|l| l.id == light_id);
        p.grid.pattern(&l.expect("light"))
    }

    fn no_cancel() -> AtomicBool {
        AtomicBool::new(false)
    }

    // ---------- picker → puzzle ----------

    #[test]
    fn a_picked_template_is_an_empty_puzzle_on_that_grid() {
        let t = &TEMPLATES[4];
        let p = new_puzzle(t.rows);
        assert_eq!((p.grid.width, p.grid.height), (13, 13));
        for (r, line) in t.rows.iter().enumerate() {
            for (c, ch) in line.chars().enumerate() {
                let cell = &p.grid.cells[r * 13 + c];
                assert_eq!(cell.block, ch == '#', "row {r} column {c}");
                assert_eq!(cell.letter, None);
            }
        }
        assert!(p.clues.is_empty());
        assert!(p.title.is_empty());
    }

    #[test]
    fn the_cursor_starts_on_the_first_answer() {
        let p = puzzle(&["#..", "...", "..#"]);
        assert_eq!(start_cursor(&p.grid), at(1, Dir::Across));
        let blocks = puzzle(&["##", "##"]);
        assert_eq!(start_cursor(&blocks.grid), at(0, Dir::Across));
    }

    #[test]
    fn the_cursor_lands_on_an_answers_first_empty_square() {
        let p = puzzle(&["c.t", "a#o", "bye"]);
        assert_eq!(cursor_on(&p.grid, "0,0,a"), Some(at(1, Dir::Across)));
        assert_eq!(cursor_on(&p.grid, "0,2,d"), Some(at(2, Dir::Down)));
        assert_eq!(cursor_on(&p.grid, "9,9,a"), None);
    }

    // ---------- undo snapshots ----------

    #[test]
    fn restoring_brings_back_letters_and_splits_but_keeps_clue_text() {
        let mut p = puzzle(&["........."]);
        let id = "0,0,a";
        let before = snapshot(&p, at(0, Dir::Across));
        grid_edit::place(&mut p, id, "cart horse");
        p.clues.get_mut(id).unwrap().text = "Wagon puller".into();
        let after = snapshot(&p, at(4, Dir::Across));

        restore(&mut p, &before);
        assert_eq!(row(&p, id), "?????????");
        assert_eq!(p.clues[id].enumeration, "");
        assert_eq!(p.clues[id].text, "Wagon puller", "clue text survives undo");
        assert_eq!(before.cursor, at(0, Dir::Across));

        restore(&mut p, &after);
        assert_eq!(row(&p, id), "carthorse");
        assert_eq!(p.clues[id].enumeration, "4,5");
        assert_eq!(p.clues[id].text, "Wagon puller");
    }

    #[test]
    fn restoring_a_split_for_an_answer_with_no_clue_entry_adds_it() {
        let mut p = puzzle(&["........."]);
        grid_edit::set_split(&mut p, "0,0,a", "4,5");
        let with_split = snapshot(&p, at(0, Dir::Across));
        let mut fresh = puzzle(&["........."]);
        restore(&mut fresh, &with_split);
        assert_eq!(split_of(&fresh, "0,0,a"), "4,5");
    }

    // ---------- splits as shapes ----------

    #[test]
    fn only_real_splits_become_shapes() {
        let mut p = puzzle(&[".........", "#########", "........."]);
        p.clues.entry("0,0,a".into()).or_default().enumeration = "4,5".into();
        p.clues.entry("2,0,a".into()).or_default().enumeration = "9".into();
        let shapes = shapes(&p);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes["0,0,a"].enumeration.as_deref(), Some("4,5"));
        assert_eq!(split_of(&p, "0,0,a"), "4,5");
        assert_eq!(split_of(&p, "1,0,a"), "");
    }

    // ---------- the words that fit ----------

    #[test]
    fn words_already_in_the_grid_are_not_offered_again() {
        let e = engine();
        // Two separate three-letter answers: CAT along the top, the bottom open.
        let p = puzzle(&["cat", "###", "..."]);
        let open = suggest(&e.lex, e.fill(), &p.grid, "2,0,a", "", WordCount::Any, "ca", 40);
        assert!(!open.is_empty());
        assert!(open.iter().all(|s| s.key != "cat"), "{open:?}");
        // An answer's own word is still listed for it.
        let own = suggest(&e.lex, e.fill(), &p.grid, "0,0,a", "", WordCount::Any, "ca", 40);
        assert!(own.iter().any(|s| s.key == "cat"), "{own:?}");
    }

    #[test]
    fn suggestions_follow_the_split_the_chips_and_the_search() {
        let e = engine();
        let p = puzzle(&["........."]);
        let id = "0,0,a";
        let split = suggest(&e.lex, e.fill(), &p.grid, id, "4,5", WordCount::Any, "", 40);
        assert!(!split.is_empty());
        assert!(split.iter().all(|s| s.enumeration == "4,5"), "{split:?}");

        let two = suggest(&e.lex, e.fill(), &p.grid, id, "", WordCount::Two, "", 40);
        assert!(!two.is_empty());
        assert!(two
            .iter()
            .all(|s| s.enumeration.matches([',', '-']).count() == 1));

        let searched = suggest(
            &e.lex,
            e.fill(),
            &p.grid,
            id,
            "",
            WordCount::Any,
            "Orch",
            40,
        );
        assert!(
            searched.iter().any(|s| s.key == "orchestra"),
            "{searched:?}"
        );
        assert!(searched.iter().all(|s| s.key.starts_with("orch")));
        assert!(searched.len() <= 40);
    }

    #[test]
    fn a_search_reaches_past_the_first_page_of_words() {
        let e = engine();
        let p = puzzle(&["....."]);
        let first_page = suggest(
            &e.lex,
            e.fill(),
            &p.grid,
            "0,0,a",
            "",
            WordCount::Any,
            "",
            WORD_LIMIT,
        );
        assert!(!first_page.iter().any(|s| s.key == "zebra"));
        let searched = suggest(
            &e.lex,
            e.fill(),
            &p.grid,
            "0,0,a",
            "",
            WordCount::Any,
            "zeb",
            WORD_LIMIT,
        );
        assert!(searched.iter().any(|s| s.key == "zebra"), "{searched:?}");
    }

    #[test]
    fn a_search_never_offers_words_that_break_a_crossing_or_the_letters_in_place() {
        // 1 across is ???A; 1 down runs from its first square. Nothing starts
        // with D going down, so DIVA would leave 1 down with no word.
        let lex = Lexicon::from_words(&["sofa", "tuba", "diva", "sit", "tan"]);
        let index = FillIndex::build(&lex, FillOptions::default());
        let p = puzzle(&["...a", ".###", ".###"]);
        let keys = |prefix: &str| -> Vec<String> {
            suggest(
                &lex,
                &index,
                &p.grid,
                "0,0,a",
                "",
                WordCount::Any,
                prefix,
                40,
            )
            .into_iter()
            .map(|s| s.key)
            .collect()
        };
        assert_eq!(keys(""), ["sofa", "tuba"]);
        assert_eq!(keys("t"), ["tuba"]);
        assert_eq!(keys("So"), ["sofa"]);
        assert!(keys("d").is_empty(), "D leaves 1 down with nothing");
        assert!(keys("sofb").is_empty(), "the last letter is already A");
        assert!(keys("sofas").is_empty(), "longer than the answer");
        // The search changed nothing.
        assert_eq!(p.grid, Grid::from_rows(&["...a", ".###", ".###"]));
    }

    #[test]
    fn after_placing_the_cursor_moves_to_the_next_answer_with_room() {
        let p = puzzle(&["cat", "a#.", "b.."]);
        // 1 across is full; next in order is 1 down (full), then 2 down.
        let c = next_open(&p.grid, at(0, Dir::Across));
        assert_eq!(c, at(5, Dir::Down));
        let full = puzzle(&["cat", "a#o", "bye"]);
        assert_eq!(
            next_open(&full.grid, at(1, Dir::Across)),
            at(1, Dir::Across)
        );
    }

    // ---------- fill the rest ----------

    #[test]
    fn fill_the_rest_keeps_letters_and_splits_and_records_new_splits() {
        let e = engine();
        let mut p = new_puzzle(TEMPLATES[0].rows);
        let lights = p.grid.lights();
        let nine = lights
            .iter()
            .find(|l| l.len() == 9 && l.dir == Dir::Across)
            .expect("a nine-letter across");
        grid_edit::set_split(&mut p, &nine.id, "4,5");
        let five = lights.iter().find(|l| l.len() == 5).expect("a five");
        grid_edit::place(&mut p, &five.id, "trees");
        p.clues.entry(five.id.clone()).or_default().text = "Woodland".into();

        // A single search gets stuck on a split about one time in four;
        // the fill has to come through every time.
        for round in 0..8 {
            let filled = fill_rest(&e.lex, e.fill(), &p, FILL_TIME, &no_cancel())
                .unwrap_or_else(|why| panic!("round {round}: {why}"));
            assert!(filled
                .grid
                .cells
                .iter()
                .all(|c| c.block || c.letter.is_some()));
            assert_eq!(row(&filled, &five.id), "trees");
            assert_eq!(filled.clues[&five.id].text, "Woodland");
            assert_eq!(filled.clues[&nine.id].enumeration, "4,5");
            let word = e.lex.lookup(&row(&filled, &nine.id)).expect("a real entry");
            assert_eq!(e.lex.word(word).enumeration(), "4,5");
            // Any phrase it chose carries its split.
            for l in filled.grid.lights() {
                let id = e
                    .lex
                    .lookup(&filled.grid.pattern(&l))
                    .expect("a real entry");
                let en = e.lex.word(id).enumeration();
                if en.contains([',', '-']) {
                    assert_eq!(split_of(&filled, &l.id), en, "{}", l.id);
                }
            }
        }
    }

    #[test]
    fn an_impossible_fill_says_why_and_gives_nothing() {
        let lex = Lexicon::from_words(&["cat"]);
        let index = FillIndex::build(&lex, FillOptions::default());
        let p = puzzle(&["...", "...", "..."]);
        let why = fill_rest(&lex, &index, &p, FILL_TIME, &no_cancel()).unwrap_err();
        assert_eq!(
            why,
            "Couldn't fill the rest: no words fit the letters already in the grid."
        );
    }

    #[test]
    fn every_reason_a_fill_stops_reads_as_a_sentence() {
        assert_eq!(
            why_not("Stopped: time limit reached"),
            "Couldn't fill the rest: ran out of time."
        );
        assert_eq!(
            why_not("No fill exists with the current word list and settings"),
            "Couldn't fill the rest: no words fit the letters already in the grid."
        );
        assert_eq!(
            why_not("Stopped: cancelled"),
            "Couldn't fill the rest: stopped."
        );
    }

    // ---------- fill just this answer ----------

    #[test]
    fn only_this_answer_and_its_crossings_stay_open() {
        // 1 across with its downs at columns 0 and 2; 3 across along the bottom.
        let g = Grid::from_rows(&["...", ".#.", "..."]);
        let prepared = only_this_answer(&g, "0,0,a");
        let ids: Vec<String> = prepared.lights().into_iter().map(|l| l.id).collect();
        assert_eq!(ids, ["0,0,a", "0,0,d", "0,2,d"]);
        assert!(prepared.cells[7].block, "the square only 3 across uses");
        for i in [0, 1, 2, 3, 5, 6, 8] {
            assert!(!prepared.cells[i].block, "square {i}");
        }
    }

    #[test]
    fn crossings_through_letters_already_in_place_are_left_out() {
        // The C is in: 1 down can't change what goes in 1 across.
        let g = Grid::from_rows(&["c..", ".#.", "..."]);
        let prepared = only_this_answer(&g, "0,0,a");
        let open: Vec<String> = prepared
            .lights()
            .into_iter()
            .filter(|l| prepared.pattern(l).contains('?'))
            .map(|l| l.id)
            .collect();
        assert_eq!(open, ["0,0,a", "0,2,d"]);
        assert!(prepared.cells[3].block && prepared.cells[6].block && prepared.cells[7].block);
        assert_eq!(prepared.cells[0].letter, Some('C'));
    }

    #[test]
    fn crossing_answers_are_cut_off_from_their_other_crossings() {
        // 1 down's last square also starts 3 across (B?E). Left joined, the
        // autofill would have to make a word of that too.
        let g = Grid::from_rows(&["...", ".#.", ".ye"]);
        let prepared = only_this_answer(&g, "0,0,a");
        let ids: Vec<String> = prepared.lights().into_iter().map(|l| l.id).collect();
        assert_eq!(ids, ["0,0,a", "0,0,d", "0,2,d"]);
        assert_eq!(prepared.cells[7].letter, Some('Y'), "letters stay");
        // Down answers work the same way round.
        let down = only_this_answer(&g, "0,0,d");
        let ids: Vec<String> = down
            .lights()
            .into_iter()
            .filter(|l| down.pattern(l).contains('?'))
            .map(|l| l.id)
            .collect();
        assert_eq!(ids, ["0,0,a", "0,0,d", "2,0,a"]);
    }

    #[test]
    fn fill_just_this_answer_changes_only_that_answer() {
        let e = engine();
        let mut p = new_puzzle(TEMPLATES[0].rows);
        let lights = p.grid.lights();
        let nine = lights
            .iter()
            .find(|l| l.len() == 9 && l.dir == Dir::Across)
            .unwrap()
            .clone();
        grid_edit::set_split(&mut p, &nine.id, "4,5");
        let filled =
            fill_answer(&e.lex, e.fill(), &p, &nine.id, ANSWER_TIME, &no_cancel()).expect("fills");
        let word = row(&filled, &nine.id);
        assert!(!word.contains('?'), "{word}");
        let id = e.lex.lookup(&word).expect("a real entry");
        assert_eq!(e.lex.word(id).enumeration(), "4,5");
        assert_eq!(split_of(&filled, &nine.id), "4,5");
        for (i, cell) in filled.grid.cells.iter().enumerate() {
            if !nine.cells.contains(&i) {
                assert_eq!(cell, &p.grid.cells[i], "square {i} was touched");
            }
        }
        // Every crossing can still be filled.
        let status = grid_status(&e.lex, e.fill(), &filled.grid);
        assert!(status.unfillable.is_empty(), "{status:?}");
    }

    #[test]
    fn an_answer_nothing_fits_says_so() {
        let e = engine();
        let p = puzzle(&["qqq.."]);
        let why =
            fill_answer(&e.lex, e.fill(), &p, "0,0,a", ANSWER_TIME, &no_cancel()).unwrap_err();
        assert_eq!(why, "Nothing in the word list fits 1 across.");
    }

    #[test]
    fn a_full_answer_is_left_as_it_is() {
        let e = engine();
        let p = puzzle(&["cat", "a#o", "bye"]);
        let same = fill_answer(&e.lex, e.fill(), &p, "0,0,a", ANSWER_TIME, &no_cancel()).unwrap();
        assert_eq!(same, p);
    }

    // ---------- add to my words ----------

    #[test]
    fn own_words_are_spelt_with_their_split() {
        let mut p = puzzle(&["zobrist", "#######", "zob...."]);
        assert_eq!(own_word(&p, "0,0,a").as_deref(), Some("zobrist"));
        grid_edit::set_split(&mut p, "0,0,a", "3,4");
        assert_eq!(own_word(&p, "0,0,a").as_deref(), Some("zob rist"));
        grid_edit::set_split(&mut p, "0,0,a", "3-4");
        assert_eq!(own_word(&p, "0,0,a").as_deref(), Some("zob-rist"));
        assert_eq!(own_word(&p, "2,0,a"), None, "not finished yet");
        assert_eq!(own_word(&p, "9,9,a"), None);
    }

    #[test]
    fn an_added_word_counts_as_a_word_everywhere() {
        let dir = std::env::temp_dir().join(format!("setter-test-myword-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        files::ensure_user_files_in(&dir);
        let mut p = puzzle(&["zobrist"]);
        grid_edit::set_split(&mut p, "0,0,a", "3,4");

        let before = files::load_engine_data_from(&dir);
        let index = FillIndex::build(&before.lex, crate::state::GRID_WORDS);
        let status = grid_status(&before.lex, &index, &p.grid);
        assert_eq!(status.non_words, ["1 across"]);

        let word = own_word(&p, "0,0,a").unwrap();
        files::add_own_word_in(&dir, &word).unwrap();
        let after = files::load_engine_data_from(&dir);
        let index = FillIndex::build(&after.lex, crate::state::GRID_WORDS);
        // Status: it is a word now.
        assert!(grid_status(&after.lex, &index, &p.grid)
            .non_words
            .is_empty());
        // Suggestions: it fits its own pattern, with its split.
        let empty = puzzle(&["zob...."]);
        let found = suggest(
            &after.lex,
            &index,
            &empty.grid,
            "0,0,a",
            "3,4",
            WordCount::Any,
            "",
            10,
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].text, "zob rist");
        // Autofill: it can be used to fill.
        let filled = fill_rest(&after.lex, &index, &empty, FILL_TIME, &no_cancel()).unwrap();
        assert_eq!(row(&filled, "0,0,a"), "zobrist");
        assert_eq!(split_of(&filled, "0,0,a"), "3,4");
    }
}
