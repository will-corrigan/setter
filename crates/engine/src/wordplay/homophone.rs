//! Homophones from the lexicon's CMUdict pronunciations, fully offline.
//! Finds whole-word sound-alikes and two-word sound-alikes ("isle" ~ "I'll").

use super::{Ctx, WordRef};
use crate::lexicon::WordId;
use crate::text;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Serialize, Default)]
pub struct Homophones {
    /// Pronunciations of the target, as phoneme strings.
    pub pronunciations: Vec<String>,
    pub single: Vec<WordRef>,
    /// Two words that together sound like the target.
    pub pairs: Vec<Vec<WordRef>>,
}

const MAX_PAIRS: usize = 80;
const PAIR_LIMIT: WordId = 60_000;

pub fn find(ctx: Ctx, target: &str) -> Homophones {
    let lex = ctx.lex;
    let t = text::key(target);
    let Some(id) = lex.lookup(&t) else {
        return Homophones::default();
    };
    let prons = lex.word(id).phones.clone();
    let mut single_ids: Vec<WordId> = Vec::new();
    let mut seen: HashSet<WordId> = HashSet::from([id]);
    for p in &prons {
        for &other in lex.with_phones(p) {
            if seen.insert(other) {
                single_ids.push(other);
            }
        }
    }
    single_ids.sort_unstable();

    let mut pairs: Vec<(WordId, WordId, WordId)> = Vec::new();
    let mut seen_pairs: HashSet<(WordId, WordId)> = HashSet::new();
    for p in &prons {
        let ph: Vec<&str> = p.split(' ').collect();
        for k in 1..ph.len() {
            let left = ph[..k].join(" ");
            let right = ph[k..].join(" ");
            let ls: Vec<WordId> = lex
                .with_phones(&left)
                .iter()
                .copied()
                .filter(|&w| w < PAIR_LIMIT && lex.is_component(w))
                .take(6)
                .collect();
            let rs: Vec<WordId> = lex
                .with_phones(&right)
                .iter()
                .copied()
                .filter(|&w| w < PAIR_LIMIT && lex.is_component(w))
                .take(6)
                .collect();
            for &a in &ls {
                for &b in &rs {
                    if seen_pairs.insert((a, b)) {
                        pairs.push((a.max(b), a, b));
                    }
                }
            }
        }
    }
    pairs.sort_unstable();
    pairs.truncate(MAX_PAIRS);

    Homophones {
        pronunciations: prons,
        single: single_ids.into_iter().map(|i| ctx.word_ref(i)).collect(),
        pairs: pairs
            .into_iter()
            .map(|(_, a, b)| vec![ctx.word_ref(a), ctx.word_ref(b)])
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordplay::testutil::ctx;

    #[test]
    fn knows_simple_homophones() {
        let h = find(ctx(), "knight");
        assert!(h.single.iter().any(|w| w.key == "night"));
        let h = find(ctx(), "isle");
        assert!(h.single.iter().any(|w| w.key == "aisle"));
    }

    #[test]
    fn two_word_homophones() {
        let h = find(ctx(), "tulip");
        assert!(
            h.pairs
                .iter()
                .any(|p| p[0].key == "two" || p[0].key == "to" || p[0].key == "too"),
            "{:?}",
            h.pairs
                .iter()
                .map(|p| format!("{} {}", p[0].key, p[1].key))
                .collect::<Vec<_>>()
        );
    }
}
