//! Grid filling: fast pattern matching via per-position letter bitsets,
//! suggestions that check the crossing lights stay fillable, and a
//! backtracking autofill.

use crate::grid::{Grid, Light};
use crate::lexicon::{Lexicon, WordId};
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct FillOptions {
    /// Only use entries more popular than this rank (1 = most popular).
    pub max_rank: u32,
    pub allow_proper: bool,
    pub allow_phrases: bool,
}

impl Default for FillOptions {
    fn default() -> Self {
        FillOptions {
            max_rank: 120_000,
            allow_proper: false,
            allow_phrases: true,
        }
    }
}

struct LenIndex {
    ids: Vec<WordId>,
    /// bits[pos * 26 + letter] = bitset over `ids`
    bits: Vec<Vec<u64>>,
    words: usize,
}

impl LenIndex {
    fn blocks(&self) -> usize {
        self.words.div_ceil(64)
    }
}

pub struct FillIndex {
    pub options: FillOptions,
    lens: Vec<Option<LenIndex>>,
}

/// A bitset of matching words for one pattern.
pub struct Matches<'a> {
    li: Option<&'a LenIndex>,
    bits: Vec<u64>,
}

impl<'a> Matches<'a> {
    pub fn count(&self) -> usize {
        self.bits.iter().map(|b| b.count_ones() as usize).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.bits.iter().all(|&b| b == 0)
    }

    /// Matching word ids, most popular first.
    pub fn ids(&self) -> impl Iterator<Item = WordId> + '_ {
        let li = self.li;
        self.bits.iter().enumerate().flat_map(move |(bi, &b)| {
            let mut b = b;
            std::iter::from_fn(move || {
                if b == 0 {
                    return None;
                }
                let t = b.trailing_zeros() as usize;
                b &= b - 1;
                Some(li.unwrap().ids[bi * 64 + t])
            })
        })
    }
}

impl FillIndex {
    pub fn build(lex: &Lexicon, options: FillOptions) -> FillIndex {
        let mut lens: Vec<Option<LenIndex>> = Vec::new();
        for len in 0..=lex.max_len().min(25) {
            let ids: Vec<WordId> = lex
                .with_len(len)
                .iter()
                .copied()
                .filter(|&id| {
                    let w = lex.word(id);
                    id < options.max_rank
                        && (options.allow_proper || !w.proper)
                        && (options.allow_phrases || !w.phrase)
                })
                .collect();
            if len == 0 || ids.is_empty() {
                lens.push(None);
                continue;
            }
            let words = ids.len();
            let blocks = words.div_ceil(64);
            let mut bits = vec![vec![0u64; blocks]; len * 26];
            for (i, &id) in ids.iter().enumerate() {
                let k = lex.word(id).key.as_bytes();
                for (p, &b) in k.iter().enumerate() {
                    bits[p * 26 + (b - b'a') as usize][i / 64] |= 1 << (i % 64);
                }
            }
            lens.push(Some(LenIndex { ids, bits, words }));
        }
        FillIndex { options, lens }
    }

    /// Words matching a pattern of lowercase letters and '?' wildcards.
    pub fn matches(&self, pattern: &[u8]) -> Matches<'_> {
        let Some(Some(li)) = self.lens.get(pattern.len()) else {
            return Matches {
                li: None,
                bits: vec![],
            };
        };
        let mut bits = vec![u64::MAX; li.blocks()];
        let tail = li.words % 64;
        if tail != 0 {
            *bits.last_mut().unwrap() = (1u64 << tail) - 1;
        }
        for (p, &b) in pattern.iter().enumerate() {
            if b.is_ascii_lowercase() {
                let set = &li.bits[p * 26 + (b - b'a') as usize];
                for (x, y) in bits.iter_mut().zip(set) {
                    *x &= *y;
                }
            }
        }
        Matches { li: Some(li), bits }
    }

    pub fn count(&self, pattern: &[u8]) -> usize {
        self.matches(pattern).count()
    }
}

/// A suggested entry for a light.
#[derive(Debug, Serialize)]
pub struct Suggestion {
    pub text: String,
    pub key: String,
    pub rank: u32,
    pub enumeration: String,
    /// Every crossing light still has at least one possible fill.
    pub viable: bool,
    /// Fewest options left among the crossing lights (higher = safer).
    pub min_crossing: usize,
}

