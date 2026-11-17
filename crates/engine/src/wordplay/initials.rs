//! Everyday words by first or last letter, for first-letter (acrostic) and
//! last-letter clues.

use super::{Ctx, WordRef};
use crate::lexicon::WordId;

fn scan(ctx: Ctx, limit: usize, keep: impl Fn(&str) -> bool) -> Vec<WordRef> {
    (0..ctx.lex.common_cutoff.min(ctx.lex.len() as WordId))
        .filter(|&id| ctx.lex.is_component(id) && keep(&ctx.lex.word(id).key))
        .take(limit)
        .map(|id| ctx.word_ref(id))
        .collect()
}

/// Common single words starting with `letter`, most popular first.
pub fn starting_with(ctx: Ctx, letter: char, limit: usize) -> Vec<WordRef> {
    let l = letter.to_ascii_lowercase();
    scan(ctx, limit, |k| k.starts_with(l))
}

/// Common single words ending with `letter`, most popular first.
pub fn ending_with(ctx: Ctx, letter: char, limit: usize) -> Vec<WordRef> {
    let l = letter.to_ascii_lowercase();
    scan(ctx, limit, |k| k.ends_with(l))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordplay::testutil::ctx;

    #[test]
    fn initials() {
        let t = starting_with(ctx(), 't', 40);
        assert_eq!(t.len(), 40);
        assert!(t.iter().all(|w| w.key.starts_with('t')));
        assert!(t.iter().any(|w| w.key == "the"));
        assert!(ending_with(ctx(), 'o', 20)
            .iter()
            .all(|w| w.key.ends_with('o')));
        assert!(starting_with(ctx(), '7', 10).is_empty());
    }
}
