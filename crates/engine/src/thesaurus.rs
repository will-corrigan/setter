//! Offline thesaurus: Open English WordNet 2025 (senses, synonyms, broader and
//! narrower terms, parts) plus Moby Thesaurus II (wide, loose synonym lists).

use flate2::read::GzDecoder;
use std::collections::{HashMap, HashSet};
use std::io::Read;

static WORDNET: &[u8] = include_bytes!("../../../data/wordnet.tsv.gz");
static WORDNET_INDEX: &[u8] = include_bytes!("../../../data/wordnet-index.tsv.gz");
static MOBY: &[u8] = include_bytes!("../../../data/moby.tsv.gz");

/// One meaning of a word.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Sense {
    /// 'n', 'v', 'a', 's' (adjective satellite) or 'r' (adverb).
    pub pos: char,
    pub gloss: String,
    /// Synonyms, then broader terms, then narrower terms (never the word itself).
    pub words: Vec<String>,
}

// Relation slots in each synset line, in file order.
const HYPER: usize = 0;
const HYPO: usize = 1;
const PARTS: usize = 2;
const PART_OF: usize = 3;
const MEMBERS: usize = 4;
const MEMBER_OF: usize = 5;
const RELATED: usize = 6;

struct Synset {
    pos: char,
    members: Vec<String>,
    gloss: String,
    rel: [Vec<u32>; 7],
}

pub struct Thesaurus {
    synsets: Vec<Synset>,
    senses: HashMap<String, Vec<u32>>,
    /// headword -> the rest of its Moby line (split on demand; keeps memory down)
    moby: HashMap<String, String>,
}

fn gunzip(bytes: &[u8]) -> String {
    let mut s = String::new();
    GzDecoder::new(bytes)
        .read_to_string(&mut s)
        .expect("bundled thesaurus data is valid gzip");
    s
}

/// Accumulates de-duplicated words into named groups under a total limit.
struct Groups {
    out: Vec<(String, Vec<String>)>,
    seen: HashSet<String>,
    left: usize,
    /// The base word looked up instead, when the word itself isn't listed
    /// ("tree" for "trees"); the group titles name it.
    base: Option<String>,
}

impl Groups {
    fn new(word: &str, limit: usize) -> Groups {
        Groups {
            out: vec![],
            seen: HashSet::from([word.to_string()]),
            left: limit,
            base: None,
        }
    }

    fn for_base(word: &str, base: Option<&str>, limit: usize) -> Groups {
        let mut g = Groups::new(word, limit);
        if let Some(b) = base {
            g.seen.insert(b.to_string());
            g.base = Some(b.to_string());
        }
        g
    }

    fn title(&self, name: &str) -> String {
        let Some(base) = &self.base else {
            return name.to_string();
        };
        match name {
            "Synonyms" => format!("Synonyms of {base}"),
            "Broader" => format!("Broader than {base}"),
            "Related" | "Associated" => format!("{name} to {base}"),
            _ => format!("{name} ({base})"),
        }
    }

    fn add<'a>(&mut self, name: &str, words: impl IntoIterator<Item = &'a str>, cap: usize) {
        let mut items = vec![];
        for w in words {
            if self.left == 0 || items.len() >= cap {
                break;
            }
            let w = w.trim();
            if !w.is_empty() && self.seen.insert(w.to_lowercase()) {
                items.push(w.to_string());
                self.left -= 1;
            }
        }
        if !items.is_empty() {
            let title = self.title(name);
            self.out.push((title, items));
        }
    }
}