fn grid_bytes(grid: &Grid) -> Vec<u8> {
    grid.cells
        .iter()
        .map(|c| {
            if c.block {
                b'#'
            } else {
                c.letter
                    .map(|l| l.to_ascii_lowercase() as u8)
                    .unwrap_or(b'?')
            }
        })
        .collect()
}

fn pattern_of(cells: &[u8], light: &Light) -> Vec<u8> {
    light.cells.iter().map(|&i| cells[i]).collect()
}

/// Entries fitting one light, each rated against its crossings. Scans at most
/// `scan` entries that pass `keep`, stopping early once `enough` are viable.
fn rated(
    lex: &Lexicon,
    index: &FillIndex,
    grid: &Grid,
    light_id: &str,
    keep: impl Fn(&crate::lexicon::Word) -> bool,
    scan: usize,
    enough: usize,
) -> Vec<Suggestion> {
    let lights = grid.lights();
    let Some(li) = lights.iter().position(|l| l.id == light_id) else {
        return vec![];
    };
    let light = &lights[li];
    let cell_lights = grid.cell_lights(&lights);
    let mut cells = grid_bytes(grid);
    let pattern = pattern_of(&cells, light);

    // Crossings through empty cells: (position in this light, crossing light index).
    let crossings: Vec<(usize, usize)> = light
        .cells
        .iter()
        .enumerate()
        .filter(|(_, &c)| cells[c] == b'?')
        .filter_map(|(p, &c)| {
            let [a, d] = cell_lights[c];
            let other = if light.dir == crate::grid::Dir::Across {
                d
            } else {
                a
            };
            other.map(|o| (p, o))
        })
        .collect();

    let m = index.matches(&pattern);
    let mut out: Vec<Suggestion> = Vec::new();
    let mut scanned = 0;
    let mut viable = 0;
    for id in m.ids() {
        let w = lex.word(id);
        if !keep(w) {
            continue;
        }
        scanned += 1;
        if scanned > scan || viable >= enough {
            break;
        }
        let key = w.key.as_bytes();
        for (p, &c) in light.cells.iter().enumerate() {
            cells[c] = key[p];
        }
        let mut min_crossing = usize::MAX;
        for &(_, o) in &crossings {
            let n = index.count(&pattern_of(&cells, &lights[o]));
            min_crossing = min_crossing.min(n);
            if n == 0 {
                break;
            }
        }
        for (p, &c) in light.cells.iter().enumerate() {
            cells[c] = pattern[p];
        }
        viable += usize::from(min_crossing > 0);
        out.push(Suggestion {
            text: w.display().to_string(),
            key: w.key.clone(),
            rank: id + 1,
            enumeration: w.enumeration(),
            viable: min_crossing > 0,
            min_crossing: if crossings.is_empty() {
                0
            } else {
                min_crossing
            },
        });
    }
    out
}

/// Ranked fills for one light, checking each against its crossings.
pub fn suggestions(
    lex: &Lexicon,
    index: &FillIndex,
    grid: &Grid,
    light_id: &str,
    limit: usize,
) -> Vec<Suggestion> {
    let mut out = rated(lex, index, grid, light_id, |_| true, limit * 4, usize::MAX);
    out.sort_by_key(|s| (!s.viable, s.rank));
    out.truncate(limit);
    out
}

/// How many words an entry should have.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WordCount {
    #[default]
    Any,
    One,
    Two,
    ThreePlus,
}

/// The shape a light's entry must have: an exact split and/or a word count.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shape {
    pub enumeration: Option<String>,
    pub words: WordCount,
}

impl Shape {
    pub fn allows(&self, enumeration: &str) -> bool {
        if let Some(e) = &self.enumeration {
            if e != enumeration {
                return false;
            }
        }
        let words = enumeration.matches([',', '-']).count() + 1;
        match self.words {
            WordCount::Any => true,
            WordCount::One => words == 1,
            WordCount::Two => words == 2,
            WordCount::ThreePlus => words >= 3,
        }
    }
}

