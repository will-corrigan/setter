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
}

impl Groups {
    fn new(word: &str, limit: usize) -> Groups {
        Groups {
            out: vec![],
            seen: HashSet::from([word.to_string()]),
            left: limit,
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
            self.out.push((name.to_string(), items));
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
        let key = word.trim().to_lowercase();
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
        let key = word.trim().to_lowercase();
        let ids = self.sense_ids(&key);
        let mut g = Groups::new(&key, limit);
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
        let key = word.trim().to_lowercase();
        if key.is_empty() {
            return vec![];
        }
        let ids = self.sense_ids(&key);
        let rel = |slot: usize| -> Vec<u32> {
            ids.iter()
                .flat_map(|&i| self.synsets[i as usize].rel[slot].clone())
                .collect()
        };
        let mut g = Groups::new(&key, limit);
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
