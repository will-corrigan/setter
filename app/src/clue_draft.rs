//! Building a clue from the word bank: each picked word goes into the clue
//! text, in a sensible place, and can be swapped, taken out or moved. The
//! clue stays plain text the setter can change however they like.

use crate::highlight;
use std::collections::HashMap;
use xw_engine::bank::{Column, Tone};

fn is_part(t: Tone) -> bool {
    matches!(t, Tone::P1 | Tone::P2 | Tone::P3 | Tone::P4 | Tone::P5)
}

/// The order the option's columns read in a clue: the definition at one end,
/// the wordplay parts in column order, with the joining or indicator words
/// after the first part ("gold with box", "fibre broken").
pub fn order(columns: &[Column], definition_last: bool) -> Vec<usize> {
    let of = |want: fn(Tone) -> bool| -> Vec<usize> {
        columns
            .iter()
            .enumerate()
            .filter(|(_, c)| want(c.tone))
            .map(|(i, _)| i)
            .collect()
    };
    let parts = of(is_part);
    let indicators = of(|t| t == Tone::Indicator);
    let definition = of(|t| t == Tone::Definition);
    let mut out = vec![];
    if !definition_last {
        out.extend(&definition);
    }
    let mut parts = parts.into_iter();
    out.extend(parts.next());
    out.extend(indicators);
    out.extend(parts);
    if definition_last {
        out.extend(&definition);
    }
    out
}