/// A split such as "4,5" or "3-4" normalised, if its parts add up to `len`
/// and it actually splits the answer. A plain length is no constraint.
pub fn shape_for(enumeration: &str, len: usize) -> Option<String> {
    let e: String = enumeration.chars().filter(|c| !c.is_whitespace()).collect();
    if !e.contains([',', '-'])
        || !e
            .chars()
            .all(|c| c.is_ascii_digit() || c == ',' || c == '-')
    {
        return None;
    }
    let parts: Option<Vec<usize>> = e
        .split([',', '-'])
        .map(|p| p.parse().ok().filter(|&n| n > 0))
        .collect();
    (parts?.iter().sum::<usize>() == len).then_some(e)
}

/// Only entries that keep every crossing fillable and match `shape`; popular
/// first, and within a band of 1,000 ranks, roomier crossings first.
pub fn safe_suggestions(
    lex: &Lexicon,
    index: &FillIndex,
    grid: &Grid,
    light_id: &str,
    shape: &Shape,
    limit: usize,
) -> Vec<Suggestion> {
    let mut out: Vec<Suggestion> = rated(
        lex,
        index,
        grid,
        light_id,
        |w| shape.allows(&w.enumeration()),
        limit * 8,
        limit,
    )
    .into_iter()
    .filter(|s| s.viable)
    .collect();
    out.sort_by_key(|s| (s.rank / 1000, std::cmp::Reverse(s.min_crossing)));
    out.truncate(limit);
    out
}

#[derive(Debug, Serialize)]
pub struct AutofillResult {
    pub ok: bool,
    pub grid: Grid,
    /// light id -> chosen entry display text (for enumerations).
    pub entries: Vec<(String, String)>,
    pub message: String,
    pub millis: u128,
}

struct Search<'a> {
    lex: &'a Lexicon,
    index: &'a FillIndex,
    lights: Vec<Light>,
    cell_lights: Vec<[Option<usize>; 2]>,
    cells: Vec<u8>,
    used: HashSet<WordId>,
    chosen: Vec<Option<WordId>>,
    deadline: Instant,
    cancel: &'a AtomicBool,
    nodes: u64,
    rng: rand::rngs::ThreadRng,
    randomness: f32,
    /// Per light: the shape its entry must have, if any.
    shapes: Vec<Option<Shape>>,
}

impl Search<'_> {
    fn open_lights(&self) -> Vec<usize> {
        (0..self.lights.len())
            .filter(|&i| self.lights[i].cells.iter().any(|&c| self.cells[c] == b'?'))
            .collect()
    }

    fn run(&mut self) -> Result<bool, &'static str> {
        self.nodes += 1;
        if self.nodes % 256 == 0 {
            if self.cancel.load(Ordering::Relaxed) {
                return Err("cancelled");
            }
            if Instant::now() > self.deadline {
                return Err("time limit reached");
            }
        }
        let open = self.open_lights();
        if open.is_empty() {
            return Ok(true);
        }
        // Most constrained light first.
        let mut best: Option<(usize, usize)> = None;
        for &li in &open {
            let n = self.index.count(&pattern_of(&self.cells, &self.lights[li]));
            if n == 0 {
                return Ok(false);
            }
            if best.is_none_or(|(_, bn)| n < bn) {
                best = Some((li, n));
            }
        }
        let (li, _) = best.unwrap();
        let light = self.lights[li].clone();
        let pattern = pattern_of(&self.cells, &light);
        let mut cands: Vec<WordId> = self
            .index
            .matches(&pattern)
            .ids()
            .filter(|id| !self.used.contains(id))
            .filter(|&id| {
                self.shapes[li]
                    .as_ref()
                    .is_none_or(|sh| sh.allows(&self.lex.word(id).enumeration()))
            })
            .take(300)
            .collect();
        // Keep popularity order roughly, with some shuffling for variety.
        if self.randomness > 0.0 {
            let chunk = (8.0 + 40.0 * self.randomness) as usize;
            for c in cands.chunks_mut(chunk) {
                c.shuffle(&mut self.rng);
            }
            if self.rng.gen::<f32>() < self.randomness * 0.3 {
                cands.shuffle(&mut self.rng);
            }
        }
        let crossing: Vec<usize> = light
            .cells
            .iter()
            .filter(|&&c| self.cells[c] == b'?')
            .filter_map(|&c| {
                let [a, d] = self.cell_lights[c];
                if light.dir == crate::grid::Dir::Across {
                    d
                } else {
                    a
                }
            })
            .collect();
        for id in cands {
            let key = self.lex.word(id).key.as_bytes().to_vec();
            for (p, &c) in light.cells.iter().enumerate() {
                self.cells[c] = key[p];
            }
            let ok = crossing
                .iter()
                .all(|&o| self.index.count(&pattern_of(&self.cells, &self.lights[o])) > 0);
            if ok {
                self.used.insert(id);
                self.chosen[li] = Some(id);
                if self.run()? {
                    return Ok(true);
                }
                self.used.remove(&id);
                self.chosen[li] = None;
            }
            for (p, &c) in light.cells.iter().enumerate() {
                self.cells[c] = pattern[p];
            }
        }
        Ok(false)
    }
}

