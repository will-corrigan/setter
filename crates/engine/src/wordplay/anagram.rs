//! Anagrams: single entries (including phrases) and multi-word anagram fodder.

use super::{Ctx, WordRef};
use crate::lexicon::WordId;
use crate::text::{self, Counts};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Serialize, Default)]
pub struct Anagrams {
    /// Whole entries (words or phrases) with exactly the same letters.
    pub single: Vec<WordRef>,
    /// Two words that together use exactly the target's letters.
    pub two: Vec<Vec<WordRef>>,
    /// Three words that together use exactly the target's letters.
    pub three: Vec<Vec<WordRef>>,
}

const MAX_TWO: usize = 400;
const MAX_THREE: usize = 400;
const CAND_LIMIT_TWO: usize = 6000;
const CAND_LIMIT_THREE: usize = 1500;

pub fn find(ctx: Ctx, target: &str) -> Anagrams {
    let t = text::key(target);
    if t.len() < 2 {
        return Anagrams::default();
    }
    let lex = ctx.lex;
    let tc = text::counts(&t);
    let single = lex
        .anagrams_of_sorted(&text::sorted_key(&t))
        .iter()
        .copied()
        .filter(|&id| lex.word(id).key != t)
        .map(|id| ctx.word_ref(id))
        .collect();
    if t.len() < 3 {
        return Anagrams {
            single,
            ..Default::default()
        };
    }

    // Candidate component words that fit inside the target's letters, most popular first.
    let mut cands: Vec<(WordId, Counts)> = Vec::new();
    for len in 1..t.len() {
        for &id in lex.with_len(len) {
            if !lex.is_component(id) {
                continue;
            }
            let c = text::counts(&lex.word(id).key);
            if text::fits(&c, &tc) {
                cands.push((id, c));
            }
        }
    }
    cands.sort_by_key(|(id, _)| *id);
    let cands2 = &cands[..cands.len().min(CAND_LIMIT_TWO)];

    let mut two_set: HashSet<(WordId, WordId)> = HashSet::new();
    let mut two: Vec<(WordId, WordId)> = Vec::new();
    for &(a, ac) in cands2 {
        let rem = text::minus(&tc, &ac);
        for &b in lex.anagrams_of_sorted(&text::counts_to_sorted(&rem)) {
            if b < a || !lex.is_component(b) {
                continue;
            }
            if two_set.insert((a, b)) {
                two.push((a, b));
            }
        }
    }
    two.sort_by_key(|&(a, b)| (a.max(b), a.min(b)));
    two.truncate(MAX_TWO);

    let mut three: Vec<[WordId; 3]> = Vec::new();
    if t.len() >= 5 {
        let cands3: Vec<&(WordId, Counts)> = cands
            .iter()
            .filter(|(id, _)| lex.is_common(*id) || *id < 60_000)
            .take(CAND_LIMIT_THREE)
            .collect();
        let mut seen: HashSet<[WordId; 3]> = HashSet::new();
        for (i, &&(a, ac)) in cands3.iter().enumerate() {
            let rem = text::minus(&tc, &ac);
            if text::total(&rem) < 2 {
                continue;
            }
            for &&(b, bc) in &cands3[i..] {
                if !text::fits(&bc, &rem) {
                    continue;
                }
                let rem2 = text::minus(&rem, &bc);
                if text::total(&rem2) == 0 {
                    continue;
                }
                for &c in lex.anagrams_of_sorted(&text::counts_to_sorted(&rem2)) {
                    if c < b || !lex.is_component(c) {
                        continue;
                    }
                    let k = [a, b, c];
                    if seen.insert(k) {
                        three.push(k);
                    }
                }
            }
        }
        three.sort_by_key(|k| {
            (
                k[2].max(k[1]).max(k[0]),
                k[0] as u64 + k[1] as u64 + k[2] as u64,
            )
        });
        three.truncate(MAX_THREE);
    }

    Anagrams {
        single,
        two: two
            .into_iter()
            .map(|(a, b)| vec![ctx.word_ref(a), ctx.word_ref(b)])
            .collect(),
        three: three
            .into_iter()
            .map(|k| k.iter().map(|&i| ctx.word_ref(i)).collect())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordplay::testutil::ctx;

    fn keys(v: &[WordRef]) -> Vec<&str> {
        v.iter().map(|w| w.key.as_str()).collect()
    }

    #[test]
    fn single_anagrams() {
        let a = find(ctx(), "orchestra");
        assert!(keys(&a.single).contains(&"carthorse"));
        assert!(!keys(&a.single).contains(&"orchestra"));
    }

    #[test]
    fn multi_word_anagrams_use_exact_letters() {
        let a = find(ctx(), "dormant");
        assert!(!a.two.is_empty());
        for pair in &a.two {
            let joined: String = pair.iter().map(|w| w.key.as_str()).collect();
            assert_eq!(text::sorted_key(&joined), text::sorted_key("dormant"));
        }
        for tri in &a.three {
            let joined: String = tri.iter().map(|w| w.key.as_str()).collect();
            assert_eq!(text::sorted_key(&joined), text::sorted_key("dormant"));
        }
    }

    #[test]
    fn finds_known_two_word_anagram() {
        let a = find(ctx(), "astronomer");
        let pairs: Vec<String> = a
            .two
            .iter()
            .map(|p| format!("{} {}", p[0].key, p[1].key))
            .collect();
        assert!(
            pairs
                .iter()
                .any(|p| p == "moon starer" || p == "starer moon"),
            "astronomer pairs: {:?}",
            &pairs[..pairs.len().min(30)]
        );
    }
}
