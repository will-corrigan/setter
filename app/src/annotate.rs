//! Writing a clue's annotation ("OR (gold) + CHEST (box) + RA (artist)") from
//! the device and the words picked in the word bank.

use xw_engine::bank::{DeviceKind, DeviceOption, Tone};

fn is_part(t: Tone) -> bool {
    matches!(t, Tone::P1 | Tone::P2 | Tone::P3 | Tone::P4 | Tone::P5)
}

/// "SE (outside)" → "SE".
fn short(title: &str) -> &str {
    title.split(" (").next().unwrap_or(title).trim()
}

/// The annotation for a clue, from its device, option and picks
/// (column index, picked word). Empty until a wordplay part is picked.
pub fn annotation(kind: DeviceKind, option: &DeviceOption, picks: &[(usize, String)]) -> String {
    let pick = |i: usize| picks.iter().find(|(c, _)| *c == i).map(|(_, w)| w.as_str());
    let parts: Vec<(String, &str)> = option
        .columns
        .iter()
        .enumerate()
        .filter(|(_, c)| is_part(c.tone))
        .filter_map(|(i, c)| pick(i).map(|w| (short(&c.title).to_string(), w)))
        .collect();
    if parts.is_empty() {
        return String::new();
    }
    let labelled: Vec<String> = parts.iter().map(|(t, w)| format!("{t} ({w})")).collect();
    let words: Vec<String> = parts.iter().map(|(_, w)| w.to_uppercase()).collect();
    let wordplay = match kind {
        DeviceKind::Charade | DeviceKind::FirstLetters => labelled.join(" + "),
        DeviceKind::Container => labelled.join(" around "),
        DeviceKind::Deletion => labelled.join(" minus "),
        DeviceKind::ChangeALetter if labelled.len() == 3 => {
            format!("{} with {} for {}", labelled[0], labelled[2], labelled[1])
        }
        DeviceKind::ChangeALetter => labelled.join(", "),
        DeviceKind::Anagram => format!("anagram of {}", words.join(" ")),
        DeviceKind::Hidden => format!("hidden in {}", words.join(" ")),
        DeviceKind::AlternateLetters => format!("alternate letters of {}", words.join(" ")),
        DeviceKind::Reversal => format!("{} reversed", labelled.join(" ")),
        DeviceKind::SoundsLike => format!("sounds like {}", labelled.join(" + ")),
        DeviceKind::Spoonerism => format!("Spoonerism of {}", labelled.join(" ")),
        DeviceKind::DoubleDefinition => format!(
            "double definition: {}",
            parts
                .iter()
                .map(|(_, w)| *w)
                .collect::<Vec<_>>()
                .join(" / ")
        ),
        DeviceKind::CrypticDefinition => "cryptic definition".to_string(),
    };
    let definition = option
        .columns
        .iter()
        .position(|c| c.tone == Tone::Definition)
        .and_then(pick);
    match definition {
        Some(d)
            if kind != DeviceKind::DoubleDefinition && kind != DeviceKind::CrypticDefinition =>
        {
            format!("{wordplay}; definition: {d}")
        }
        _ => wordplay,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xw_engine::bank::{Column, DeviceKind, DeviceOption, Tone};

    fn col(title: &str, tone: Tone) -> Column {
        Column {
            title: title.into(),
            tone,
            wide: false,
            groups: vec![],
            marks: vec![],
        }
    }
    fn option(cols: Vec<Column>) -> DeviceOption {
        DeviceOption {
            label: String::new(),
            tiles: vec![],
            columns: cols,
            reads_as: String::new(),
        }
    }

    #[test]
    fn charade_joins_parts() {
        let o = option(vec![
            col("OR", Tone::P1),
            col("CHEST", Tone::P2),
            col("RA", Tone::P3),
            col("Definition", Tone::Definition),
            col("Joining words", Tone::Indicator),
        ]);
        let picks = vec![
            (0, "gold".to_string()),
            (1, "box".to_string()),
            (2, "artist".to_string()),
            (3, "band".to_string()),
        ];
        assert_eq!(
            annotation(DeviceKind::Charade, &o, &picks),
            "OR (gold) + CHEST (box) + RA (artist); definition: band"
        );
    }

    #[test]
    fn container_and_partial_picks() {
        let o = option(vec![
            col("SE (outside)", Tone::P1),
            col("TAR (inside)", Tone::P2),
            col("Definition", Tone::Definition),
        ]);
        let picks = vec![(0, "south-east".to_string()), (1, "sailor".to_string())];
        assert_eq!(
            annotation(DeviceKind::Container, &o, &picks),
            "SE (south-east) around TAR (sailor)"
        );
        assert_eq!(annotation(DeviceKind::Container, &o, &[]), "");
    }

    #[test]
    fn whole_word_devices() {
        let o = option(vec![
            col("Letters to mix", Tone::P2),
            col("Definition", Tone::Definition),
        ]);
        let picks = vec![(0, "chart rose".to_string()), (1, "band".to_string())];
        assert_eq!(
            annotation(DeviceKind::Anagram, &o, &picks),
            "anagram of CHART ROSE; definition: band"
        );
        let o = option(vec![col("Words that hide it", Tone::P2)]);
        assert_eq!(
            annotation(
                DeviceKind::Hidden,
                &o,
                &[(0, "majOR CHEST RAnge".to_string())]
            ),
            "hidden in MAJOR CHEST RANGE"
        );
    }
}
