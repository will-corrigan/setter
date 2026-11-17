//! Wordplay finders: given a target answer, find the raw material for clues.
//! Each finder returns plain, serialisable data; nothing here writes clues.

pub mod alternation;
pub mod anagram;
pub mod charade;
pub mod container;
pub mod deletion;
pub mod hidden;
pub mod homophone;
pub mod reversal;
pub mod shift;
pub mod substitution;

use crate::lexicon::{Lexicon, WordId};
use crate::lists::Abbreviations;
use serde::Serialize;

/// What every finder needs.
#[derive(Clone, Copy)]
pub struct Ctx<'a> {
    pub lex: &'a Lexicon,
    pub abbr: &'a Abbreviations,
}

/// A word from the lexicon, as shown to the user.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WordRef {
    pub text: String,
    pub key: String,
    /// Popularity rank (1 = most common).
    pub rank: u32,
    pub common: bool,
}

/// One way of cluing a chunk of letters.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Piece {
    /// The chunk is itself a word: clue it with a synonym.
    Word { word: WordRef },
    /// The chunk is a standard abbreviation.
    Abbrev { meanings: Vec<String> },
    /// The chunk is a word written backwards.
    Reversed { word: WordRef },
}

/// A chunk of letters together with the ways it could be clued (best first).
#[derive(Debug, Clone, Serialize)]
pub struct Part {
    pub letters: String,
    pub options: Vec<Piece>,
    #[serde(skip)]
    pub score: f32,
}

/// How obscure a piece word may be before we stop suggesting it.
const PIECE_RANK_LIMIT: WordId = 120_000;
/// Short pieces must be everyday words ("ian", "che" are not).
const SHORT_PIECE_RANK_LIMIT: WordId = 25_000;
const REVERSED_RANK_LIMIT: WordId = 60_000;

impl<'a> Ctx<'a> {
    pub fn new(lex: &'a Lexicon, abbr: &'a Abbreviations) -> Self {
        Ctx { lex, abbr }
    }

    pub fn word_ref(&self, id: WordId) -> WordRef {
        let w = self.lex.word(id);
        WordRef {
            text: w.display().to_string(),
            key: w.key.clone(),
            rank: id + 1,
            common: self.lex.is_common(id),
        }
    }

    /// 0..1, higher for more popular words.
    pub fn word_score(&self, id: WordId) -> f32 {
        let s = 1.0 - ((id as f32 + 2.0).ln() / 400_000f32.ln());
        s.clamp(0.02, 1.0)
    }

    /// All the ways to clue `letters` as one unit, best first, with a score.
    pub fn part(&self, letters: &str, allow_reversal: bool) -> Option<Part> {
        let mut scored: Vec<(f32, Piece)> = Vec::new();
        let limit = if letters.len() <= 3 {
            SHORT_PIECE_RANK_LIMIT
        } else {
            PIECE_RANK_LIMIT
        };
        if let Some(id) = self.lex.component(letters).filter(|&id| id < limit) {
            scored.push((
                self.word_score(id) + 0.15,
                Piece::Word {
                    word: self.word_ref(id),
                },
            ));
        }
        if let Some(m) = self.abbr.get(letters) {
            let s = if letters.len() == 1 { 0.55 } else { 0.6 };
            scored.push((
                s,
                Piece::Abbrev {
                    meanings: m.to_vec(),
                },
            ));
        }
        if allow_reversal && letters.len() >= 2 {
            let rev = crate::text::reverse(letters);
            if rev != letters {
                if let Some(id) = self
                    .lex
                    .component(&rev)
                    .filter(|&id| id < REVERSED_RANK_LIMIT)
                {
                    scored.push((
                        self.word_score(id) * 0.7,
                        Piece::Reversed {
                            word: self.word_ref(id),
                        },
                    ));
                }
            }
        }
        if scored.is_empty() {
            return None;
        }
        scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        Some(Part {
            letters: letters.to_string(),
            score: scored[0].0,
            options: scored.into_iter().map(|(_, p)| p).collect(),
        })
    }

    pub fn is_word_part(part: &Part) -> bool {
        matches!(part.options.first(), Some(Piece::Word { .. }))
    }
}

/// Normalise user input into a key, rejecting empty input.
pub fn target_key(s: &str) -> String {
    crate::text::key(s)
}

#[cfg(test)]
pub(crate) mod testutil {
    use crate::lexicon::Lexicon;
    use crate::lists::Abbreviations;
    use std::collections::HashSet;
    use std::sync::OnceLock;

    pub fn lex() -> &'static Lexicon {
        static L: OnceLock<Lexicon> = OnceLock::new();
        L.get_or_init(|| Lexicon::bundled(&[], &HashSet::new()))
    }

    pub fn abbr() -> &'static Abbreviations {
        static A: OnceLock<Abbreviations> = OnceLock::new();
        A.get_or_init(Abbreviations::bundled)
    }

    pub fn ctx() -> super::Ctx<'static> {
        super::Ctx::new(lex(), abbr())
    }
}
