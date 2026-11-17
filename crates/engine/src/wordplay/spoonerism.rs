//! Spoonerisms: two words whose first sounds swap to give the answer,
//! e.g. FLUTTER BY → BUTTER FLY (BUTTERFLY). Spelling-based for now.

use super::{Ctx, WordRef};
use crate::lexicon::WordId;
use crate::text;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Spoonerism {
    /// The answer split into the two words that get swapped back, e.g. ["butter", "fly"].
    pub answer_parts: [String; 2],
    /// The phrase the solver hears, e.g. FLUTTER BY.
    pub from: [WordRef; 2],
}

const MAX_RESULTS: usize = 60;

/// Leading consonant cluster. "qu" counts as a cluster; "y" is a vowel unless it
/// starts the word.
fn onset(w: &str) -> &str {
    if w.starts_with("qu") {
        return &w[..2];
    }
    let end = w
        .char_indices()
        .find(|&(i, c)| "aeiou".contains(c) || (c == 'y' && i > 0))
        .map(|(i, _)| i)
        .unwrap_or(w.len());
    &w[..end]
}

pub fn find(ctx: Ctx, target: &str) -> Vec<Spoonerism> {
    let t = text::key(target);
    let lex = ctx.lex;
    let mut out: Vec<(WordId, Spoonerism)> = Vec::new();
    for k in 2..t.len().saturating_sub(1) {
        let (x, y) = (&t[..k], &t[k..]);
        if lex.component(x).is_none() || lex.component(y).is_none() {
            continue;
        }
        let (ox, oy) = (onset(x), onset(y));
        if ox.is_empty() || oy.is_empty() || ox.len() == x.len() || oy.len() == y.len() {
            continue;
        }
        let a = format!("{oy}{}", &x[ox.len()..]);
        let b = format!("{ox}{}", &y[oy.len()..]);
        if a == x {
            continue;
        }
        if let (Some(ia), Some(ib)) = (lex.component(&a), lex.component(&b)) {
            out.push((
                ia.max(ib),
                Spoonerism {
                    answer_parts: [x.to_string(), y.to_string()],
                    from: [ctx.word_ref(ia), ctx.word_ref(ib)],
                },
            ));
        }
    }
    out.sort_by_key(|(r, _)| *r);
    out.truncate(MAX_RESULTS);
    out.into_iter().map(|(_, s)| s).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordplay::testutil::ctx;

    #[test]
    fn butterfly_is_flutter_by() {
        let s = find(ctx(), "butterfly");
        assert!(s.iter().any(|s| s.from[0].key == "flutter"
            && s.from[1].key == "by"
            && s.answer_parts == ["butter".to_string(), "fly".to_string()]));
    }

    #[test]
    fn short_or_vowel_only_returns_empty() {
        assert!(find(ctx(), "an").is_empty());
        assert!(find(ctx(), "aeiou").is_empty());
    }
}
