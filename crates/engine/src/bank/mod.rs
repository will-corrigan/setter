//! The word bank: every device's material for one answer, shaped for the
//! Write clues screen (devices → options → columns). No UI code here.

mod devices;

use crate::lists::Indicators;
use crate::text;
use crate::thesaurus::Thesaurus;
use crate::wordplay::Ctx;
use serde::Serialize;

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Group {
    Wordplay,
    MeaningOnly,
}

impl Group {
    pub fn name(self) -> &'static str {
        match self {
            Group::Wordplay => "Wordplay",
            Group::MeaningOnly => "Meaning only",
        }
    }
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DeviceKind {
    AlternateLetters,
    Anagram,
    ChangeALetter,
    Charade,
    Container,
    Deletion,
    FirstLetters,
    Hidden,
    Reversal,
    SoundsLike,
    Spoonerism,
    CrypticDefinition,
    DoubleDefinition,
}

impl DeviceKind {
    /// Screen order: Wordplay then Meaning only, each alphabetical.
    pub const ALL: [DeviceKind; 13] = [
        DeviceKind::AlternateLetters,
        DeviceKind::Anagram,
        DeviceKind::ChangeALetter,
        DeviceKind::Charade,
        DeviceKind::Container,
        DeviceKind::Deletion,
        DeviceKind::FirstLetters,
        DeviceKind::Hidden,
        DeviceKind::Reversal,
        DeviceKind::SoundsLike,
        DeviceKind::Spoonerism,
        DeviceKind::CrypticDefinition,
        DeviceKind::DoubleDefinition,
    ];

    pub fn name(self) -> &'static str {
        match self {
            DeviceKind::AlternateLetters => "Alternate letters",
            DeviceKind::Anagram => "Anagram",
            DeviceKind::ChangeALetter => "Change a letter",
            DeviceKind::Charade => "Charade",
            DeviceKind::Container => "Container",
            DeviceKind::Deletion => "Deletion",
            DeviceKind::FirstLetters => "First letters",
            DeviceKind::Hidden => "Hidden",
            DeviceKind::Reversal => "Reversal",
            DeviceKind::SoundsLike => "Sounds like",
            DeviceKind::Spoonerism => "Spoonerism",
            DeviceKind::CrypticDefinition => "Cryptic definition",
            DeviceKind::DoubleDefinition => "Double definition",
        }
    }

    pub fn group(self) -> Group {
        match self {
            DeviceKind::CrypticDefinition | DeviceKind::DoubleDefinition => Group::MeaningOnly,
            _ => Group::Wordplay,
        }
    }
}

/// Colour role: wordplay parts p1–p5, the definition, or an indicator.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Tone {
    P1,
    P2,
    P3,
    P4,
    P5,
    Definition,
    Indicator,
}

impl Tone {
    /// The n-th part colour, cycling through p1–p5.
    pub fn part(n: usize) -> Tone {
        [Tone::P1, Tone::P2, Tone::P3, Tone::P4, Tone::P5][n % 5]
    }

