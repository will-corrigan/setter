//! Across Lite .puz (binary, with checksums), the format most crossword apps open.

use crate::grid::Dir;
use crate::puzzle::Puzzle;

fn cksum(data: &[u8], mut c: u16) -> u16 {
    for &b in data {
        c = if c & 1 == 1 {
            (c >> 1) | 0x8000
        } else {
            c >> 1
        };
        c = c.wrapping_add(b as u16);
    }
    c
}

/// Latin-1 bytes, with typographic quotes and dashes made plain first.
fn latin1(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| match c {
            '‘' | '’' | '′' => '\'',
            '“' | '”' | '″' => '"',
            '–' | '—' => '-',
            '…' => '.',
            _ => c,
        })
        .map(|c| {
            if (c as u32) < 256 {
                c as u32 as u8
            } else {
                b'?'
            }
        })
        .collect()
}

/// The puzzle as an Across Lite .puz file. Every square must be filled.
pub fn puz(p: &Puzzle) -> Result<Vec<u8>, String> {
    let g = &p.grid;
    if g.width > 255 || g.height > 255 {
        return Err("Grids bigger than 255 squares across can't be saved as .puz.".into());
    }
    if g.cells.iter().any(|c| !c.block && c.letter.is_none()) {
        return Err("Fill every square before exporting .puz.".into());
    }
    let solution: Vec<u8> = g
        .cells
        .iter()
        .map(|c| {
            if c.block {
                b'.'
            } else {
                c.letter.unwrap().to_ascii_uppercase() as u8
            }
        })
        .collect();
    let state: Vec<u8> = g
        .cells
        .iter()
        .map(|c| if c.block { b'.' } else { b'-' })
        .collect();

    // Clues in Across Lite order: by starting square, across before down.
    let mut lights = g.lights();
    lights.sort_by_key(|l| (l.cells[0], l.dir != Dir::Across));
    let clues: Vec<Vec<u8>> = lights
        .iter()
        .map(|l| {
            let c = p.clues.get(&l.id).cloned().unwrap_or_default();
            let e = if c.enumeration.trim().is_empty() {
                l.len().to_string()
            } else {
                c.enumeration.trim().to_string()
            };
            let t = c.text.split_whitespace().collect::<Vec<_>>().join(" ");
            latin1(&if t.is_empty() {
                format!("({e})")
            } else {
                format!("{t} ({e})")
            })
        })
        .collect();
    let (title, author, copyright, notes) = (
        latin1(p.title.trim()),
        latin1(p.author.trim()),
        Vec::new(),
        latin1(p.notes.trim()),
    );

    let mut cib = vec![g.width as u8, g.height as u8];
    cib.extend_from_slice(&(clues.len() as u16).to_le_bytes());
    cib.extend_from_slice(&1u16.to_le_bytes()); // puzzle type: normal
    cib.extend_from_slice(&0u16.to_le_bytes()); // not scrambled
    let c_cib = cksum(&cib, 0);

    let mut text = vec![];
    for s in [&title, &author, &copyright] {
        if !s.is_empty() {
            text.extend_from_slice(s);
            text.push(0);
        }
    }
    for c in &clues {
        text.extend_from_slice(c);
    }
    if !notes.is_empty() {
        text.extend_from_slice(&notes);
        text.push(0);
    }
    let (c_sol, c_state, c_text) = (cksum(&solution, 0), cksum(&state, 0), cksum(&text, 0));
    let overall = cksum(&text, cksum(&state, cksum(&solution, c_cib)));

    let mut out = Vec::with_capacity(0x34 + solution.len() * 2 + text.len() + 16);
    out.extend_from_slice(&overall.to_le_bytes());
    out.extend_from_slice(b"ACROSS&DOWN\0");
    out.extend_from_slice(&c_cib.to_le_bytes());
    let magic = b"ICHEATED";
    let parts = [c_cib, c_sol, c_state, c_text];
    out.extend(
        parts
            .iter()
            .enumerate()
            .map(|(i, p)| magic[i] ^ (p & 0xFF) as u8),
    );
    out.extend(
        parts
            .iter()
            .enumerate()
            .map(|(i, p)| magic[i + 4] ^ (p >> 8) as u8),
    );
    out.extend_from_slice(b"1.3\0");
    out.extend_from_slice(&[0; 2]); // reserved
    out.extend_from_slice(&[0; 2]); // scrambled checksum
    out.extend_from_slice(&[0; 12]); // reserved
    out.extend_from_slice(&cib);
    out.extend_from_slice(&solution);
    out.extend_from_slice(&state);
    for s in [&title, &author, &copyright] {
        out.extend_from_slice(s);
        out.push(0);
    }
    for c in &clues {
        out.extend_from_slice(c);
        out.push(0);
    }
    out.extend_from_slice(&notes);
    out.push(0);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::Grid;
    use crate::puzzle::{Clue, Puzzle};

    fn sample() -> Puzzle {
        let mut p = Puzzle::new(3, 3);
        p.title = "Tiny".into();
        p.author = "Setter".into();
        p.grid = Grid::from_rows(&["cat", "a#o", "bye"]);
        p.clues.insert(
            "0,0,a".into(),
            Clue {
                text: "Pet’s feline".into(),
                ..Default::default()
            },
        );
        p
    }

    fn cksum(data: &[u8], mut c: u16) -> u16 {
        for &b in data {
            c = if c & 1 == 1 {
                (c >> 1) | 0x8000
            } else {
                c >> 1
            };
            c = c.wrapping_add(b as u16);
        }
        c
    }

    #[test]
    fn header_layout_and_grids() {
        let b = puz(&sample()).unwrap();
        assert_eq!(&b[2..14], b"ACROSS&DOWN\0");
        assert_eq!(&b[0x18..0x1C], b"1.3\0");
        assert_eq!((b[0x2C], b[0x2D]), (3, 3));
        assert_eq!(u16::from_le_bytes([b[0x2E], b[0x2F]]), 4, "four clues");
        assert_eq!(&b[0x34..0x34 + 9], b"CATA.OBYE");
        assert_eq!(&b[0x34 + 9..0x34 + 18], b"----.----");
        let strings: Vec<&[u8]> = b[0x34 + 18..].split(|&x| x == 0).collect();
        assert_eq!(strings[0], b"Tiny");
        assert_eq!(strings[1], b"Setter");
        assert_eq!(
            strings[3], b"Pet's feline (3)",
            "clue order 1a, 1d, 2d, 3a; latin-1 safe"
        );
        assert_eq!(strings[4], b"(3)");
    }

    #[test]
    fn checksums_verify() {
        let b = puz(&sample()).unwrap();
        let cib = cksum(&b[0x2C..0x34], 0);
        assert_eq!(u16::from_le_bytes([b[0x0E], b[0x0F]]), cib);
        let sol = &b[0x34..0x34 + 9];
        let state = &b[0x34 + 9..0x34 + 18];
        let mut text = vec![];
        let strings: Vec<&[u8]> = b[0x34 + 18..].split(|&x| x == 0).collect();
        // title, author, copyright (with NUL when non-empty), clues (no NUL), notes (with NUL if non-empty)
        for s in &strings[..3] {
            if !s.is_empty() {
                text.extend_from_slice(s);
                text.push(0);
            }
        }
        for s in &strings[3..7] {
            text.extend_from_slice(s);
        }
        let overall = cksum(&text, cksum(state, cksum(sol, cib)));
        assert_eq!(u16::from_le_bytes([b[0], b[1]]), overall);
        let parts = [cib, cksum(sol, 0), cksum(state, 0), cksum(&text, 0)];
        let magic = b"ICHEATED";
        for i in 0..4 {
            assert_eq!(b[0x10 + i], magic[i] ^ (parts[i] & 0xFF) as u8);
            assert_eq!(b[0x14 + i], magic[i + 4] ^ (parts[i] >> 8) as u8);
        }
    }

    #[test]
    fn unfinished_grids_are_refused() {
        let mut p = sample();
        p.grid.cells[0].letter = None;
        assert!(puz(&p).unwrap_err().contains("Fill every square"));
    }
}
