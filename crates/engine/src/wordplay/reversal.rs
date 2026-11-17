//! Reversals: the target read backwards.

use super::{Ctx, WordRef};
use crate::text;
use serde::Serialize;

#[derive(Debug, Serialize, Default)]
pub struct Reversal {
    pub reversed: String,
    pub palindrome: bool,
    /// Lexicon entries spelling the reversed letters (may be a phrase).
    pub words: Vec<WordRef>,
}

pub fn find(ctx: Ctx, target: &str) -> Reversal {
    let t = text::key(target);
    let r = text::reverse(&t);
    let palindrome = !t.is_empty() && r == t;
    let words = if palindrome {
        vec![]
    } else {
        ctx.lex
            .lookup(&r)
            .map(|id| vec![ctx.word_ref(id)])
            .unwrap_or_default()
    };
    Reversal {
        reversed: r,
        palindrome,
        words,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordplay::testutil::ctx;

    #[test]
    fn reversals() {
        assert_eq!(find(ctx(), "stressed").words[0].key, "desserts");
        assert!(find(ctx(), "level").palindrome);
    }
}
