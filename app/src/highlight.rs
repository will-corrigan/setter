//! Colouring the clue as you type, and the quick checks under it.

use xw_engine::bank::Tone;

/// Word tokens: (byte start, byte end, lowercase text). Letters and digits,
/// so "Artist's" gives "artist" and "s", and "(9)" gives "9".
fn tokens(text: &str) -> Vec<(usize, usize, String)> {
    let mut out = vec![];
    let mut start = None;
    for (i, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        match (c.is_alphanumeric(), start) {
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

/// A picked word or phrase: its tokens and the exact separators between them
/// ("King's Counsel" → [king, s, counsel] with ["'", " "]).
struct Pick {
    words: Vec<String>,
    seps: Vec<String>,
}

fn pick_of(p: &str) -> Pick {
    let toks = tokens(p);
    Pick {
        seps: toks
            .windows(2)
            .map(|w| p[w[0].1..w[1].0].to_string())
            .collect(),
        words: toks.into_iter().map(|t| t.2).collect(),
    }
}

/// Text between two tokens matches a pick's separator: any whitespace for a
/// space, otherwise exactly ("-", "'").
fn same_sep(text_sep: &str, pick_sep: &str) -> bool {
    if pick_sep.trim().is_empty() {
        !text_sep.is_empty() && text_sep.chars().all(char::is_whitespace)
    } else {
        text_sep == pick_sep
    }
}

/// Does `pick` match the tokens starting at `i`, with the same separators?
/// Returns the byte range covered.
fn match_at(
    text: &str,
    toks: &[(usize, usize, String)],
    i: usize,
    pick: &Pick,
) -> Option<(usize, usize)> {
    let n = pick.words.len();
    if n == 0 || i + n > toks.len() {
        return None;
    }
    for (k, w) in pick.words.iter().enumerate() {
        let t = &toks[i + k];
        if &t.2 != w || (k > 0 && !same_sep(&text[toks[i + k - 1].1..t.0], &pick.seps[k - 1])) {
            return None;
        }
    }
    Some((toks[i].0, toks[i + n - 1].1))
}

/// Where a picked word or phrase first appears as whole words, as a byte
/// range: "box" is in "Gold box" but not in "boxer".
pub fn find(text: &str, word: &str) -> Option<(usize, usize)> {
    let pick = pick_of(word);
    let toks = tokens(text);
    (0..toks.len()).find_map(|i| match_at(text, &toks, i, &pick))
}

/// Split `text` into spans, colouring whole-word (or whole-phrase) matches of
/// each pick. Earlier picks win. The span texts concatenate back to `text`.
pub fn spans(text: &str, picks: &[(String, Tone)]) -> Vec<(String, Option<Tone>)> {
    let toks = tokens(text);
    let picks: Vec<(Pick, Tone)> = picks
        .iter()
        .map(|(p, t)| (pick_of(p), *t))
        .filter(|(w, _)| !w.words.is_empty())
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
            .find_map(|(w, tone)| match_at(text, &toks, i, w).map(|r| (r, w.words.len(), *tone)));
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

/// The clue without a trailing enumeration such as "(9)" or "(5,3)".
fn without_enumeration(text: &str) -> &str {
    let t = text.trim_end();
    if let Some(open) = t.rfind('(') {
        let inner = &t[open + 1..];
        if inner.ends_with(')')
            && inner[..inner.len() - 1]
                .chars()
                .all(|c| c.is_ascii_digit() || ",- ".contains(c))
        {
            return &t[..open];
        }
    }
    text
}

pub fn checks(text: &str, parts: &[(String, Tone)], definition: Option<&str>) -> Checks {
    checks_for(text, parts, parts.len(), definition)
}

/// Like `checks`, but "adds up" needs only `required` of the parts in the clue
/// (see `xw_engine::bank::required_parts`); 0 means the device has no parts.
pub fn checks_for(
    text: &str,
    parts: &[(String, Tone)],
    required: usize,
    definition: Option<&str>,
) -> Checks {
    let text = without_enumeration(text);
    let toks = tokens(text);
    let found = |pick: &str| {
        let w = pick_of(pick);
        (0..toks.len()).any(|i| match_at(text, &toks, i, &w).is_some())
    };
    let adds_up = required > 0 && parts.iter().filter(|(p, _)| found(p)).count() >= required;
    let definition_at_end = definition.map(pick_of).is_some_and(|d| {
        !d.words.is_empty()
            && d.words.len() <= toks.len()
            && (match_at(text, &toks, 0, &d).is_some()
                || match_at(text, &toks, toks.len() - d.words.len(), &d).is_some())
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
    fn hyphen_apostrophe_and_digit_picks_colour() {
        let picks = [
            ("first-class".to_string(), Tone::P1),
            ("King's Counsel".to_string(), Tone::P2),
            ("500".to_string(), Tone::P3),
        ];
        let s = spans("A first-class King's Counsel and 500 men", &picks);
        assert!(s.contains(&("first-class".into(), Some(Tone::P1))), "{s:?}");
        assert!(
            s.contains(&("King's Counsel".into(), Some(Tone::P2))),
            "{s:?}"
        );
        assert!(s.contains(&("500".into(), Some(Tone::P3))), "{s:?}");
        assert!(spans("first class", &picks[..1])
            .iter()
            .all(|x| x.1.is_none()));
        assert!(
            checks(
                "A first-class lot of 500",
                &[picks[0].clone(), picks[2].clone()],
                None
            )
            .adds_up
        );
    }

    #[test]
    fn adds_up_needs_only_the_required_number_of_parts() {
        let parts = [
            ("throw".to_string(), Tone::P1),
            ("tar".to_string(), Tone::P2),
            ("field".to_string(), Tone::P3),
        ];
        assert!(checks_for("Throw tar", &parts[..2], 2, None).adds_up);
        assert!(!checks_for("Throw tar", &parts[..2], 0, None).adds_up);
        assert!(!checks_for("Throw tar", &parts, 3, None).adds_up);
        assert!(!checks_for("Throw", &parts[..1], 2, None).adds_up);
    }

    #[test]
    fn empty_inputs() {
        assert!(spans("", &[("gold".into(), Tone::P1)]).is_empty());
        let c = checks("", &[], None);
        assert!(!c.adds_up && !c.definition_at_end);
    }
}
