//! Hidden words: the target concealed inside a word, or running across
//! consecutive words — forwards and reversed.

use super::{Ctx, WordRef};
use crate::lexicon::WordId;
use crate::text;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Serialize)]
pub struct HiddenIn {
    pub word: WordRef,
    /// Offset of the target within `word.key`.
    pub start: usize,
}

#[derive(Debug, Serialize)]
pub struct HiddenAcross {
    pub words: Vec<WordRef>,
    /// Offset of the target within the concatenated keys.
    pub start: usize,
}

#[derive(Debug, Serialize, Default)]
pub struct Hiddens {
    pub in_word: Vec<HiddenIn>,
    pub across: Vec<HiddenAcross>,
    pub reversed_in_word: Vec<HiddenIn>,
    pub reversed_across: Vec<HiddenAcross>,
}

const PREFIXES: &[&str] = &[
    "un", "re", "de", "dis", "mis", "pre", "post", "over", "under", "out", "up", "non", "anti",
    "semi", "sub", "super", "inter", "counter", "fore", "off", "co", "ex", "in", "im", "en", "be",
];
const SUFFIXES: &[&str] = &[
    "s", "es", "ed", "d", "er", "ers", "ing", "ings", "ly", "y", "ier", "iest", "ness", "less",
    "ful", "ish", "able", "ible", "ment", "ments", "tion", "tions", "ist", "ists", "ism", "isms",
    "dom", "hood", "like", "r", "rs", "st", "est", "en", "al",
];

const MAX_IN_WORD: usize = 120;
const MAX_ACROSS: usize = 300;
const PER_SPLIT: usize = 40;
const SIDE_CANDIDATES: usize = 60;
/// Words used to carry a hidden should be reasonably everyday.
const CARRIER_LIMIT: WordId = 80_000;

pub fn find(ctx: Ctx, target: &str) -> Hiddens {
    let t = text::key(target);
    if t.len() < 3 {
        return Hiddens::default();
    }
    let r = text::reverse(&t);
    let (in_word, across) = find_one(ctx, &t);
    let (reversed_in_word, reversed_across) = if r != t {
        find_one(ctx, &r)
    } else {
        (vec![], vec![])
    };
    Hiddens {
        in_word,
        across,
        reversed_in_word,
        reversed_across,
    }
}

/// True if `t` sits in `w` merely as a stem + affix (e.g. "plant" in "planting"),
/// which makes a feeble hidden.
fn is_trivial(lex: &crate::lexicon::Lexicon, w: &str, start: usize, len: usize) -> bool {
    let left = &w[..start];
    let right = &w[start + len..];
    let left_ok =
        left.is_empty() || PREFIXES.contains(&left) || (left.len() >= 3 && lex.contains(left));
    let right_ok =
        right.is_empty() || SUFFIXES.contains(&right) || (right.len() >= 3 && lex.contains(right));
    left_ok && right_ok
}