/// Fill every empty cell. Existing letters are kept as given.
pub fn autofill(
    lex: &Lexicon,
    index: &FillIndex,
    grid: &Grid,
    time_limit: Duration,
    randomness: f32,
    cancel: &AtomicBool,
) -> AutofillResult {
    autofill_shaped(
        lex,
        index,
        grid,
        &HashMap::new(),
        time_limit,
        randomness,
        cancel,
    )
}

/// Like `autofill`, but lights listed in `shapes` (by light id) only take
/// entries of that shape, e.g. a "4,5" split.
pub fn autofill_shaped(
    lex: &Lexicon,
    index: &FillIndex,
    grid: &Grid,
    shapes: &HashMap<String, Shape>,
    time_limit: Duration,
    randomness: f32,
    cancel: &AtomicBool,
) -> AutofillResult {
    let start = Instant::now();
    let lights = grid.lights();
    let cell_lights = grid.cell_lights(&lights);
    let cells = grid_bytes(grid);
    // Words already complete in the grid count as used.
    let used: HashSet<WordId> = lights
        .iter()
        .filter_map(|l| {
            let p = pattern_of(&cells, l);
            (!p.contains(&b'?')).then(|| lex.lookup(std::str::from_utf8(&p).unwrap()))?
        })
        .collect();
    let mut s = Search {
        lex,
        index,
        chosen: vec![None; lights.len()],
        lights,
        cell_lights,
        cells,
        used,
        deadline: start + time_limit,
        cancel,
        nodes: 0,
        rng: rand::thread_rng(),
        randomness: randomness.clamp(0.0, 1.0),
        shapes: Vec::new(),
    };
    s.shapes = s
        .lights
        .iter()
        .map(|l| shapes.get(&l.id).cloned())
        .collect();
    let result = s.run();
    let mut out = grid.clone();
    let (ok, message) = match result {
        Ok(true) => (true, "Filled".to_string()),
        Ok(false) => (
            false,
            "No fill exists with the current word list and settings".to_string(),
        ),
        Err(e) => (false, format!("Stopped: {e}")),
    };
    if ok {
        for (i, c) in out.cells.iter_mut().enumerate() {
            if !c.block {
                c.letter = Some((s.cells[i] as char).to_ascii_uppercase());
            }
        }
    }
    let entries = if ok {
        s.lights
            .iter()
            .zip(&s.chosen)
            .filter_map(|(l, c)| c.map(|id| (l.id.clone(), lex.word(id).display().to_string())))
            .collect()
    } else {
        vec![]
    };
    AutofillResult {
        ok,
        grid: out,
        entries,
        message,
        millis: start.elapsed().as_millis(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordplay::testutil::lex;

    #[test]
    fn pattern_matching() {
        let idx = FillIndex::build(lex(), FillOptions::default());
        let m = idx.matches(b"c?t");
        let words: Vec<&str> = m.ids().map(|id| lex().word(id).key.as_str()).collect();
        assert!(words.contains(&"cat") && words.contains(&"cut"));
        assert!(words
            .iter()
            .all(|w| w.len() == 3 && w.starts_with('c') && w.ends_with('t')));
        assert_eq!(m.count(), words.len());
    }

    #[test]
    fn suggestions_check_crossings() {
        let idx = FillIndex::build(lex(), FillOptions::default());
        // 3x3 with a block-free corner: across 1 must cross down 1 at 'Q?' etc.
        let g = Grid::from_rows(&["...", ".#.", "..."]);
        let s = suggestions(lex(), &idx, &g, "0,0,a", 50);
        assert!(!s.is_empty());
        assert!(s.iter().any(|s| s.viable));
    }

    #[test]
    fn shape_for_validates() {
        assert_eq!(shape_for("4,5", 9), Some("4,5".to_string()));
        assert_eq!(shape_for("4,4", 9), None);
        assert_eq!(shape_for("9", 9), None);
        assert_eq!(shape_for(" 3 - 4 ", 7), Some("3-4".to_string()));
        assert_eq!(shape_for("2,1,3", 6), Some("2,1,3".to_string()));
        assert_eq!(shape_for("a,b", 2), None);
    }

    #[test]
    fn split_filter() {
        let idx = FillIndex::build(lex(), FillOptions::default());
        let g = Grid::from_rows(&["........."]);
        let id = g.lights()[0].id.clone();
        let two = Shape {
            enumeration: Some("4,5".into()),
            words: WordCount::Any,
        };
        let s = safe_suggestions(lex(), &idx, &g, &id, &two, 50);
        assert!(
            !s.is_empty() && s.iter().all(|x| x.enumeration == "4,5"),
            "{s:?}"
        );
        let s = safe_suggestions(
            lex(),
            &idx,
            &g,
            &id,
            &Shape {
                enumeration: None,
                words: WordCount::Two,
            },
            50,
        );
        assert!(
            !s.is_empty()
                && s.iter()
                    .all(|x| x.enumeration.matches([',', '-']).count() == 1)
        );
        let s = safe_suggestions(lex(), &idx, &g, &id, &Shape::default(), 50);
        assert!(s.iter().any(|x| !x.enumeration.contains([',', '-'])));
        let s = safe_suggestions(
            lex(),
            &idx,
            &g,
            &id,
            &Shape {
                enumeration: None,
                words: WordCount::One,
            },
            50,
        );
        assert!(s.iter().all(|x| !x.enumeration.contains([',', '-'])));
    }

    #[test]
    fn only_safe_words() {
        let idx = FillIndex::build(lex(), FillOptions::default());
        let g = Grid::from_rows(&["...", ".#.", "..."]);
        let s = safe_suggestions(lex(), &idx, &g, "0,0,a", &Shape::default(), 100);
        assert!(!s.is_empty() && s.iter().all(|x| x.viable));
    }

    #[test]
    fn autofill_respects_splits() {
        let idx = FillIndex::build(lex(), FillOptions::default());
        let g = Grid::from_rows(&["........."]);
        let id = g.lights()[0].id.clone();
        let shapes = std::collections::HashMap::from([(
            id.clone(),
            Shape {
                enumeration: Some("4,5".into()),
                words: WordCount::Any,
            },
        )]);
        let r = autofill_shaped(
            lex(),
            &idx,
            &g,
            &shapes,
            Duration::from_secs(5),
            0.0,
            &AtomicBool::new(false),
        );
        assert!(r.ok, "{}", r.message);
        let (_, text) = r.entries.iter().find(|(l, _)| *l == id).unwrap();
        assert_eq!(crate::text::enumeration(text), "4,5", "{text}");
    }

    #[test]
    fn impossible_fill_leaves_grid_unchanged() {
        let tiny = crate::Lexicon::from_words(&["cat"]);
        let idx = FillIndex::build(&tiny, FillOptions::default());
        let g = Grid::from_rows(&["...", "...", "..."]);
        let r = autofill_shaped(
            &tiny,
            &idx,
            &g,
            &Default::default(),
            Duration::from_secs(2),
            0.0,
            &AtomicBool::new(false),
        );
        assert!(!r.ok);
        assert_eq!(r.grid, g);
    }

    #[test]
    fn autofill_small_grid() {
        let idx = FillIndex::build(lex(), FillOptions::default());
        let g = Grid::from_rows(&["....", ".#.#", "....", ".#.#"]);
        let cancel = AtomicBool::new(false);
        let r = autofill(lex(), &idx, &g, Duration::from_secs(5), 0.0, &cancel);
        assert!(r.ok, "{}", r.message);
        for l in r.grid.lights() {
            let p = r.grid.pattern(&l);
            assert!(lex().contains(&p), "{p} not a word");
        }
    }
}