/// Single spaces, no space at either end, and a capital to start.
fn tidy(text: &str) -> String {
    let mut joined = text.split_whitespace().collect::<Vec<_>>().join(" ");
    for mark in [",", ".", ";", ":", "!", "?", ")"] {
        joined = joined.replace(&format!(" {mark}"), mark);
    }
    let mut chars = joined.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// After an edit: the pick that began the clue before, if it no longer does,
/// goes back to how it was picked ("Gold" → "gold"); then [`tidy`].
fn settle(before: &str, after: &str, picks: &HashMap<usize, String>) -> String {
    let was_first = picks
        .values()
        .find(|w| highlight::find(before, w).is_some_and(|(a, _)| a == 0));
    let mut out = after.trim_start().to_string();
    if let Some(w) = was_first {
        if let Some((a, b)) = highlight::find(&out, w).filter(|(a, _)| *a > 0) {
            out = format!("{}{}{}", &out[..a], w, &out[b..]);
        }
    }
    tidy(&out)
}

fn insert_at(text: &str, at: usize, word: &str) -> String {
    format!("{} {} {}", &text[..at], word, &text[at..])
}

/// The clue after the pick in `column` changes from `old` to `new`.
/// `picks` holds every column's pick as it is now, `order` the reading order.
/// A swapped word takes the old one's place; a new one goes after the nearest
/// earlier piece already in the clue, else before the nearest later one; a
/// removed one is taken out. Anything else in the clue is left as it is.
pub fn place(
    text: &str,
    order: &[usize],
    picks: &HashMap<usize, String>,
    column: usize,
    old: Option<&str>,
    new: Option<&str>,
) -> String {
    if let Some((a, b)) = old.and_then(|w| highlight::find(text, w)) {
        let rest = format!("{}{}{}", &text[..a], new.unwrap_or(""), &text[b..]);
        return settle(text, &rest, picks);
    }
    let Some(word) = new else {
        return text.to_string();
    };
    let at = order.iter().position(|c| *c == column);
    let found = |c: &usize| {
        picks
            .get(c)
            .filter(|_| *c != column)
            .and_then(|w| highlight::find(text, w))
    };
    let placed = at.and_then(|at| {
        order[..at]
            .iter()
            .rev()
            .find_map(found)
            .map(|(_, end)| insert_at(text, end, word))
            .or_else(|| {
                order[at + 1..]
                    .iter()
                    .find_map(found)
                    .map(|(start, _)| insert_at(text, start, word))
            })
    });
    let first = at == Some(0);
    let placed = placed.unwrap_or_else(|| {
        if first {
            format!("{word} {text}")
        } else {
            format!("{text} {word}")
        }
    });
    settle(text, &placed, picks)
}

/// The clue with the definition moved to its start or its end, written as it
/// was picked (so a definition leaving the start loses its capital).
pub fn move_definition(
    text: &str,
    definition: &str,
    last: bool,
    picks: &HashMap<usize, String>,
) -> String {
    let Some((a, b)) = highlight::find(text, definition) else {
        return text.to_string();
    };
    let rest = format!("{} {}", &text[..a], &text[b..]);
    let rest = rest.split_whitespace().collect::<Vec<_>>().join(" ");
    let moved = if last {
        format!("{rest} {definition}")
    } else {
        format!("{definition} {rest}")
    };
    settle(text, &moved, picks)
}

/// The clue after the setter dragged its pieces about: single spaces, a
/// capital to start, and picked words away from the start as they were
/// picked ("Band" in the middle goes back to "band").
pub fn arrange(text: &str, picks: &HashMap<usize, String>) -> String {
    let mut out = text.to_string();
    for word in picks.values() {
        if let Some((a, b)) = highlight::find(&out, word)
            .filter(|(a, _)| out[..*a].chars().any(|c| !c.is_whitespace()))
        {
            out = format!("{}{}{}", &out[..a], word, &out[b..]);
        }
    }
    tidy(&out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(tone: Tone) -> Column {
        Column {
            title: String::new(),
            tone,
            wide: false,
            groups: vec![],
            marks: vec![],
        }
    }

    /// A charade's columns: two parts, the definition, joining words.
    fn charade() -> Vec<Column> {
        vec![
            col(Tone::P1),
            col(Tone::P2),
            col(Tone::Definition),
            col(Tone::Indicator),
        ]
    }

    fn picks(list: &[(usize, &str)]) -> HashMap<usize, String> {
        list.iter().map(|(c, w)| (*c, w.to_string())).collect()
    }

    #[test]
    fn the_reading_order_puts_the_definition_at_one_end() {
        assert_eq!(order(&charade(), false), vec![2, 0, 3, 1]);
        assert_eq!(order(&charade(), true), vec![0, 3, 1, 2]);
        // An anagram: the letters, then the mixing word.
        let anagram = vec![col(Tone::P1), col(Tone::Definition), col(Tone::Indicator)];
        assert_eq!(order(&anagram, false), vec![1, 0, 2]);
    }

    #[test]
    fn picks_build_the_clue_in_reading_order() {
        let o = order(&charade(), false);
        let mut p = picks(&[(0, "gold")]);
        let t = place("", &o, &p, 0, None, Some("gold"));
        assert_eq!(t, "Gold");
        p.insert(1, "box".into());
        let t = place(&t, &o, &p, 1, None, Some("box"));
        assert_eq!(t, "Gold box");
        p.insert(3, "with".into());
        let t = place(&t, &o, &p, 3, None, Some("with"));
        assert_eq!(t, "Gold with box");
        p.insert(2, "band".into());
        let t = place(&t, &o, &p, 2, None, Some("band"));
        assert_eq!(t, "Band gold with box");
    }

    #[test]
    fn another_pick_in_a_column_takes_the_old_one_s_place() {
        let o = order(&charade(), false);
        let p = picks(&[(0, "or"), (1, "box")]);
        let t = place("Gold with box, surely", &o, &p, 0, Some("gold"), Some("or"));
        assert_eq!(t, "Or with box, surely");
    }

    #[test]
    fn un_picking_takes_the_word_out_and_keeps_the_rest() {
        let o = order(&charade(), false);
        let p = picks(&[(0, "gold")]);
        let t = place("Gold rush with box", &o, &p, 1, Some("box"), None);
        assert_eq!(t, "Gold rush with");
        // Not in the clue any more (the setter removed it): nothing to do.
        let t = place("Gold rush", &o, &p, 1, Some("box"), None);
        assert_eq!(t, "Gold rush");
    }

    #[test]
    fn the_setter_s_own_words_stay_put() {
        let o = order(&charade(), true);
        let p = picks(&[(0, "gold"), (2, "band")]);
        // Their own clue; the definition goes on the end.
        let t = place("Precious gold, they say", &o, &p, 2, None, Some("band"));
        assert_eq!(t, "Precious gold band, they say");
    }

    #[test]
    fn the_definition_moves_between_the_ends() {
        let p = picks(&[(0, "gold"), (1, "box"), (2, "band")]);
        let t = move_definition("Band gold with box", "band", true, &p);
        assert_eq!(t, "Gold with box band");
        let t = move_definition(&t, "band", false, &p);
        assert_eq!(t, "Band gold with box");
        // Not there: nothing moves.
        assert_eq!(move_definition("Gold box", "band", true, &p), "Gold box");
        // A word the setter capitalised mid-clue is theirs.
        let t = move_definition("Band of Gold", "band", true, &p);
        assert_eq!(t, "Of Gold band");
    }

    #[test]
    fn dragged_pieces_settle_into_a_tidy_clue() {
        let p = picks(&[(0, "gold"), (2, "band")]);
        assert_eq!(arrange("gold  Band with box ", &p), "Gold band with box");
        // Picked words take their picked form; other words are left alone.
        assert_eq!(arrange("box with Gold Band", &p), "Box with gold band");
        assert_eq!(arrange("New York gold", &p), "New York gold");
    }

    #[test]
    fn a_pick_pushed_off_the_front_loses_its_capital() {
        let o = order(&charade(), false);
        let p = picks(&[(0, "gold"), (2, "band")]);
        let t = place("Gold", &o, &p, 2, None, Some("band"));
        assert_eq!(t, "Band gold");
    }
}
