//! The word list. Entries are deduplicated by key (letters only), and word ids
//! are assigned in popularity order: a lower id is a more common word, so
//! sorting by id sorts by popularity.

use crate::text;
use flate2::read::GzDecoder;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read};

pub type WordId = u32;

/// The bundled Lufz lexicon (UKACD-derived, popularity ordered, with CMUdict phones).
pub static BUNDLED_LEXICON: &[u8] = include_bytes!("../../../data/lexicon.tsv.gz");

#[derive(Debug, Clone)]
pub struct Word {
    /// Letters-only lowercase key, e.g. "itis".
    pub key: String,
    /// Display forms as found in the list, e.g. ["it is"], ["And", "and"].
    pub forms: Vec<String>,
    /// CMUdict-style phoneme strings, e.g. ["AE N D", "AH N D"].
    pub phones: Vec<String>,
    /// Only ever appears capitalised (a proper noun).
    pub proper: bool,
    /// Only ever appears as a multi-word / hyphenated phrase.
    pub phrase: bool,
}

impl Word {
    /// The nicest form to show: prefer lowercase, single-word forms.
    pub fn display(&self) -> &str {
        self.forms
            .iter()
            .find(|f| f.chars().next().is_some_and(|c| c.is_lowercase()))
            .unwrap_or(&self.forms[0])
    }

    pub fn enumeration(&self) -> String {
        text::enumeration(self.display())
    }
}

/// One raw line of input before deduplication.
pub struct RawEntry {
    pub text: String,
    pub phones: Vec<String>,
}

pub struct Lexicon {
    pub words: Vec<Word>,
    by_key: HashMap<String, WordId>,
    anagrams: HashMap<String, Vec<WordId>>,
    by_len: Vec<Vec<WordId>>,
    by_phones: HashMap<String, Vec<WordId>>,
    /// Ids below this are considered "common" words.
    pub common_cutoff: WordId,
}

pub const DEFAULT_COMMON_CUTOFF: WordId = 40_000;

/// Genuine two-letter words. The list has many two-letter oddments ("st", "es")
/// that make poor wordplay pieces; abbreviations cover those instead.
pub const TWO_LETTER_WORDS: &[&str] = &[
    "ad", "ah", "am", "an", "as", "at", "aw", "ax", "be", "by", "do", "eh", "go", "ha", "he", "hi",
    "ho", "id", "if", "in", "is", "it", "lo", "ma", "me", "my", "no", "of", "oh", "ok", "on", "or",
    "ow", "ox", "pa", "pi", "so", "to", "uh", "um", "up", "us", "we", "ye", "yo",
];

impl Lexicon {
    /// Load the bundled lexicon, with optional user additions (placed first, so
    /// treated as most popular) and bans.
    pub fn bundled(extra: &[String], banned: &HashSet<String>) -> Lexicon {
        let mut raw: Vec<RawEntry> = extra
            .iter()
            .map(|w| RawEntry {
                text: w.trim().to_string(),
                phones: vec![],
            })
            .collect();
        raw.extend(parse_tsv(GzDecoder::new(BUNDLED_LEXICON)));
        Lexicon::build(raw, banned)
    }

    pub fn from_words<S: AsRef<str>>(words: &[S]) -> Lexicon {
        let raw = words
            .iter()
            .map(|w| RawEntry {
                text: w.as_ref().to_string(),
                phones: vec![],
            })
            .collect();
        Lexicon::build(raw, &HashSet::new())
    }

