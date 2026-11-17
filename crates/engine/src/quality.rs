//! How good each word is as grid fill (data/quality.tsv.gz, built by
//! scripts/build_quality.py from the hand-scored Nediger list, adjusted for
//! British spellings and partial phrases).

use crate::lexicon::{Lexicon, WordId};
use crate::text;
use flate2::read::GzDecoder;
use std::collections::HashMap;
use std::io::Read;

static BUNDLED_QUALITY: &[u8] = include_bytes!("../../../data/quality.tsv.gz");

/// Best first, so `Good < Fair < Rare`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Quality {
    /// A setter would happily put it in a grid.
    Good,
    /// Fine when needed: odd but fair, or common but unscored.
    Fair,
    /// Obscure, racy, or a fragment like "as to": a last resort.
    Rare,
}

/// A quality for every word in a lexicon, by word id.
#[derive(Clone, Debug, Default)]
pub struct Qualities(Vec<Quality>);

impl Qualities {
    /// The bundled scores for `lex`. The setter's own words are always good;
    /// words the data doesn't know are rare.
    pub fn bundled(lex: &Lexicon, own: &[String]) -> Qualities {
        let mut data = String::new();
        let _ = GzDecoder::new(BUNDLED_QUALITY).read_to_string(&mut data);
        let mut by_key: HashMap<&str, Quality> = HashMap::new();
        for line in data.lines().filter(|l| !l.starts_with('#')) {
            if let Some((key, class)) = line.split_once('\t') {
                let q = match class {
                    "g" => Quality::Good,
                    "f" => Quality::Fair,
                    _ => Quality::Rare,
                };
                by_key.insert(key, q);
            }
        }
        let mut out: Vec<Quality> = lex
            .words
            .iter()
            .map(|w| by_key.get(w.key.as_str()).copied().unwrap_or(Quality::Rare))
            .collect();
        for w in own {
            if let Some(id) = lex.lookup(&text::key(w)) {
                out[id as usize] = Quality::Good;
            }
        }
        Qualities(out)
    }

    /// Given qualities, one per word id (for tests and custom lists).
    pub fn from_vec(qualities: Vec<Quality>) -> Qualities {
        Qualities(qualities)
    }

    pub fn of(&self, id: WordId) -> Quality {
        self.0.get(id as usize).copied().unwrap_or(Quality::Rare)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::OnceLock;

    fn bundled() -> &'static (Lexicon, Qualities) {
        static B: OnceLock<(Lexicon, Qualities)> = OnceLock::new();
        B.get_or_init(|| {
            let own = vec!["zobrist".to_string()];
            let lex = Lexicon::bundled(&own, &HashSet::new());
            let q = Qualities::bundled(&lex, &own);
            (lex, q)
        })
    }

    fn quality(word: &str) -> Quality {
        let (lex, q) = bundled();
        q.of(lex
            .lookup(&text::key(word))
            .unwrap_or_else(|| panic!("{word} not in the list")))
    }

    #[test]
    fn junk_and_fragments_are_rare() {
        for w in ["embus", "whoom", "nemesia", "as to", "for her", "the same"] {
            assert_eq!(quality(w), Quality::Rare, "{w}");
        }
    }

    #[test]
    fn british_spellings_and_staples_are_good() {
        for w in [
            "colour",
            "centre",
            "realise",
            "programme",
            "kerb",
            "grey",
            "cricket",
            "oboe",
            "whilst",
            "small",
        ] {
            assert_eq!(quality(w), Quality::Good, "{w}");
        }
    }

    #[test]
    fn the_setter_s_own_words_are_good() {
        assert_eq!(quality("zobrist"), Quality::Good);
    }
}
