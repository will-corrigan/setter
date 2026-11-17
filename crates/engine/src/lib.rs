//! xw-engine: everything a cryptic crossword setter's tool needs that isn't UI.
//!
//! - [`lexicon`]: the popularity-ranked word list (with pronunciations)
//! - [`lists`]: abbreviations and wordplay indicators (user-extendable)
//! - [`wordplay`]: anagram, hidden, container, charade, deletion, reversal,
//!   homophone, letter-shift, substitution and alternate-letter finders
//! - [`grid`], [`fill`], [`puzzle`]: grid model, fill suggestions/autofill, save format

pub mod bank;
pub mod fill;
pub mod grid;
pub mod lexicon;
pub mod lists;
pub mod puzzle;
pub mod text;
pub mod thesaurus;
pub mod wordplay;

pub use lexicon::Lexicon;
pub use lists::{Abbreviations, Indicators};
pub use wordplay::Ctx;
