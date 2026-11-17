//! Letter shifts: every letter moved the same distance along the alphabet
//! (a Caesar shift / ROT-n) that still spells a word.

use super::{Ctx, WordRef};
use crate::text;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Shift {
    pub shift: u8,
    pub word: WordRef,
}

pub fn shift_key(key: &str, by: u8) -> String {
    key.bytes()
        .map(|b| (b'a' + (b - b'a' + by) % 26) as char)
        .collect()
}

pub fn find(ctx: Ctx, target: &str) -> Vec<Shift> {
    let t = text::key(target);
    if t.len() < 2 {
        return vec![];
    }
    let mut out: Vec<Shift> = (1..26u8)
        .filter_map(|by| {
            let s = shift_key(&t, by);
            ctx.lex.lookup(&s).map(|id| Shift {
                shift: by,
                word: ctx.word_ref(id),
            })
        })
        .collect();
    // ROT13 is the famous one; then smaller shifts either way.
    out.sort_by_key(|s| (s.shift != 13, s.shift.min(26 - s.shift)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordplay::testutil::ctx;

    #[test]
    fn hal_ibm() {
        assert_eq!(shift_key("hal", 1), "ibm");
        let s = find(ctx(), "cheer");
        assert!(s.iter().any(|s| s.word.key == "jolly" && s.shift == 7));
    }
}
