//! The word bank: every device's material for one answer, shaped for the
//! Write clues screen (devices → options → columns). No UI code here.

mod devices;

use crate::lists::Indicators;
use crate::text;
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

/// Material that has to be fetched online for a column.
#[derive(Serialize, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Online {
    /// Synonyms and looser meanings of a word.
    Meanings(String),
    /// Straight definitions of the answer.
    Definitions(String),
    /// The answer's separate senses (double definition).
    Senses(String),
    /// Ideas linked to the answer (cryptic definition).
    Linked(String),
}

#[derive(Serialize, Clone, Debug)]
pub struct Column {
    pub title: String,
    pub tone: Tone,
    pub wide: bool,
    pub groups: Vec<(String, Vec<String>)>,
    pub online: Option<Online>,
    /// item text -> highlighted byte ranges (hidden words, alternate letters)
    pub marks: Vec<(String, Vec<(usize, usize)>)>,
}

#[derive(Serialize, Clone, Debug)]
pub struct DeviceOption {
    pub label: String,
    pub tiles: Vec<Tile>,
    pub columns: Vec<Column>,
    pub reads_as: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct DeviceView {
    pub kind: DeviceKind,
    pub count: usize,
    pub options: Vec<DeviceOption>,
}

/// Most options shown per device (counts are not trimmed).
pub const MAX_OPTIONS: usize = 12;

/// Every device's material for `answer` (a grid pattern; '?' marks a blank).
pub fn build(ctx: Ctx, ind: &Indicators, answer: &str) -> Vec<DeviceView> {
    let key = text::key(answer);
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
        .map(|&kind| devices::build_one(ctx, ind, kind, &key))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lists::Indicators;
    use crate::wordplay::testutil::ctx;

    #[test]
    fn orchestra_devices() {
        let v = build(ctx(), &Indicators::bundled(), "orchestra");
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
        assert_eq!(
            opt.columns[1].online,
            Some(Online::Meanings("chest".into()))
        );
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
    fn blanks_and_tiny_answers_are_all_zero() {
        for a in ["orch?stra", "a", ""] {
            assert!(build(ctx(), &Indicators::bundled(), a)
                .iter()
                .all(|d| d.count == 0 && d.options.is_empty()));
        }
    }

    #[test]
    fn phrase_answers_use_letters_only() {
        let v = build(ctx(), &Indicators::bundled(), "it is");
        assert_eq!(v.len(), 13);
    }

    #[test]
    fn long_answers_do_not_panic() {
        let v = build(ctx(), &Indicators::bundled(), "counterrevolution");
        assert_eq!(v.len(), 13);
    }

    #[test]
    fn first_letters_has_column_per_letter() {
        let v = build(ctx(), &Indicators::bundled(), "tempo");
        let fl = v
            .iter()
            .find(|d| d.kind == DeviceKind::FirstLetters)
            .unwrap();
        let cols = &fl.options[0].columns;
        assert_eq!(cols.iter().filter(|c| c.title.ends_with('…')).count(), 5);
    }
}
