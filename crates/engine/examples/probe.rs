use std::collections::HashSet;
use xw_engine::*;
fn main() {
    let word = std::env::args().nth(1).unwrap_or("rant".into());
    let t = std::time::Instant::now();
    let lex = Lexicon::bundled(&[], &HashSet::new());
    let abbr = Abbreviations::bundled();
    println!("load {:?}", t.elapsed());
    let ctx = Ctx::new(&lex, &abbr);
    let t = std::time::Instant::now();
    let h = wordplay::hidden::find(ctx, &word);
    println!(
        "hidden {:?}: {:?}",
        t.elapsed(),
        h.in_word
            .iter()
            .take(15)
            .map(|x| &x.word.text)
            .collect::<Vec<_>>()
    );
    println!(
        "across: {:?}",
        h.across
            .iter()
            .take(10)
            .map(|x| x
                .words
                .iter()
                .map(|w| w.text.as_str())
                .collect::<Vec<_>>()
                .join(" "))
            .collect::<Vec<_>>()
    );
    let t = std::time::Instant::now();
    let a = wordplay::anagram::find(ctx, &word);
    println!(
        "anagram {:?}: single {:?}",
        t.elapsed(),
        a.single.iter().map(|w| &w.text).collect::<Vec<_>>()
    );
    println!(
        "  two: {:?}",
        a.two
            .iter()
            .take(12)
            .map(|p| format!("{} {}", p[0].text, p[1].text))
            .collect::<Vec<_>>()
    );
    println!(
        "  three: {:?}",
        a.three
            .iter()
            .take(8)
            .map(|p| p
                .iter()
                .map(|w| w.text.as_str())
                .collect::<Vec<_>>()
                .join(" "))
            .collect::<Vec<_>>()
    );
    let t = std::time::Instant::now();
    let c = wordplay::charade::find(ctx, &word);
    println!(
        "charade {:?}: {:?}",
        t.elapsed(),
        c.iter()
            .take(10)
            .map(|c| c
                .parts
                .iter()
                .map(|p| p.letters.as_str())
                .collect::<Vec<_>>()
                .join("+"))
            .collect::<Vec<_>>()
    );
    let c = wordplay::container::find(ctx, &word);
    println!(
        "container: {:?}",
        c.iter()
            .take(10)
            .map(|c| format!("{}[{}]", c.outer.letters, c.inner.letters))
            .collect::<Vec<_>>()
    );
    let d = wordplay::deletion::find(ctx, &word);
    println!(
        "deletion: {:?}",
        d.literal
            .iter()
            .take(10)
            .map(|d| format!("{}-{}", d.source.text, d.removed))
            .collect::<Vec<_>>()
    );
    println!(
        "anag-del: {:?}",
        d.anagram
            .iter()
            .take(10)
            .map(|d| format!("{}-{}", d.source.text, d.removed))
            .collect::<Vec<_>>()
    );
    let h = wordplay::homophone::find(ctx, &word);
    println!(
        "homophones: {:?} pairs {:?}",
        h.single.iter().map(|w| &w.text).collect::<Vec<_>>(),
        h.pairs
            .iter()
            .take(6)
            .map(|p| format!("{} {}", p[0].text, p[1].text))
            .collect::<Vec<_>>()
    );
    println!(
        "subs: {:?}",
        wordplay::substitution::find(ctx, &word)
            .iter()
            .take(10)
            .map(|s| &s.source.text)
            .collect::<Vec<_>>()
    );
    println!(
        "alt: {:?}",
        wordplay::alternation::find(ctx, &word)
            .iter()
            .take(10)
            .map(|s| &s.source.text)
            .collect::<Vec<_>>()
    );
    println!(
        "shift: {:?}",
        wordplay::shift::find(ctx, &word)
            .iter()
            .map(|s| (s.shift, &s.word.text))
            .collect::<Vec<_>>()
    );
}
