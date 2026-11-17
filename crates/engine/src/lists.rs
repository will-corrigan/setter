//! Abbreviations and indicator lists: bundled TOML defaults, merged with the
//! user's own files.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub static BUNDLED_ABBREVIATIONS: &str = include_str!("../../../data/abbreviations.toml");
pub static BUNDLED_INDICATORS: &str = include_str!("../../../data/indicators.toml");

#[derive(Debug, Clone, Default)]
pub struct Abbreviations {
    /// letters -> meanings
    pub map: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize)]
struct AbbrevFile {
    #[serde(default)]
    abbreviations: BTreeMap<String, Vec<String>>,
}

impl Abbreviations {
    pub fn parse(src: &str) -> Result<Abbreviations, String> {
        let f: AbbrevFile = toml::from_str(src).map_err(|e| e.to_string())?;
        let mut a = Abbreviations::default();
        a.merge(f.abbreviations);
        Ok(a)
    }

    pub fn bundled() -> Abbreviations {
        Abbreviations::parse(BUNDLED_ABBREVIATIONS).expect("bundled abbreviations.toml is valid")
    }

    /// Merge a user file: entries extend the bundled meanings; an empty list removes the entry.
    pub fn merge_toml(&mut self, src: &str) -> Result<(), String> {
        let f: AbbrevFile = toml::from_str(src).map_err(|e| e.to_string())?;
        self.merge(f.abbreviations);
        Ok(())
    }

    fn merge(&mut self, entries: BTreeMap<String, Vec<String>>) {
        for (k, v) in entries {
            let key = crate::text::key(&k);
            if key.is_empty() {
                continue;
            }
            if v.is_empty() {
                self.map.remove(&key);
                continue;
            }
            let e = self.map.entry(key).or_default();
            for m in v {
                if !e.iter().any(|x| x.eq_ignore_ascii_case(&m)) {
                    e.push(m);
                }
            }
        }
    }

    pub fn get(&self, letters: &str) -> Option<&[String]> {
        self.map.get(letters).map(Vec::as_slice)
    }

    pub fn contains(&self, letters: &str) -> bool {
        self.map.contains_key(letters)
    }

    /// Reverse lookup: which abbreviations can a word/phrase clue? ("doctor" -> DR, MO, MD, GP, MB…)
    pub fn for_meaning(&self, meaning: &str) -> Vec<(String, String)> {
        let q = meaning.trim().to_lowercase();
        if q.is_empty() {
            return vec![];
        }
        let mut out = Vec::new();
        for (k, v) in &self.map {
            for m in v {
                let ml = m.to_lowercase();
                if ml == q || ml.split(|c: char| !c.is_alphanumeric()).any(|w| w == q) {
                    out.push((k.clone(), m.clone()));
                    break;
                }
            }
        }
        out.sort_by_key(|(k, _)| k.len());
        out
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorList {
    pub label: String,
    pub words: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Indicators {
    pub lists: BTreeMap<String, IndicatorList>,
}

impl Indicators {
    pub fn parse(src: &str) -> Result<Indicators, String> {
        let lists: BTreeMap<String, IndicatorList> =
            toml::from_str(src).map_err(|e| e.to_string())?;
        Ok(Indicators { lists })
    }

    pub fn bundled() -> Indicators {
        Indicators::parse(BUNDLED_INDICATORS).expect("bundled indicators.toml is valid")
    }

    pub fn merge_toml(&mut self, src: &str) -> Result<(), String> {
        #[derive(Deserialize)]
        struct Partial {
            label: Option<String>,
            #[serde(default)]
            words: Vec<String>,
        }
        let lists: BTreeMap<String, Partial> = toml::from_str(src).map_err(|e| e.to_string())?;
        for (k, p) in lists {
            let e = self
                .lists
                .entry(k.clone())
                .or_insert_with(|| IndicatorList {
                    label: k.clone(),
                    words: vec![],
                });
            if let Some(l) = p.label {
                e.label = l;
            }
            for w in p.words {
                if !e.words.contains(&w) {
                    e.words.push(w);
                }
            }
        }
        Ok(())
    }

    pub fn get(&self, kind: &str) -> &[String] {
        self.lists
            .get(kind)
            .map(|l| l.words.as_slice())
            .unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_lists_parse() {
        let a = Abbreviations::bundled();
        assert!(a.get("dr").unwrap().iter().any(|m| m == "doctor"));
        let docs: Vec<String> = a
            .for_meaning("doctor")
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        for d in ["dr", "mo", "md", "gp", "mb"] {
            assert!(docs.contains(&d.to_string()), "{d} missing from {docs:?}");
        }
        let i = Indicators::bundled();
        assert!(i.get("anagram").contains(&"wild".to_string()));
    }

    #[test]
    fn user_files_merge() {
        let mut a = Abbreviations::bundled();
        a.merge_toml("[abbreviations]\nzz = [\"sleep\"]\ndr = []")
            .unwrap();
        assert_eq!(a.get("zz").unwrap(), ["sleep"]);
        assert!(a.get("dr").is_none());

        let mut i = Indicators::bundled();
        i.merge_toml(
            "[anagram]\nwords = [\"discombobulated\"]\n[mine]\nlabel = \"Mine\"\nwords=[\"x\"]",
        )
        .unwrap();
        assert!(i.get("anagram").contains(&"discombobulated".to_string()));
        assert!(i.get("anagram").contains(&"wild".to_string()));
        assert_eq!(i.lists["mine"].label, "Mine");
    }
}
