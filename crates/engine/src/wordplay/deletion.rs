//! Deletions: longer words that become the target when something is removed.
//! PLANT = PLAN(E)T minus E; and "anagram deletions" where the remainder is
//! only a jumble of the target.

use super::{Ctx, Piece, WordRef};
use crate::lexicon::WordId;
use crate::text;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Position {
    Start,
    End,
    Middle,
}

#[derive(Debug, Serialize)]
pub struct Deletion {
    /// The longer word to start from.
    pub source: WordRef,
    /// What is removed from it.
    pub removed: String,
    /// Offset of the removed letters within `source.key`.
    pub index: usize,
    pub position: Position,
    /// Ways to clue the removed chunk (abbreviation, word…), if any.
    pub removed_as: Vec<Piece>,
}

#[derive(Debug, Serialize)]
pub struct AnagramDeletion {
    pub source: WordRef,
    pub removed: String,
    pub index: usize,
    pub position: Position,
}

#[derive(Debug, Serialize, Default)]
pub struct Deletions {
    pub literal: Vec<Deletion>,
    pub anagram: Vec<AnagramDeletion>,
}

const MAX_LITERAL: usize = 200;
const MAX_ANAGRAM: usize = 60;
const SOURCE_LIMIT: WordId = 150_000;

fn position(index: usize, removed_len: usize, len: usize) -> Position {
    if index == 0 {
        Position::Start
    } else if index + removed_len == len {
        Position::End
    } else {
        Position::Middle
    }
}

pub fn find(ctx: Ctx, target: &str) -> Deletions {
    let t = text::key(target);
    let n = t.len();
    if n < 2 {
        return Deletions::default();
    }
    let lex = ctx.lex;

    let mut literal: Vec<(usize, Position, WordId, Deletion)> = Vec::new();
    for extra in 1..=3usize {
        for &id in lex.with_len(n + extra) {
            if id >= SOURCE_LIMIT || !lex.is_component(id) {
                continue;
            }
            let w = lex.word(id).key.as_str();
            // Find every i with w[..i] == t[..i] and w[i+extra..] == t[i..].
            let common_prefix = w.bytes().zip(t.bytes()).take_while(|(a, b)| a == b).count();
            let mut last: Option<String> = None;
            for i in 0..=common_prefix.min(n) {
                if w[i + extra..] != t[i..] {
                    continue;
                }
                let removed = &w[i..i + extra];
                // Removing "e" from "plaent" at 3 or 4 gives the same chunk; report once.
                if last.as_deref() == Some(removed) {
                    continue;
                }
                last = Some(removed.to_string());
                let removed_as = ctx
                    .part(removed, false)
                    .map(|p| p.options)
                    .unwrap_or_default();
                // Multi-letter chunks need to be clueable to be useful.
                if extra > 1 && removed_as.is_empty() {
                    continue;
                }
                let pos = position(i, extra, w.len());
                literal.push((
                    extra,
                    pos,
                    id,
                    Deletion {
                        source: ctx.word_ref(id),
                        removed: removed.to_string(),
                        index: i,
                        position: pos,
                        removed_as,
                    },
                ));
            }
        }
    }
    // Prefer single-letter, common, start/end deletions.
    literal.sort_by_key(|(extra, pos, id, _)| (!lex.is_common(*id), *extra, *pos, *id));
    literal.truncate(MAX_LITERAL);

    let mut anagram: Vec<(WordId, AnagramDeletion)> = Vec::new();
    let tc = text::counts(&t);
    for c in b'a'..=b'z' {
        let mut cc = tc;
        cc[(c - b'a') as usize] += 1;
        for &id in lex.anagrams_of_sorted(&text::counts_to_sorted(&cc)) {
            if id >= SOURCE_LIMIT || !lex.is_component(id) {
                continue;
            }
            let w = lex.word(id).key.as_str();
            for (i, b) in w.bytes().enumerate() {
                if b != c {
                    continue;
                }
                let rest = format!("{}{}", &w[..i], &w[i + 1..]);
                if rest == t {
                    continue; // that's a literal deletion
                }
                anagram.push((
                    id,
                    AnagramDeletion {
                        source: ctx.word_ref(id),
                        removed: (c as char).to_string(),
                        index: i,
                        position: position(i, 1, w.len()),
                    },
                ));
                break;
            }
        }
    }
    anagram.sort_by_key(|(id, d)| {
        (
            d.position != Position::Start && d.position != Position::End,
            *id,
        )
    });
    anagram.truncate(MAX_ANAGRAM);

    Deletions {
        literal: literal.into_iter().map(|(.., d)| d).collect(),
        anagram: anagram.into_iter().map(|(_, d)| d).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordplay::testutil::ctx;

    #[test]
    fn plant_from_planet() {
        let d = find(ctx(), "plant");
        let hit = d
            .literal
            .iter()
            .find(|d| d.source.key == "planet")
            .expect("planet");
        assert_eq!(hit.removed, "e");
        assert_eq!(hit.position, Position::Middle);
        for x in &d.literal {
            let k = &x.source.key;
            let rest = format!("{}{}", &k[..x.index], &k[x.index + x.removed.len()..]);
            assert_eq!(rest, "plant");
        }
    }

    #[test]
    fn beheadments_are_marked_start() {
        let d = find(ctx(), "rate");
        assert!(d
            .literal
            .iter()
            .any(|d| d.source.key == "crate" && d.position == Position::Start));
    }
}
