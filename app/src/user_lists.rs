//! The setter's own lists, as Settings edits them: words to add, words never
//! to suggest, abbreviations and indicator words. They live in the config
//! folder in the formats the engine reads (words.txt, banned.txt,
//! abbreviations.toml, indicators.toml); Settings is the way to change them.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The two word lists.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WordList {
    /// Words and phrases treated as everyday words.
    Yours,
    /// Never offered anywhere.
    Never,
}

impl WordList {
    pub fn file(self) -> &'static str {
        match self {
            WordList::Yours => "words.txt",
            WordList::Never => "banned.txt",
        }
    }

    fn header(self) -> &'static str {
        match self {
            WordList::Yours => "# Your words and phrases, one per line, as Settings keeps them.\n",
            WordList::Never => "# Words never to suggest, one per line, as Settings keeps them.\n",
        }
    }
}

/// One word or phrase, and when it was added (if known).
#[derive(Clone, PartialEq, Debug)]
pub struct Entry {
    pub text: String,
    pub added: Option<SystemTime>,
}

/// A line is "text", or "text<TAB>seconds since 1970" once Settings has
/// added it. The engine reads only the text.
fn parse_line(line: &str) -> Option<Entry> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (text, when) = match line.split_once('\t') {
        Some((t, w)) => (t.trim(), w.trim().parse::<u64>().ok()),
        None => (line, None),
    };
    Some(Entry {
        text: text.to_string(),
        added: when.map(|s| UNIX_EPOCH + Duration::from_secs(s)),
    })
}

/// A list, newest first (undated entries last, in file order).
pub fn read(dir: &Path, list: WordList) -> Vec<Entry> {
    let mut entries: Vec<Entry> = fs::read_to_string(dir.join(list.file()))
        .unwrap_or_default()
        .lines()
        .filter_map(parse_line)
        .collect();
    entries.sort_by_key(|e| std::cmp::Reverse(e.added));
    entries
}

fn write(dir: &Path, list: WordList, entries: &[Entry]) -> Result<(), String> {
    let mut out = String::from(list.header());
    for e in entries {
        out.push_str(&e.text);
        if let Some(secs) = e
            .added
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
        {
            out.push('\t');
            out.push_str(&secs.to_string());
        }
        out.push('\n');
    }
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    crate::files::write_whole(&dir.join(list.file()), out.as_bytes())
        .map_err(|e| format!("Couldn't save the list: {e}"))
}

/// Add words (once each, any case). Returns how many were new.
pub fn add(dir: &Path, list: WordList, texts: &[String], now: SystemTime) -> Result<usize, String> {
    let mut entries = read(dir, list);
    let mut added = 0;
    for t in texts {
        let text = t.trim().to_lowercase();
        if text.is_empty()
            || text.starts_with('#')
            || entries.iter().any(|e| e.text.to_lowercase() == text)
        {
            continue;
        }
        entries.insert(
            0,
            Entry {
                text,
                added: Some(now),
            },
        );
        added += 1;
    }
    if added > 0 {
        write(dir, list, &entries)?;
    }
    Ok(added)
}

pub fn remove(dir: &Path, list: WordList, text: &str) -> Result<(), String> {
    let mut entries = read(dir, list);
    entries.retain(|e| e.text != text);
    write(dir, list, &entries)
}

/// Words from an imported text file: one per line, `#` lines skipped.
pub fn lines_of(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(parse_line)
        .map(|e| e.text)
        .collect()
}

/// A list as a plain text file, one per line.
pub fn export(entries: &[Entry]) -> String {
    entries.iter().map(|e| format!("{}\n", e.text)).collect()
}

/// The lengths of a phrase's words, as an enumeration: "6,7".
pub fn enumeration(text: &str) -> String {
    text.split([' ', '-'])
        .filter(|w| !w.is_empty())
        .map(|w| w.chars().filter(|c| c.is_alphabetic()).count().to_string())
        .collect::<Vec<_>>()
        .join(",")
}

// ---------- abbreviations ----------

#[derive(Serialize, Deserialize, Default)]
struct AbbrevFile {
    #[serde(default)]
    abbreviations: BTreeMap<String, Vec<String>>,
}

const ABBREV_HEADER: &str =
    "# Your abbreviations, merged with the built-in list, as Settings keeps them.\n";

/// The setter's abbreviations: letters and what they stand for.
pub fn abbreviations(dir: &Path) -> Vec<(String, Vec<String>)> {
    let src = fs::read_to_string(dir.join("abbreviations.toml")).unwrap_or_default();
    toml::from_str::<AbbrevFile>(&src)
        .map(|f| f.abbreviations.into_iter().collect())
        .unwrap_or_default()
}

fn write_abbreviations(dir: &Path, all: BTreeMap<String, Vec<String>>) -> Result<(), String> {
    let body = toml::to_string(&AbbrevFile { abbreviations: all }).map_err(|e| e.to_string())?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    crate::files::write_whole(
        &dir.join("abbreviations.toml"),
        format!("{ABBREV_HEADER}{body}").as_bytes(),
    )
    .map_err(|e| format!("Couldn't save the abbreviations: {e}"))
}

/// Set the meanings of some letters ("ZZ" → "sleep, snore"). Meanings are
/// split at commas; none removes the letters.
pub fn set_abbreviation(dir: &Path, letters: &str, meanings: &str) -> Result<(), String> {
    let key: String = letters
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase();
    if key.is_empty() {
        return Err("Type the letters first.".into());
    }
    let list: Vec<String> = meanings
        .split(',')
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
        .collect();
    let mut all: BTreeMap<String, Vec<String>> = abbreviations(dir).into_iter().collect();
    if list.is_empty() {
        all.remove(&key);
    } else {
        all.insert(key, list);
    }
    write_abbreviations(dir, all)
}

