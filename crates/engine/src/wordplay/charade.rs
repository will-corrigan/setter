//! Charades: the target split into consecutive pieces, each a word,
//! abbreviation or reversed word — e.g. CARPET = CAR + PET.

use super::{Ctx, Part};
use crate::text;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Charade {
    pub parts: Vec<Part>,
    pub score: f32,
}

const MAX_RESULTS: usize = 200;

pub fn find(ctx: Ctx, target: &str) -> Vec<Charade> {
    let t = text::key(target);
    let n = t.len();
    if n < 3 {
        return vec![];
    }
    // Memoise the part analysis for every substring.
    let mut parts: Vec<Vec<Option<Part>>> = vec![vec![None; n + 1]; n + 1];
    for i in 0..n {
        for j in (i + 1)..=n {
            if j - i < n {
                parts[i][j] = ctx.part(&t[i..j], true);
            }
        }
    }
    let mut out = Vec::new();
    let mut push = |ps: Vec<&Part>| {
        // Need at least one real word of 3+ letters; all-abbreviation charades are noise.
        if !ps
            .iter()
            .any(|p| Ctx::is_word_part(p) && p.letters.len() >= 3)
        {
            return;
        }
        let mean = ps.iter().map(|p| p.score).sum::<f32>() / ps.len() as f32;
        let score = mean - 0.08 * (ps.len() as f32 - 2.0);
        out.push(Charade {
            parts: ps.into_iter().cloned().collect(),
            score,
        });
    };
    // Two grids indexed together: a range reads clearer than zipped iterators.
    #[allow(clippy::needless_range_loop)]
    for i in 1..n {
        if let (Some(a), Some(b)) = (&parts[0][i], &parts[i][n]) {
            push(vec![a, b]);
        }
    }
    for i in 1..n {
        for j in (i + 1)..n {
            if let (Some(a), Some(b), Some(c)) = (&parts[0][i], &parts[i][j], &parts[j][n]) {
                push(vec![a, b, c]);
            }
        }
    }
    out.sort_by(|a, b| b.score.total_cmp(&a.score));
    out.truncate(MAX_RESULTS);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordplay::testutil::ctx;

    #[test]
    fn carpet_is_car_pet() {
        let c = find(ctx(), "carpet");
        let splits: Vec<Vec<&str>> = c
            .iter()
            .map(|c| c.parts.iter().map(|p| p.letters.as_str()).collect())
            .collect();
        assert!(splits.contains(&vec!["car", "pet"]), "{splits:?}");
        for s in &splits {
            assert_eq!(s.concat(), "carpet");
        }
    }
}
