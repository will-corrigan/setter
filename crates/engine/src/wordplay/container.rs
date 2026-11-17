//! Containers: the target read as one piece wrapped around another,
//! e.g. STARE = S(TAR)E — outer "se" (south-east), inner "tar" (sailor).

use super::{Ctx, Part};
use crate::text;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Container {
    /// The outer piece, which is split around the inner one.
    pub outer: Part,
    pub inner: Part,
    /// Where the inner piece goes within the outer piece's letters.
    pub at: usize,
    pub score: f32,
}

const MAX_RESULTS: usize = 150;

pub fn find(ctx: Ctx, target: &str) -> Vec<Container> {
    let t = text::key(target);
    let n = t.len();
    if n < 3 {
        return vec![];
    }
    let mut out = Vec::new();
    for i in 1..n - 1 {
        for j in (i + 1)..n {
            let inner_letters = &t[i..j];
            let outer_letters = format!("{}{}", &t[..i], &t[j..]);
            let Some(inner) = ctx.part(inner_letters, true) else {
                continue;
            };
            let Some(outer) = ctx.part(&outer_letters, true) else {
                continue;
            };
            // Two abbreviations nested in each other is rarely useful.
            if !Ctx::is_word_part(&inner) && !Ctx::is_word_part(&outer) {
                continue;
            }
            let score = (inner.score + outer.score) / 2.0 + 0.02 * (n.min(8) as f32);
            out.push(Container {
                outer,
                inner,
                at: i,
                score,
            });
        }
    }
    out.sort_by(|a, b| b.score.total_cmp(&a.score));
    out.truncate(MAX_RESULTS);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordplay::testutil::ctx;

    #[test]
    fn finds_classic_containers() {
        let c = find(ctx(), "stare");
        assert!(
            c.iter()
                .any(|c| c.inner.letters == "tar" && c.outer.letters == "se"),
            "{:?}",
            c.iter()
                .map(|c| (&c.outer.letters, &c.inner.letters))
                .collect::<Vec<_>>()
        );
        // every result reassembles to the target
        for x in &c {
            let mut s = x.outer.letters.clone();
            s.insert_str(x.at, &x.inner.letters);
            assert_eq!(s, "stare");
        }
    }
}