// ---------- indicators ----------

#[derive(Serialize, Deserialize, Default)]
struct IndicatorWords {
    #[serde(default)]
    words: Vec<String>,
}

const INDICATOR_HEADER: &str =
    "# Your indicator words, added to the built-in lists, as Settings keeps them.\n";

/// The setter's indicator words, by kind ("anagram", "reversal", …).
pub fn indicators(dir: &Path) -> BTreeMap<String, Vec<String>> {
    let src = fs::read_to_string(dir.join("indicators.toml")).unwrap_or_default();
    toml::from_str::<BTreeMap<String, IndicatorWords>>(&src)
        .map(|m| {
            m.into_iter()
                .map(|(k, v)| (k, v.words))
                .filter(|(_, w)| !w.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn write_indicators(dir: &Path, all: &BTreeMap<String, Vec<String>>) -> Result<(), String> {
    let file: BTreeMap<&String, IndicatorWords> = all
        .iter()
        .filter(|(_, w)| !w.is_empty())
        .map(|(k, w)| (k, IndicatorWords { words: w.clone() }))
        .collect();
    let body = toml::to_string(&file).map_err(|e| e.to_string())?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    crate::files::write_whole(
        &dir.join("indicators.toml"),
        format!("{INDICATOR_HEADER}{body}").as_bytes(),
    )
    .map_err(|e| format!("Couldn't save the indicators: {e}"))
}

pub fn add_indicator(dir: &Path, kind: &str, word: &str) -> Result<(), String> {
    let word = word.trim().to_lowercase();
    if word.is_empty() {
        return Ok(());
    }
    let mut all = indicators(dir);
    let list = all.entry(kind.to_string()).or_default();
    if !list.contains(&word) {
        list.push(word);
    }
    write_indicators(dir, &all)
}

pub fn remove_indicator(dir: &Path, kind: &str, word: &str) -> Result<(), String> {
    let mut all = indicators(dir);
    if let Some(list) = all.get_mut(kind) {
        list.retain(|w| w != word);
    }
    write_indicators(dir, &all)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> std::path::PathBuf {
        let d = crate::files::scratch_dir(&format!("user-lists-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn words_are_added_once_newest_first_and_removed() {
        let d = dir("words");
        // An older file the engine wrote by hand: undated lines, a comment.
        fs::write(d.join("words.txt"), "# mine\nquixote\n").unwrap();
        let t = |s| UNIX_EPOCH + Duration::from_secs(s);
        let new = add(
            &d,
            WordList::Yours,
            &["Wordplay".into(), "quixote".into(), " ".into()],
            t(100),
        )
        .unwrap();
        assert_eq!(new, 1, "quixote is already there, any case");
        add(&d, WordList::Yours, &["minute cryptic".into()], t(200)).unwrap();
        let texts: Vec<String> = read(&d, WordList::Yours)
            .into_iter()
            .map(|e| e.text)
            .collect();
        assert_eq!(texts, ["minute cryptic", "wordplay", "quixote"]);
        assert_eq!(read(&d, WordList::Yours)[0].added, Some(t(200)));
        remove(&d, WordList::Yours, "wordplay").unwrap();
        assert_eq!(read(&d, WordList::Yours).len(), 2);
        // The engine still reads plain words from the file.
        let data = crate::files::load_engine_data_from(&d);
        assert!(data.lex.lookup("minutecryptic").is_some());
    }

    #[test]
    fn imports_skip_comments_and_blanks() {
        assert_eq!(lines_of("# list\nalpha\n\n beta \n"), ["alpha", "beta"]);
        assert_eq!(
            export(&[Entry {
                text: "a".into(),
                added: None
            }]),
            "a\n"
        );
    }

    #[test]
    fn phrases_show_their_enumeration() {
        assert_eq!(enumeration("minute cryptic"), "6,7");
        assert_eq!(enumeration("quixote"), "7");
        assert_eq!(enumeration("well-to-do"), "4,2,2");
    }

    #[test]
    fn abbreviations_are_set_and_cleared() {
        let d = dir("abbr");
        set_abbreviation(&d, "ZZ", "sleep, snore").unwrap();
        set_abbreviation(&d, "qx", "quixote").unwrap();
        assert_eq!(
            abbreviations(&d),
            [
                ("qx".to_string(), vec!["quixote".to_string()]),
                (
                    "zz".to_string(),
                    vec!["sleep".to_string(), "snore".to_string()]
                )
            ]
        );
        set_abbreviation(&d, "qx", "").unwrap();
        assert_eq!(abbreviations(&d).len(), 1);
        assert!(set_abbreviation(&d, " ", "x").is_err());
        let data = crate::files::load_engine_data_from(&d);
        assert!(data
            .abbr
            .get("zz")
            .is_some_and(|m| m.contains(&"snore".to_string())));
    }

    #[test]
    fn indicator_words_are_added_and_removed_by_kind() {
        let d = dir("ind");
        add_indicator(&d, "anagram", "Discombobulated").unwrap();
        add_indicator(&d, "anagram", "discombobulated").unwrap();
        add_indicator(&d, "reversal", "boomeranging").unwrap();
        assert_eq!(indicators(&d)["anagram"], ["discombobulated"]);
        remove_indicator(&d, "reversal", "boomeranging").unwrap();
        assert!(!indicators(&d).contains_key("reversal"));
        let data = crate::files::load_engine_data_from(&d);
        assert!(data
            .ind
            .get("anagram")
            .contains(&"discombobulated".to_string()));
    }
}