    pub fn build(raw: Vec<RawEntry>, banned: &HashSet<String>) -> Lexicon {
        let mut words: Vec<Word> = Vec::with_capacity(raw.len());
        let mut by_key: HashMap<String, WordId> = HashMap::with_capacity(raw.len());
        for e in raw {
            let key = text::key(&e.text);
            if key.is_empty() || banned.contains(&key) || banned.contains(&e.text.to_lowercase()) {
                continue;
            }
            let is_phrase = e.text.contains(' ') || e.text.contains('-');
            let is_proper = e.text.chars().next().is_some_and(|c| c.is_uppercase());
            match by_key.get(&key) {
                Some(&id) => {
                    let w = &mut words[id as usize];
                    if !w.forms.contains(&e.text) {
                        w.forms.push(e.text);
                    }
                    w.proper &= is_proper;
                    w.phrase &= is_phrase;
                    for p in e.phones {
                        if !w.phones.contains(&p) {
                            w.phones.push(p);
                        }
                    }
                }
                None => {
                    by_key.insert(key.clone(), words.len() as WordId);
                    words.push(Word {
                        key,
                        forms: vec![e.text],
                        phones: e.phones,
                        proper: is_proper,
                        phrase: is_phrase,
                    });
                }
            }
        }

        let mut anagrams: HashMap<String, Vec<WordId>> = HashMap::new();
        let mut by_len: Vec<Vec<WordId>> = Vec::new();
        let mut by_phones: HashMap<String, Vec<WordId>> = HashMap::new();
        for (i, w) in words.iter().enumerate() {
            let id = i as WordId;
            anagrams
                .entry(text::sorted_key(&w.key))
                .or_default()
                .push(id);
            let n = w.key.len();
            if by_len.len() <= n {
                by_len.resize_with(n + 1, Vec::new);
            }
            by_len[n].push(id);
            for p in &w.phones {
                by_phones.entry(p.clone()).or_default().push(id);
            }
        }

        Lexicon {
            words,
            by_key,
            anagrams,
            by_len,
            by_phones,
            common_cutoff: DEFAULT_COMMON_CUTOFF,
        }
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    pub fn word(&self, id: WordId) -> &Word {
        &self.words[id as usize]
    }

    pub fn lookup(&self, key: &str) -> Option<WordId> {
        self.by_key.get(key).copied()
    }

    pub fn contains(&self, key: &str) -> bool {
        self.by_key.contains_key(key)
    }

    pub fn is_common(&self, id: WordId) -> bool {
        id < self.common_cutoff
    }

    /// Ids of all entries that are exact anagrams of the given sorted key.
    pub fn anagrams_of_sorted(&self, sorted: &str) -> &[WordId] {
        self.anagrams.get(sorted).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn with_len(&self, n: usize) -> &[WordId] {
        self.by_len.get(n).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn max_len(&self) -> usize {
        self.by_len.len().saturating_sub(1)
    }

    pub fn with_phones(&self, phones: &str) -> &[WordId] {
        self.by_phones.get(phones).map(Vec::as_slice).unwrap_or(&[])
    }

    /// A word that is reasonable to use as a building block of wordplay:
    /// a single word, not a proper noun, and not a stray single letter.
    pub fn is_component(&self, id: WordId) -> bool {
        let w = self.word(id);
        if w.phrase || w.proper || w.forms.iter().all(|f| f.contains('\'')) {
            return false;
        }
        match w.key.len() {
            1 => matches!(w.key.as_str(), "a" | "i" | "o"),
            2 => TWO_LETTER_WORDS.contains(&w.key.as_str()),
            _ => true,
        }
    }

    /// Component lookup for a key.
    pub fn component(&self, key: &str) -> Option<WordId> {
        self.lookup(key).filter(|&id| self.is_component(id))
    }
}

pub fn parse_tsv<R: Read>(r: R) -> impl Iterator<Item = RawEntry> {
    BufReader::new(r)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| {
            if line.starts_with('#') || line.trim().is_empty() {
                return None;
            }
            let mut parts = line.splitn(2, '\t');
            let text = parts.next()?.to_string();
            let phones = parts
                .next()
                .unwrap_or("")
                .split('|')
                .filter(|p| !p.is_empty())
                .map(str::to_string)
                .collect();
            Some(RawEntry { text, phones })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupes_by_key_and_keeps_popularity_order() {
        let lex = Lexicon::from_words(&["the", "And", "and", "it is", "London", "itis"]);
        assert_eq!(lex.len(), 4);
        let and = lex.lookup("and").unwrap();
        assert_eq!(lex.word(and).forms, vec!["And", "and"]);
        assert!(!lex.word(and).proper);
        assert_eq!(lex.word(and).display(), "and");
        let itis = lex.lookup("itis").unwrap();
        assert!(!lex.word(itis).phrase, "also seen as a single word");
        assert!(lex.word(lex.lookup("london").unwrap()).proper);
        assert!(lex.lookup("the").unwrap() < and);
    }

    #[test]
    fn bundled_lexicon_loads() {
        let lex = Lexicon::bundled(&[], &HashSet::new());
        assert!(lex.len() > 250_000);
        let o = lex.lookup("orchestra").expect("orchestra");
        let anas: Vec<&str> = lex
            .anagrams_of_sorted(&text::sorted_key("orchestra"))
            .iter()
            .map(|&i| lex.word(i).key.as_str())
            .collect();
        assert!(anas.contains(&"carthorse"));
        assert!(!lex.word(o).phones.is_empty());
    }
}