impl Thesaurus {
    pub fn bundled() -> Thesaurus {
        let mut synsets = Vec::with_capacity(110_000);
        for line in gunzip(WORDNET).lines().filter(|l| !l.starts_with('#')) {
            let f: Vec<&str> = line.split('\t').collect();
            let ids = |i: usize| -> Vec<u32> {
                f.get(i)
                    .map(|s| s.split(',').filter_map(|x| x.parse().ok()).collect())
                    .unwrap_or_default()
            };
            synsets.push(Synset {
                pos: f[0].chars().next().unwrap_or('n'),
                members: f[1].split('|').map(str::to_string).collect(),
                gloss: f.get(2).unwrap_or(&"").to_string(),
                rel: [ids(3), ids(4), ids(5), ids(6), ids(7), ids(8), ids(9)],
            });
        }
        let mut senses: HashMap<String, Vec<u32>> = HashMap::new();
        for line in gunzip(WORDNET_INDEX)
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            if let Some((lemma, ids)) = line.split_once('\t') {
                let e = senses.entry(lemma.to_lowercase()).or_default();
                for id in ids.split(',').filter_map(|x| x.parse::<u32>().ok()) {
                    if !e.contains(&id) {
                        e.push(id);
                    }
                }
            }
        }
        let mut moby = HashMap::new();
        for line in gunzip(MOBY).lines().filter(|l| !l.starts_with('#')) {
            if let Some((head, rest)) = line.split_once(',') {
                moby.insert(head.to_lowercase(), rest.to_string());
            }
        }
        Thesaurus {
            synsets,
            senses,
            moby,
        }
    }

    /// Has the thesaurus anything to offer for this exact word: another word
    /// in one of its senses, a broader term, or a Moby entry?
    fn offers(&self, key: &str) -> bool {
        self.moby.contains_key(key)
            || self.sense_ids(key).iter().any(|&i| {
                let s = &self.synsets[i as usize];
                !s.rel[HYPER].is_empty() || s.members.iter().any(|m| m.to_lowercase() != key)
            })
    }

    /// The word to look up for `word` (lower case), and the base word if it
    /// had to drop an ending or was irregular: "trees" → "tree", "carried" →
    /// "carry", "knew" → "know". A word whose own entry offers nothing falls
    /// back the same way ("directed" → "direct").
    fn lookup(&self, word: &str) -> (String, Option<String>) {
        let key = word.trim().to_lowercase();
        if self.offers(&key) {
            return (key, None);
        }
        let irregular = IRREGULAR.get(key.as_str()).map(|b| b.to_string());
        match irregular
            .into_iter()
            .chain(base_forms(&key))
            .find(|b| self.offers(b))
        {
            Some(base) => (base.clone(), Some(base)),
            None => (key, None),
        }
    }

    /// Is there anything to define this word with, by itself or by its base?
    pub fn knows(&self, word: &str) -> bool {
        let (key, _) = self.lookup(word);
        self.offers(&key)
    }

    fn sense_ids(&self, key: &str) -> &[u32] {
        self.senses.get(key).map(Vec::as_slice).unwrap_or(&[])
    }

    fn members_of<'a>(&'a self, ids: &'a [u32]) -> impl Iterator<Item = &'a str> + 'a {
        ids.iter()
            .flat_map(|&i| self.synsets[i as usize].members.iter().map(String::as_str))
    }

    fn moby_words(&self, key: &str) -> impl Iterator<Item = &str> {
        self.moby.get(key).into_iter().flat_map(|s| s.split(','))
    }

    /// The word's meanings in WordNet order (most common first), at most `max`.
    pub fn senses(&self, word: &str, max: usize) -> Vec<Sense> {
        let (key, _) = self.lookup(word);
        self.sense_ids(&key)
            .iter()
            .take(max)
            .map(|&i| {
                let s = &self.synsets[i as usize];
                let mut g = Groups::new(&key, 24);
                g.add("", s.members.iter().map(String::as_str), 24);
                g.add("", self.members_of(&s.rel[HYPER]), 8);
                g.add("", self.members_of(&s.rel[HYPO]), 12);
                Sense {
                    pos: s.pos,
                    gloss: s.gloss.clone(),
                    words: g.out.into_iter().flat_map(|(_, v)| v).collect(),
                }
            })
            .collect()
    }

    /// Words that can stand for `word`: WordNet synonyms, broader terms, then Moby.
    pub fn meanings(&self, word: &str, limit: usize) -> Vec<(String, Vec<String>)> {
        let (key, base) = self.lookup(word);
        let ids = self.sense_ids(&key);
        let mut g = Groups::for_base(&word.trim().to_lowercase(), base.as_deref(), limit);
        g.add("Synonyms", self.members_of(ids), limit);
        let hypers: Vec<u32> = ids
            .iter()
            .flat_map(|&i| self.synsets[i as usize].rel[HYPER].clone())
            .collect();
        g.add("Broader", self.members_of(&hypers), (limit / 4).max(4));
        g.add("Related", self.moby_words(&key), limit);
        g.out
    }

    /// Ideas connected to `word`, for cryptic definitions.
    pub fn linked(&self, word: &str, limit: usize) -> Vec<(String, Vec<String>)> {
        let (key, base) = self.lookup(word);
        if key.is_empty() {
            return vec![];
        }
        let ids = self.sense_ids(&key);
        let rel = |slot: usize| -> Vec<u32> {
            ids.iter()
                .flat_map(|&i| self.synsets[i as usize].rel[slot].clone())
                .collect()
        };
        let mut g = Groups::for_base(&word.trim().to_lowercase(), base.as_deref(), limit);
        for (name, slot) in [
            ("Kinds", HYPO),
            ("Is a kind of", HYPER),
            ("Parts", PARTS),
            ("Part of", PART_OF),
            ("Members", MEMBERS),
            ("Member of", MEMBER_OF),
            ("Related", RELATED),
        ] {
            let r = rel(slot);
            g.add(name, self.members_of(&r), 16);
        }
        g.add("Associated", self.moby_words(&key), limit);
        g.out
    }
}