    /// CSS class suffix: "p1".."p5", "def", "ind".
    pub fn class(self) -> &'static str {
        match self {
            Tone::P1 => "p1",
            Tone::P2 => "p2",
            Tone::P3 => "p3",
            Tone::P4 => "p4",
            Tone::P5 => "p5",
            Tone::Definition => "def",
            Tone::Indicator => "ind",
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Tile {
    pub letters: String,
    pub tone: Option<Tone>,
    pub struck: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Column {
    pub title: String,
    pub tone: Tone,
    pub wide: bool,
    pub groups: Vec<(String, Vec<String>)>,
    /// item text -> highlighted byte ranges (hidden words, alternate letters)
    pub marks: Vec<(String, Vec<(usize, usize)>)>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct DeviceOption {
    pub label: String,
    pub tiles: Vec<Tile>,
    pub columns: Vec<Column>,
    pub reads_as: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct DeviceView {
    pub kind: DeviceKind,
    pub count: usize,
    pub options: Vec<DeviceOption>,
}

/// How many wordplay parts must appear in the clue for it to "add up".
/// Most devices need every part; a double definition needs any two of its
/// meanings; a cryptic definition has no parts to add up.
pub fn required_parts(kind: DeviceKind, option: &DeviceOption) -> usize {
    let parts = option
        .columns
        .iter()
        .filter(|c| matches!(c.tone, Tone::P1 | Tone::P2 | Tone::P3 | Tone::P4 | Tone::P5))
        .count();
    match kind {
        DeviceKind::DoubleDefinition => parts.min(2),
        DeviceKind::CrypticDefinition => 0,
        _ => parts,
    }
}

/// Most options shown per device (counts are not trimmed).
pub const MAX_OPTIONS: usize = 12;

/// Every device's material for `answer` (a grid pattern; '?' marks a blank).
/// A phrase is best given as words ("way in"): its meanings are looked up as
/// a phrase; the wordplay uses its letters.
pub fn build(ctx: Ctx, ind: &Indicators, th: &Thesaurus, answer: &str) -> Vec<DeviceView> {
    let key = text::key(answer);
    let phrase = answer.trim().to_lowercase();
    if answer.contains('?') || key.len() < 2 {
        return DeviceKind::ALL
            .iter()
            .map(|&kind| DeviceView {
                kind,
                count: 0,
                options: vec![],
            })
            .collect();
    }
    DeviceKind::ALL
        .iter()
        .map(|&kind| devices::build_one(ctx, ind, th, kind, &key, &phrase))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lists::Indicators;
    use crate::wordplay::testutil::{ctx, thesaurus};

    #[test]
    fn orchestra_devices() {
        let v = build(ctx(), &Indicators::bundled(), thesaurus(), "orchestra");
        let names: Vec<_> = v.iter().map(|d| d.kind.name()).collect();
        assert_eq!(
            names,
            [
                "Alternate letters",
                "Anagram",
                "Change a letter",
                "Charade",
                "Container",
                "Deletion",
                "First letters",
                "Hidden",
                "Reversal",
                "Sounds like",
                "Spoonerism",
                "Cryptic definition",
                "Double definition"
            ]
        );
        let ch = v.iter().find(|d| d.kind == DeviceKind::Charade).unwrap();
        let opt = ch
            .options
            .iter()
            .find(|o| o.label == "OR + CHEST + RA")
            .expect("split");
        let titles: Vec<_> = opt.columns.iter().map(|c| c.title.as_str()).collect();
        assert_eq!(titles, ["OR", "CHEST", "RA", "Definition", "Joining words"]);
        assert!(opt.columns[0]
            .groups
            .iter()
            .any(|(_, items)| items.iter().any(|i| i == "gold")));
        let chest: Vec<&str> = opt.columns[1]
            .groups
            .iter()
            .flat_map(|(_, v)| v.iter().map(String::as_str))
            .collect();
        assert!(
            chest.contains(&"coffer") && chest.contains(&"box"),
            "{chest:?}"
        );
        let def: Vec<&str> = opt.columns[3]
            .groups
            .iter()
            .flat_map(|(_, v)| v.iter().map(String::as_str))
            .collect();
        assert!(def.contains(&"band"), "{def:?}");
        assert!(
            v.iter()
                .find(|d| d.kind == DeviceKind::Anagram)
                .unwrap()
                .count
                > 100
        );
        assert_eq!(
            v.iter()
                .find(|d| d.kind == DeviceKind::Container)
                .unwrap()
                .count,
            0
        );
    }

    #[test]
    fn double_definition_has_a_column_per_sense() {
        let v = build(ctx(), &Indicators::bundled(), thesaurus(), "pitch");
        let dd = v
            .iter()
            .find(|d| d.kind == DeviceKind::DoubleDefinition)
            .unwrap();
        let cols = &dd.options[0].columns;
        assert_eq!(cols.len(), 6);
        assert!(cols
            .iter()
            .all(|c| c.title.contains(": ") && !c.groups.is_empty()));
    }

    #[test]
    fn loose_synonyms_lead_with_common_words() {
        let v = build(ctx(), &Indicators::bundled(), thesaurus(), "orchestra");
        let ch = v.iter().find(|d| d.kind == DeviceKind::Charade).unwrap();
        let def = ch.options[0]
            .columns
            .iter()
            .find(|c| c.title == "Definition")
            .unwrap();
        let related = &def.groups.iter().find(|(g, _)| g == "Related").unwrap().1;
        let band = related
            .iter()
            .position(|w| w == "band")
            .expect("band listed");
        assert!(band < 8, "band at {band}: {related:?}");
        let german = related
            .iter()
            .position(|w| w == "German band")
            .unwrap_or(usize::MAX);
        assert!(band < german);
        let everything: Vec<&String> = v
            .iter()
            .flat_map(|d| &d.options)
            .flat_map(|o| &o.columns)
            .flat_map(|c| &c.groups)
            .flat_map(|(_, items)| items)
            .collect();
        assert!(
            everything.iter().all(|w| !crate::text::is_offensive(w)),
            "offensive item listed"
        );
        assert!(crate::text::is_offensive("Nigger heaven"));
        assert!(!crate::text::is_offensive("niggard"));
    }

    /// Slurs reported by the final review on everyday answers.
    const SLURS: &[&str] = &[
        "jigaboo",
        "nigra",
        "pickaninny",
        "darkey",
        "wop",
        "dago",
        "ginzo",
        "greaseball",
        "honky",
        "honkie",
        "honkey",
        "jungle bunny",
        "ofay",
        "slant-eye",
        "sambo",
    ];

    #[test]
    fn sensitive_answers_list_no_slurs() {
        for answer in ["black", "white", "italian", "whitey", "negro", "mongrel"] {
            for d in build(ctx(), &Indicators::bundled(), thesaurus(), answer) {
                for item in d
                    .options
                    .iter()
                    .flat_map(|o| &o.columns)
                    .flat_map(|c| &c.groups)
                    .flat_map(|(_, i)| i)
                {
                    let low = item.to_lowercase();
                    assert!(
                        !SLURS.contains(&low.as_str()),
                        "{answer}/{}: {item}",
                        d.kind.name()
                    );
                }
            }
        }
    }

    #[test]
    fn banned_words_never_appear_in_the_bank() {
        let banned = std::collections::HashSet::from(["box".to_string(), "coffer".to_string()]);
        let lex = crate::Lexicon::bundled(&[], &banned);
        let abbr = crate::Abbreviations::bundled();
        let v = build(
            Ctx::new(&lex, &abbr),
            &Indicators::bundled(),
            thesaurus(),
            "orchestra",
        );
        let ch = v.iter().find(|d| d.kind == DeviceKind::Charade).unwrap();
        let opt = ch
            .options
            .iter()
            .find(|o| o.label == "OR + CHEST + RA")
            .unwrap();
        let chest: Vec<&str> = opt.columns[1]
            .groups
            .iter()
            .flat_map(|(_, v)| v.iter().map(String::as_str))
            .collect();
        assert!(
            !chest.contains(&"box") && !chest.contains(&"coffer"),
            "{chest:?}"
        );
        assert!(chest.contains(&"casket"));
    }

    #[test]
    fn parts_needed_to_add_up_depend_on_the_device() {
        let get = |answer: &str, kind: DeviceKind| {
            let v = build(ctx(), &Indicators::bundled(), thesaurus(), answer);
            let d = v.into_iter().find(|d| d.kind == kind).unwrap();
            required_parts(d.kind, &d.options[0])
        };
        assert_eq!(get("orchestra", DeviceKind::Charade), 3);
        assert_eq!(get("pitch", DeviceKind::DoubleDefinition), 2);
        assert_eq!(get("orchestra", DeviceKind::CrypticDefinition), 0);
        assert_eq!(get("tempo", DeviceKind::FirstLetters), 5);
    }

    #[test]
    fn labels_and_reads_as_never_show_filtered_words() {
        // An offensive word that is also an anagram of GOAD, ranked first.
        let lex = crate::Lexicon::bundled(&["dago".to_string()], &Default::default());
        let abbr = crate::Abbreviations::bundled();
        let v = build(
            Ctx::new(&lex, &abbr),
            &Indicators::bundled(),
            thesaurus(),
            "goad",
        );
        for d in &v {
            for o in &d.options {
                assert!(
                    !o.label.to_lowercase().contains("dago"),
                    "{}: {}",
                    d.kind.name(),
                    o.label
                );
                assert!(
                    !o.reads_as.to_lowercase().contains("dago"),
                    "{}: {}",
                    d.kind.name(),
                    o.reads_as
                );
            }
        }
    }

    #[test]
    fn columns_list_each_word_once_and_titles_are_tidy() {
        for answer in ["orchestra", "pitch", "chest", "band"] {
            for d in build(ctx(), &Indicators::bundled(), thesaurus(), answer) {
                for c in d.options.iter().flat_map(|o| &o.columns) {
                    let mut seen = std::collections::HashSet::new();
                    for item in c.groups.iter().flat_map(|(_, i)| i) {
                        assert!(
                            seen.insert(item.to_lowercase()),
                            "{answer}/{}/{}: {item} twice",
                            d.kind.name(),
                            c.title
                        );
                    }
                    assert!(
                        !c.title.ends_with([',', ';', ':', '.']),
                        "{answer}: title {:?}",
                        c.title
                    );
                }
            }
        }
    }

    #[test]
    fn blanks_and_tiny_answers_are_all_zero() {
        for a in ["orch?stra", "a", ""] {
            assert!(build(ctx(), &Indicators::bundled(), thesaurus(), a)
                .iter()
                .all(|d| d.count == 0 && d.options.is_empty()));
        }
    }

    #[test]
    fn phrase_answers_use_letters_only() {
        let v = build(ctx(), &Indicators::bundled(), thesaurus(), "it is");
        assert_eq!(v.len(), 13);
    }

    #[test]
    fn long_answers_do_not_panic() {
        let v = build(
            ctx(),
            &Indicators::bundled(),
            thesaurus(),
            "counterrevolution",
        );
        assert_eq!(v.len(), 13);
    }

    #[test]
    fn first_letters_has_column_per_letter() {
        let v = build(ctx(), &Indicators::bundled(), thesaurus(), "tempo");
        let fl = v
            .iter()
            .find(|d| d.kind == DeviceKind::FirstLetters)
            .unwrap();
        let cols = &fl.options[0].columns;
        assert_eq!(cols.iter().filter(|c| c.title.ends_with('…')).count(), 5);
    }

    #[test]
    fn anagram_splits_use_no_rare_word() {
        let c = ctx();
        let splits = |c: Ctx| -> Vec<String> {
            build(c, &Indicators::bundled(), thesaurus(), "begin")
                .into_iter()
                .find(|d| d.kind == DeviceKind::Anagram)
                .unwrap()
                .options
                .into_iter()
                .flat_map(|o| o.columns.into_iter().next().unwrap().groups)
                .flat_map(|(_, items)| items)
                .collect()
        };
        assert!(splits(c).contains(&"be gin".to_string()), "{:?}", splits(c));
        let gin = c.lex.lookup("gin").unwrap();
        let ratings = crate::quality::Qualities::from_vec(
            (0..c.lex.words.len() as u32)
                .map(|id| {
                    if id == gin {
                        crate::quality::Quality::Rare
                    } else {
                        crate::quality::Quality::Good
                    }
                })
                .collect(),
        );
        let rated = splits(c.with_quality(&ratings));
        assert!(!rated.contains(&"be gin".to_string()), "{rated:?}");
        assert!(
            rated.iter().any(|s| s.contains(' ')),
            "other splits stay: {rated:?}"
        );
    }

    fn labels(answer: &str, kind: DeviceKind) -> Vec<String> {
        build(ctx(), &Indicators::bundled(), thesaurus(), answer)
            .into_iter()
            .find(|d| d.kind == kind)
            .unwrap()
            .options
            .into_iter()
            .map(|o| o.label)
            .collect()
    }

    #[test]
    fn every_part_offered_has_something_to_clue_it_with() {
        let th = thesaurus();
        // REES has no meaning of its own: it is offered only as SEER backwards.
        let v = build(ctx(), &Indicators::bundled(), th, "trees");
        let charade = v.iter().find(|d| d.kind == DeviceKind::Charade).unwrap();
        let rees = charade
            .options
            .iter()
            .find(|o| o.label == "T + REES")
            .expect("T + REES");
        assert!(
            rees.reads_as.contains("seer backwards"),
            "{}",
            rees.reads_as
        );
        let groups: Vec<&str> = rees.columns[1]
            .groups
            .iter()
            .map(|(g, _)| g.as_str())
            .collect();
        assert!(!groups.contains(&"Word"), "{groups:?}");
        let joining = rees.columns.last().unwrap();
        assert!(
            joining.groups.iter().any(|(g, _)| g == "Backwards"),
            "{:?}",
            joining.groups
        );
        let plain = charade
            .options
            .iter()
            .find(|o| o.label == "TREE + S")
            .unwrap();
        let joining = plain.columns.last().unwrap();
        assert!(!joining.groups.iter().any(|(g, _)| g == "Backwards"));
        assert!(labels("trees", DeviceKind::Charade).contains(&"TREE + S".to_string()));
        for answer in ["trees", "orchestra", "carpet", "teaspoon", "standard"] {
            for kind in [DeviceKind::Charade, DeviceKind::Container] {
                let v = build(ctx(), &Indicators::bundled(), th, answer);
                let d = v.iter().find(|d| d.kind == kind).unwrap();
                for o in &d.options {
                    for c in o.columns.iter().filter(|c| {
                        matches!(c.tone, Tone::P1 | Tone::P2 | Tone::P3 | Tone::P4 | Tone::P5)
                    }) {
                        let clues: usize = c
                            .groups
                            .iter()
                            .filter(|(g, _)| g != "Word" && g != "Backwards")
                            .map(|(_, items)| items.len())
                            .sum();
                        assert!(
                            clues > 0,
                            "{answer} {}: {} has nothing to clue it",
                            o.label,
                            c.title
                        );
                    }
                }
            }
        }
    }
}
