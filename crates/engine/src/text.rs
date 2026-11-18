//! Small string helpers. Everything in the engine works on "keys": lowercase
//! ASCII letters only (spaces, hyphens and apostrophes stripped).

/// Letters-only lowercase key: "It's a-go" -> "itsago".
pub fn key(s: &str) -> String {
    s.chars()
        .filter_map(|c| {
            let c = fold_accent(c);
            c.is_ascii_alphabetic().then(|| c.to_ascii_lowercase())
        })
        .collect()
}

fn fold_accent(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
        'ç' => 'c',
        'è' | 'é' | 'ê' | 'ë' => 'e',
        'ì' | 'í' | 'î' | 'ï' => 'i',
        'ñ' => 'n',
        'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
        'ù' | 'ú' | 'û' | 'ü' => 'u',
        _ => c,
    }
}

static OFFENSIVE: &str = include_str!("../../../data/offensive.txt");

/// Is `s` (a word or phrase) on the built-in offensive list? Matches any word
/// of it, or the whole item with spaces and hyphens removed ("slant-eye").
/// Lowercase list entries match any case; capitalised ones only that case
/// ("Chink" is blocked, "chink" — a gap — is not). "niggard" is fine.
pub fn is_offensive(s: &str) -> bool {
    use std::collections::HashSet;
    use std::sync::OnceLock;
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    let set = SET.get_or_init(|| {
        OFFENSIVE
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect()
    });
    // Lowercase entries match any case; capitalised entries match exactly.
    let hit = |w: &str| set.contains(w.to_lowercase().as_str()) || set.contains(w);
    let words: Vec<&str> = s
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty())
        .collect();
    words.iter().any(|w| hit(w)) || (words.len() > 1 && hit(&words.concat()))
}

/// Letter histogram of a key.
pub type Counts = [u8; 26];

pub fn counts(key: &str) -> Counts {
    let mut c = [0u8; 26];
    for b in key.bytes() {
        if b.is_ascii_lowercase() {
            c[(b - b'a') as usize] += 1;
        }
    }
    c
}

/// Is `small` a sub-multiset of `big`?
pub fn fits(small: &Counts, big: &Counts) -> bool {
    small.iter().zip(big).all(|(a, b)| a <= b)
}

pub fn minus(big: &Counts, small: &Counts) -> Counts {
    let mut out = [0u8; 26];
    for i in 0..26 {
        out[i] = big[i] - small[i];
    }
    out
}

pub fn total(c: &Counts) -> usize {
    c.iter().map(|&n| n as usize).sum()
}

/// Sorted-letter signature used by the anagram index.
pub fn sorted_key(key: &str) -> String {
    let mut b: Vec<u8> = key.bytes().collect();
    b.sort_unstable();
    String::from_utf8(b).unwrap()
}

pub fn counts_to_sorted(c: &Counts) -> String {
    let mut s = String::with_capacity(total(c));
    for (i, &n) in c.iter().enumerate() {
        for _ in 0..n {
            s.push((b'a' + i as u8) as char);
        }
    }
    s
}

pub fn reverse(key: &str) -> String {
    key.chars().rev().collect()
}

/// Crossword enumeration for a display form: "it is" -> "2,2", "to-do" -> "2-2".
pub fn enumeration(display: &str) -> String {
    let mut out = String::new();
    let mut run = 0usize;
    for c in display.chars() {
        if c == ' ' || c == '-' {
            if run > 0 {
                out.push_str(&run.to_string());
                out.push(if c == ' ' { ',' } else { '-' });
                run = 0;
            }
        } else if fold_accent(c).is_ascii_alphabetic() {
            run += 1;
        }
    }
    out.push_str(&run.to_string());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offensive_matching() {
        assert!(is_offensive("slant-eye"));
        assert!(is_offensive("jungle bunny"));
        assert!(is_offensive("Chink"));
        assert!(!is_offensive("chink"));
        assert!(!is_offensive("vocal chink"));
        assert!(!is_offensive("spick and span"));
        assert!(!is_offensive("niggard"));
        // Anagram finders split words into made-up phrases: no slur may appear.
        assert!(is_offensive("be nig"));
        assert!(is_offensive("nig nog"));
    }

    #[test]
    fn keys_strip_punctuation() {
        assert_eq!(key("It's a-go"), "itsago");
        assert_eq!(key("Café"), "cafe");
    }

    #[test]
    fn enumerations() {
        assert_eq!(enumeration("orchestra"), "9");
        assert_eq!(enumeration("it is"), "2,2");
        assert_eq!(enumeration("to-do list"), "2-2,4");
        assert_eq!(enumeration("isn't"), "4");
    }

    #[test]
    fn count_arithmetic() {
        let big = counts("orchestra");
        let small = counts("cart");
        assert!(fits(&small, &big));
        assert_eq!(counts_to_sorted(&minus(&big, &small)), "ehors");
        assert!(!fits(&counts("zoo"), &big));
    }
}