/// Irregular forms and their base words (data/irregular.tsv).
static IRREGULAR: std::sync::LazyLock<HashMap<&'static str, &'static str>> =
    std::sync::LazyLock::new(|| {
        include_str!("../../../data/irregular.tsv")
            .lines()
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| l.split_once('\t'))
            .collect()
    });

/// Possible base words for an inflected word, most likely first: plurals
/// and -s verbs, then -ed, then -ing. Only regular endings.
fn base_forms(word: &str) -> Vec<String> {
    let mut out = vec![];
    let cut = |n: usize| word[..word.len() - n].to_string();
    // "stopp" → "stop"
    let undouble = |stem: &str| {
        let b = stem.as_bytes();
        (b.len() >= 3 && b[b.len() - 1] == b[b.len() - 2])
            .then(|| stem[..stem.len() - 1].to_string())
    };
    if !word.is_ascii() {
        return out;
    }
    if word.ends_with("ies") {
        out.push(format!("{}y", cut(3)));
    }
    if word.ends_with('s') && !word.ends_with("ss") {
        out.push(cut(1));
    }
    if word.ends_with("es") {
        out.push(cut(2));
    }
    if word.ends_with("ied") {
        out.push(format!("{}y", cut(3)));
    }
    if word.ends_with("ed") {
        out.push(cut(1));
        out.push(cut(2));
        out.extend(undouble(&cut(2)));
    }
    if word.ends_with("ing") {
        out.push(cut(3));
        out.push(format!("{}e", cut(3)));
        out.extend(undouble(&cut(3)));
    }
    // darker, happier, larger, bigger; deepest, happiest, largest, biggest
    for (ending, n) in [("er", 2), ("est", 3)] {
        if word.ends_with(ending) {
            let stem = cut(n);
            if let Some(y) = stem.strip_suffix('i') {
                out.push(format!("{y}y"));
            }
            out.push(stem.clone());
            out.push(format!("{stem}e"));
            out.extend(undouble(&stem));
        }
    }
    // typically, dramatically, happily, gently
    if word.ends_with("ly") {
        out.push(cut(2));
        if word.ends_with("ily") {
            out.push(format!("{}y", cut(3)));
        }
        if word.ends_with("ally") {
            out.push(cut(4));
        }
        out.push(format!("{}e", cut(1)));
    }
    out.retain(|b| b.len() >= 3);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    fn th() -> &'static Thesaurus {
        static T: OnceLock<Thesaurus> = OnceLock::new();
        T.get_or_init(Thesaurus::bundled)
    }

    fn flat(groups: &[(String, Vec<String>)]) -> Vec<&str> {
        groups
            .iter()
            .flat_map(|(_, v)| v.iter().map(String::as_str))
            .collect()
    }

    #[test]
    fn chest_meanings_are_broad() {
        let g = th().meanings("chest", 80);
        assert_eq!(g[0].0, "Synonyms");
        let all = flat(&g);
        for w in ["coffer", "box", "thorax"] {
            assert!(all.contains(&w), "{w} missing from {all:?}");
        }
        assert!(!all.contains(&"chest"));
        assert!(all.len() <= 80);
    }

    #[test]
    fn inflected_words_fall_back_to_their_base_word() {
        for (word, base) in [
            ("trees", "tree"),
            ("boxes", "box"),
            ("cherries", "cherry"),
            ("dodged", "dodge"),
            ("jumped", "jump"),
            ("carried", "carry"),
            ("grabbed", "grab"),
            ("grabbing", "grab"),
            ("dodgers", "dodger"),
        ] {
            let g = th().meanings(word, 80);
            assert!(!g.is_empty(), "{word}: nothing");
            assert!(
                g.iter().all(|(title, _)| title.ends_with(base)),
                "{word}: {:?}",
                g.iter().map(|(t, _)| t).collect::<Vec<_>>()
            );
        }
        let g = th().meanings("trees", 80);
        assert_eq!(g[0].0, "Synonyms of tree");
        assert!(!th().linked("orchestras", 80).is_empty());
        assert!(!th().senses("pitches", 6).is_empty());
    }

    #[test]
    fn irregular_forms_use_their_base_word() {
        for (word, base) in [
            ("knew", "know"),
            ("feet", "foot"),
            ("began", "begin"),
            ("mice", "mouse"),
        ] {
            let g = th().meanings(word, 80);
            assert!(!g.is_empty(), "{word}: nothing");
            assert!(
                g.iter().all(|(t, _)| t.ends_with(base)),
                "{word}: {:?}",
                g.iter().map(|(t, _)| t).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn comparatives_and_adverbs_use_their_base_word() {
        for (word, base) in [
            ("darker", "dark"),
            ("deepest", "deep"),
            ("biggest", "big"),
            ("happier", "happy"),
            ("typically", "typical"),
            ("dramatically", "dramatic"),
        ] {
            let g = th().meanings(word, 80);
            assert!(!g.is_empty(), "{word}: nothing");
            assert!(
                g.iter().all(|(t, _)| t.ends_with(base)),
                "{word}: {:?}",
                g.iter().map(|(t, _)| t).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn an_entry_with_nothing_to_offer_falls_back_too() {
        // WordNet lists "directed" only as an adjective with no other words.
        let g = th().meanings("directed", 80);
        assert!(!g.is_empty());
        assert!(
            g.iter().all(|(t, _)| t.ends_with("direct")),
            "{:?}",
            g.iter().map(|(t, _)| t).collect::<Vec<_>>()
        );
    }

    #[test]
    fn knows_says_whether_anything_can_define_a_word() {
        assert!(th().knows("trees"));
        assert!(th().knows("knew"));
        assert!(!th().knows("whoom"));
        assert!(!th().knows("as to"));
    }

    #[test]
    fn words_with_their_own_entry_keep_it() {
        // "dodging" is a noun in its own right.
        let g = th().meanings("dodging", 80);
        assert_eq!(g[0].0, "Synonyms");
        assert!(flat(&g).contains(&"evasion"));
    }

    #[test]
    fn pitch_has_separate_senses() {
        let s = th().senses("pitch", 30);
        assert!(s.len() >= 10);
        assert!(s.iter().any(|x| x.words.iter().any(|w| w == "tar")));
        assert!(s.iter().any(|x| x.words.iter().any(|w| w == "toss")));
        assert!(s.iter().all(|x| !x.words.iter().any(|w| w == "pitch")));
        assert_eq!(th().senses("pitch", 6).len(), 6);
    }

    #[test]
    fn orchestra_links() {
        let g = th().linked("orchestra", 80);
        let get = |name: &str| {
            g.iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        assert!(get("Parts").contains(&"section".to_string()));
        assert!(get("Kinds").contains(&"symphony orchestra".to_string()));
        assert!(g.iter().all(|(_, v)| !v.is_empty()));
    }

    #[test]
    fn unknown_words_are_empty() {
        assert!(th().meanings("zzqxv", 10).is_empty());
        assert!(th().senses("zzqxv", 5).is_empty());
        assert!(th().linked("", 5).is_empty());
    }
}
