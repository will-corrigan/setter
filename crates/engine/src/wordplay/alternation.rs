//! Alternate letters: words or phrases whose odd (or even) letters spell the
//! target — "oddly" / "evenly" / "regularly" clues.

use super::{Ctx, WordRef};
use crate::lexicon::WordId;
use crate::text;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Alternation {
    pub source: WordRef,
    /// true: letters 1, 3, 5… ("oddly"); false: letters 2, 4, 6… ("evenly").
    pub odd: bool,
}

const MAX_RESULTS: usize = 120;
const SOURCE_LIMIT: WordId = 200_000;

fn every_other(s: &[u8], from: usize) -> impl Iterator<Item = u8> + '_ {
    s.iter().skip(from).step_by(2).copied()
}

pub fn find(ctx: Ctx, target: &str) -> Vec<Alternation> {
    let t = text::key(target);
    let n = t.len();
    if n < 3 {
        return vec![];
    }
    let tb = t.as_bytes();
    let mut out: Vec<(WordId, Alternation)> = Vec::new();
    for len in [2 * n - 1, 2 * n, 2 * n + 1] {
        for &id in ctx.lex.with_len(len) {
            if id >= SOURCE_LIMIT || ctx.lex.word(id).proper {
                continue;
            }
            let w = ctx.lex.word(id).key.as_bytes();
            for (from, odd) in [(0usize, true), (1usize, false)] {
                if (len - from).div_ceil(2) == n && every_other(w, from).eq(tb.iter().copied()) {
                    out.push((
                        id,
                        Alternation {
                            source: ctx.word_ref(id),
                            odd,
                        },
                    ));
                }
            }
        }
    }
    out.sort_by_key(|(id, _)| *id);
    out.truncate(MAX_RESULTS);
    out.into_iter().map(|(_, a)| a).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordplay::testutil::ctx;

    #[test]
    fn results_spell_target() {
        let a = find(ctx(), "tea");
        assert!(!a.is_empty());
        for x in &a {
            let k = x.source.key.as_bytes();
            let got: String = every_other(k, if x.odd { 0 } else { 1 })
                .map(|b| b as char)
                .collect();
            assert_eq!(got, "tea");
        }
    }
}
