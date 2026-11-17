//! Colouring the clue as you type, and the quick checks under it.

use xw_engine::bank::Tone;

/// Word tokens: (byte start, byte end, lowercase text). Letters only, so
/// "Artist's" gives "artist" and "s", and "(9)" gives nothing.
fn tokens(text: &str) -> Vec<(usize, usize, String)> {
    let mut out = vec![];
    let mut start = None;
    for (i, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        match (c.is_alphabetic(), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push((s, i, text[s..i].to_lowercase()));
                start = None;
            }
            _ => {}
        }
    }
    out
}

fn words(pick: &str) -> Vec<String> {
    tokens(pick).into_iter().map(|t| t.2).collect()
}

/// Does `pick` (one or more words) match the tokens starting at `i`, with only
/// whitespace between them? Returns the byte range covered.
fn match_at(
    text: &str,
    toks: &[(usize, usize, String)],
    i: usize,
    pick: &[String],
) -> Option<(usize, usize)> {
    if pick.is_empty() || i + pick.len() > toks.len() {
        return None;
    }
    for (k, w) in pick.iter().enumerate() {
        let t = &toks[i + k];
        if &t.2 != w {
            return None;
        }
        if k > 0
            && !text[toks[i + k - 1].1..t.0]
                .chars()
                .all(char::is_whitespace)
        {
            return None;
        }
    }
    Some((toks[i].0, toks[i + pick.len() - 1].1))
}

/// Split `text` into spans, colouring whole-word (or whole-phrase) matches of
/// each pick. Earlier picks win. The span texts concatenate back to `text`.
pub fn spans(text: &str, picks: &[(String, Tone)]) -> Vec<(String, Option<Tone>)> {
    let toks = tokens(text);
    let picks: Vec<(Vec<String>, Tone)> = picks
        .iter()
        .map(|(p, t)| (words(p), *t))
        .filter(|(w, _)| !w.is_empty())
        .collect();
    let mut out: Vec<(String, Option<Tone>)> = vec![];
    let mut push = |s: &str, tone: Option<Tone>| {
        if s.is_empty() {
            return;
        }
        match out.last_mut() {
            Some((prev, None)) if tone.is_none() => prev.push_str(s),
            _ => out.push((s.to_string(), tone)),
        }
    };
    let (mut pos, mut i) = (0, 0);
    while i < toks.len() {
        let hit = picks
            .iter()
            .find_map(|(w, tone)| match_at(text, &toks, i, w).map(|r| (r, w.len(), *tone)));
        if let Some(((a, b), n, tone)) = hit {
            push(&text[pos..a], None);
            push(&text[a..b], Some(tone));
            pos = b;
            i += n;
        } else {
            i += 1;
        }
    }
    push(&text[pos..], None);
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Checks {
    /// Every wordplay part's chosen word appears in the clue.
    pub adds_up: bool,
    /// The chosen definition is the clue's first or last word(s).
    pub definition_at_end: bool,
}

pub fn checks(text: &str, parts: &[(String, Tone)], definition: Option<&str>) -> Checks {
    let toks = tokens(text);
    let found = |pick: &str| {
        let w = words(pick);
        (0..toks.len()).any(|i| match_at(text, &toks, i, &w).is_some())
    };
    let adds_up = !parts.is_empty() && parts.iter().all(|(p, _)| found(p));
    let definition_at_end = definition.map(words).is_some_and(|d| {
        !d.is_empty()
            && d.len() <= toks.len()
            && (match_at(text, &toks, 0, &d).is_some()
                || match_at(text, &toks, toks.len() - d.len(), &d).is_some())
    });
    Checks {
        adds_up,
        definition_at_end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xw_engine::bank::Tone;

    #[test]
    fn colours_whole_words_case_insensitively() {
        let s = spans(
            "Gold box by Artist's band (9)",
            &[
                ("gold".into(), Tone::P1),
                ("box".into(), Tone::P2),
                ("artist".into(), Tone::P3),
                ("band".into(), Tone::Definition),
                ("by".into(), Tone::Indicator),
            ],
        );
        assert_eq!(
            s.iter().map(|x| x.0.as_str()).collect::<String>(),
            "Gold box by Artist's band (9)"
        );
        assert!(s.contains(&("Gold".into(), Some(Tone::P1))));
        assert!(s.contains(&("Artist".into(), Some(Tone::P3))));
        assert!(s.contains(&("band".into(), Some(Tone::Definition))));
        assert!(!s.iter().any(|x| x.0 == "o" && x.1.is_some()));
    }

    #[test]
    fn picks_inside_other_words_are_not_coloured() {
        let s = spans("Goldsmith", &[("gold".into(), Tone::P1)]);
        assert_eq!(s, vec![("Goldsmith".to_string(), None)]);
    }

    #[test]
    fn phrases_and_checks() {
        let s = spans("Gold next to box", &[("next to".into(), Tone::Indicator)]);
        assert!(s.contains(&("next to".into(), Some(Tone::Indicator))));
        let parts = [
            ("gold".to_string(), Tone::P1),
            ("box".to_string(), Tone::P2),
            ("artist".to_string(), Tone::P3),
        ];
        let c = checks("Gold box by artist for band (9)", &parts, Some("band"));
        assert!(c.adds_up && c.definition_at_end);
        assert!(checks("Band: gold, box, artist", &parts, Some("band")).definition_at_end);
        assert!(
            !checks(
                "Gold box",
                &[("artist".to_string(), Tone::P3)],
                Some("band")
            )
            .adds_up
        );
        assert!(!checks("Gold band box artist", &parts, Some("band")).definition_at_end);
    }

    #[test]
    fn empty_inputs() {
        assert!(spans("", &[("gold".into(), Tone::P1)]).is_empty());
        let c = checks("", &[], None);
        assert!(!c.adds_up && !c.definition_at_end);
    }
}
