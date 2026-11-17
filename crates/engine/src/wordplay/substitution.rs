//! Letter substitutions: words that become the target when one letter is
//! changed — "PLANE with T for E" gives PLANT.

use super::{Ctx, Piece, WordRef};
use crate::lexicon::WordId;
use crate::text;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Substitution {
    pub source: WordRef,
    pub index: usize,
    /// Letter in the source word.
    pub from: String,
    /// Letter in the target.
    pub to: String,
    pub from_as: Vec<Piece>,
    pub to_as: Vec<Piece>,
}

const MAX_RESULTS: usize = 150;
const SOURCE_LIMIT: WordId = 100_000;

pub fn find(ctx: Ctx, target: &str) -> Vec<Substitution> {
    let t = text::key(target);
    let tb = t.as_bytes();
    if t.len() < 3 {
        return vec![];
    }
    let mut out = Vec::new();
    for &id in ctx.lex.with_len(t.len()) {
        if id >= SOURCE_LIMIT || !ctx.lex.is_component(id) {
            continue;
        }
        let w = ctx.lex.word(id).key.as_bytes();
        let mut diff = None;
        let mut count = 0;
        for i in 0..tb.len() {
            if w[i] != tb[i] {
                count += 1;
                diff = Some(i);
                if count > 1 {
                    break;
                }
            }
        }
        if count != 1 {
            continue;
        }
        let i = diff.unwrap();
        let from = (w[i] as char).to_string();
        let to = (tb[i] as char).to_string();
        out.push(Substitution {
            source: ctx.word_ref(id),
            index: i,
            from_as: ctx
                .part(&from, false)
                .map(|p| p.options)
                .unwrap_or_default(),
            to_as: ctx.part(&to, false).map(|p| p.options).unwrap_or_default(),
            from,
            to,
        });
        if out.len() >= MAX_RESULTS {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordplay::testutil::ctx;

    #[test]
    fn plane_to_plant() {
        let s = find(ctx(), "plant");
        let hit = s.iter().find(|s| s.source.key == "plane").expect("plane");
        assert_eq!(
            (hit.from.as_str(), hit.to.as_str(), hit.index),
            ("e", "t", 4)
        );
    }
}