fn find_one(ctx: Ctx, t: &str) -> (Vec<HiddenIn>, Vec<HiddenAcross>) {
    let lex = ctx.lex;
    let n = t.len();
    let mut in_word: Vec<(bool, WordId, usize)> = Vec::new();
    // ends_with[k]: words ending with t[..k]; starts_with[k]: words starting with t[k..]
    let mut ends_with: Vec<Vec<WordId>> = vec![Vec::new(); n];
    let mut starts_with: Vec<Vec<WordId>> = vec![Vec::new(); n];

    for id in 0..(lex.len() as WordId).min(CARRIER_LIMIT * 3) {
        if !lex.is_component(id) {
            continue;
        }
        let w = lex.word(id).key.as_str();
        if w.len() > n {
            if let Some(pos) = w.find(t) {
                if !is_trivial(lex, w, pos, n) {
                    let interior = pos > 0 && pos + n < w.len();
                    in_word.push((interior, id, pos));
                }
            }
        }
        if id >= CARRIER_LIMIT {
            continue;
        }
        for k in 1..n {
            if ends_with[k].len() < SIDE_CANDIDATES && w.len() > k && w.ends_with(&t[..k]) {
                ends_with[k].push(id);
            }
            if starts_with[k].len() < SIDE_CANDIDATES && w.len() > n - k && w.starts_with(&t[k..]) {
                starts_with[k].push(id);
            }
        }
    }
    in_word.sort_by_key(|&(interior, id, _)| (!interior, id));
    in_word.truncate(MAX_IN_WORD);

    let mut across: Vec<(WordId, Vec<WordId>, usize)> = Vec::new();
    let mut seen: HashSet<Vec<WordId>> = HashSet::new();
    // Two words: ...[t[..k]] [t[k..]]...
    for k in 1..n {
        let mut combos: Vec<(WordId, Vec<WordId>, usize)> = Vec::new();
        for &a in &ends_with[k] {
            for &b in &starts_with[k] {
                let a_len = lex.word(a).key.len();
                combos.push((a.max(b), vec![a, b], a_len - k));
            }
        }
        combos.sort_by_key(|c| c.0);
        for c in combos.into_iter().take(PER_SPLIT) {
            if seen.insert(c.1.clone()) {
                across.push(c);
            }
        }
    }
    // Three words: ...[t[..i]] [t[i..j]] [t[j..]]...
    for i in 1..n.saturating_sub(1) {
        for j in (i + 1)..n {
            let Some(mid) = lex.component(&t[i..j]).filter(|&m| m < CARRIER_LIMIT) else {
                continue;
            };
            let mut combos: Vec<(WordId, Vec<WordId>, usize)> = Vec::new();
            for &a in ends_with[i].iter().take(15) {
                for &b in starts_with[j].iter().take(15) {
                    let a_len = lex.word(a).key.len();
                    combos.push((a.max(b).max(mid), vec![a, mid, b], a_len - i));
                }
            }
            combos.sort_by_key(|c| c.0);
            for c in combos.into_iter().take(PER_SPLIT / 2) {
                if seen.insert(c.1.clone()) {
                    across.push(c);
                }
            }
        }
    }
    across.sort_by_key(|c| (c.0, c.1.len()));
    // Variety: don't let one very common word dominate the list.
    let mut uses: std::collections::HashMap<WordId, usize> = std::collections::HashMap::new();
    across.retain(|c| {
        let ends = [c.1[0], *c.1.last().unwrap()];
        if ends.iter().any(|w| uses.get(w).copied().unwrap_or(0) >= 3) {
            return false;
        }
        for w in ends {
            *uses.entry(w).or_default() += 1;
        }
        true
    });
    across.truncate(MAX_ACROSS);

    (
        in_word
            .into_iter()
            .map(|(_, id, start)| HiddenIn {
                word: ctx.word_ref(id),
                start,
            })
            .collect(),
        across
            .into_iter()
            .map(|(_, ids, start)| HiddenAcross {
                words: ids.into_iter().map(|i| ctx.word_ref(i)).collect(),
                start,
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordplay::testutil::ctx;

    #[test]
    fn hidden_in_single_words() {
        let h = find(ctx(), "rant");
        assert!(h.in_word.iter().any(|x| x.word.key == "frantic"));
        assert!(h.in_word.iter().any(|x| x.word.key == "tyrants"));
        for x in &h.in_word {
            assert_eq!(&x.word.key[x.start..x.start + 4], "rant");
        }
    }

    #[test]
    fn trivial_affixes_are_skipped() {
        let h = find(ctx(), "plant");
        assert!(!h
            .in_word
            .iter()
            .any(|x| x.word.key == "plants" || x.word.key == "planted"));
    }

    #[test]
    fn hidden_across_words_spells_target() {
        let h = find(ctx(), "orchestra");
        assert!(!h.across.is_empty());
        for x in h.across.iter().chain(&h.reversed_across) {
            let joined: String = x.words.iter().map(|w| w.key.as_str()).collect();
            let found = &joined[x.start..x.start + 9];
            assert!(
                found == "orchestra" || found == "artsehcro",
                "{joined} @ {}",
                x.start
            );
        }
    }
}
