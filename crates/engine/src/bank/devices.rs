//! One builder per device: finder results → `DeviceView`.

use super::{Column, DeviceKind, DeviceOption, DeviceView, Tile, Tone, MAX_OPTIONS};
use crate::lists::Indicators;
use crate::thesaurus::Thesaurus;
use crate::wordplay::{self, Ctx, Part, Piece};

pub(super) fn build_one(
    ctx: Ctx,
    ind: &Indicators,
    th: &Thesaurus,
    kind: DeviceKind,
    key: &str,
) -> DeviceView {
    let (count, options) = match kind {
        DeviceKind::AlternateLetters => alternate_letters(ctx, ind, th, key),
        DeviceKind::Anagram => anagram(ctx, ind, th, key),
        DeviceKind::ChangeALetter => change_a_letter(ctx, ind, th, key),
        DeviceKind::Charade => charade(ctx, ind, th, key),
        DeviceKind::Container => container(ctx, ind, th, key),
        DeviceKind::Deletion => deletion(ctx, ind, th, key),
        DeviceKind::FirstLetters => first_letters(ctx, ind, th, key),
        DeviceKind::Hidden => hidden(ctx, ind, th, key),
        DeviceKind::Reversal => reversal(ctx, ind, th, key),
        DeviceKind::SoundsLike => sounds_like(ctx, ind, th, key),
        DeviceKind::Spoonerism => spoonerism(ctx, ind, th, key),
        DeviceKind::CrypticDefinition => cryptic_definition(ctx, th, key),
        DeviceKind::DoubleDefinition => double_definition(ctx, th, key),
    };
    let mut options = options;
    options.truncate(MAX_OPTIONS);
    DeviceView {
        kind,
        count,
        options,
    }
}

type Built = (usize, Vec<DeviceOption>);

// ---------- shared column helpers ----------

fn column(title: impl Into<String>, tone: Tone) -> Column {
    Column {
        title: title.into(),
        tone,
        wide: false,
        groups: vec![],
        marks: vec![],
    }
}

fn definition(th: &Thesaurus, key: &str) -> Column {
    Column {
        groups: th.meanings(key, 80),
        ..column("Definition", Tone::Definition)
    }
}

/// An indicator column built from named indicator lists: (group label, list key).
fn indicators(title: &str, ind: &Indicators, lists: &[(&str, &str)]) -> Column {
    Column {
        groups: lists
            .iter()
            .map(|(label, k)| (label.to_string(), ind.get(k).to_vec()))
            .filter(|(_, words)| !words.is_empty())
            .collect(),
        ..column(title, Tone::Indicator)
    }
}

/// A column for one wordplay part: abbreviation meanings first, then the word and its meanings.
fn part_column(
    th: &Thesaurus,
    title: impl Into<String>,
    part: Option<&Part>,
    letters: &str,
    tone: Tone,
) -> Column {
    let mut col = column(title, tone);
    let Some(part) = part else {
        col.groups
            .push(("Letters".into(), vec![letters.to_uppercase()]));
        return col;
    };
    for p in &part.options {
        if let Piece::Abbrev { meanings } = p {
            col.groups.push(("Abbreviation".into(), meanings.clone()));
        }
    }
    let mut word = None;
    for p in &part.options {
        match p {
            Piece::Word { word: w } => {
                col.groups.push(("Word".into(), vec![w.text.clone()]));
                word.get_or_insert(w.text.clone());
            }
            Piece::Reversed { word: w } => {
                col.groups.push(("Backwards".into(), vec![w.text.clone()]));
                word.get_or_insert(w.text.clone());
            }
            Piece::Abbrev { .. } => {}
        }
    }
    if let Some(w) = word {
        col.groups.extend(th.meanings(&w, 60));
    }
    col
}

/// A column for a whole word: the word itself, then its meanings.
fn word_column(th: &Thesaurus, title: impl Into<String>, word: &str, tone: Tone) -> Column {
    let mut groups = vec![("Word".to_string(), vec![word.to_string()])];
    groups.extend(th.meanings(word, 60));
    Column {
        groups,
        ..column(title, tone)
    }
}

fn tile(letters: &str, tone: Option<Tone>) -> Tile {
    Tile {
        letters: letters.to_uppercase(),
        tone,
        struck: false,
    }
}

fn whole(key: &str) -> Vec<Tile> {
    vec![tile(key, Some(Tone::P2))]
}

fn first_label(part: &Part) -> String {
    match part.options.first() {
        Some(Piece::Word { word }) => word.text.clone(),
        Some(Piece::Reversed { word }) => format!("{} backwards", word.text),
        Some(Piece::Abbrev { meanings }) => meanings.first().cloned().unwrap_or_default(),
        None => part.letters.clone(),
    }
}

// ---------- devices ----------

fn alternate_letters(ctx: Ctx, ind: &Indicators, th: &Thesaurus, key: &str) -> Built {
    let found = wordplay::alternation::find(ctx, key);
    let mut options = vec![];
    for (odd, label) in [(true, "Odd letters"), (false, "Even letters")] {
        let items: Vec<_> = found.iter().filter(|a| a.odd == odd).collect();
        if items.is_empty() {
            continue;
        }
        let mut col = Column {
            wide: true,
            ..column("Spelt out by every other letter", Tone::P2)
        };
        col.groups.push((
            label.into(),
            items.iter().map(|a| a.source.text.clone()).collect(),
        ));
        col.marks = items
            .iter()
            .map(|a| (a.source.text.clone(), alternate_marks(&a.source.text, odd)))
            .collect();
        options.push(DeviceOption {
            label: label.into(),
            tiles: whole(key),
            reads_as: format!("{} of “{}”", label.to_lowercase(), items[0].source.text),
            columns: vec![
                col,
                definition(th, key),
                indicators("Every-other words", ind, &[("Alternate", "alternate")]),
            ],
        });
    }
    (found.len(), options)
}

/// Byte ranges of the letters picked out by every-other-letter, skipping spaces/punctuation.
fn alternate_marks(text: &str, odd: bool) -> Vec<(usize, usize)> {
    let mut n = 0;
    let mut out = vec![];
    for (i, c) in text.char_indices() {
        if c.is_ascii_alphabetic() {
            if (n % 2 == 0) == odd {
                out.push((i, i + c.len_utf8()));
            }
            n += 1;
        }
    }
    out
}

fn anagram(ctx: Ctx, ind: &Indicators, th: &Thesaurus, key: &str) -> Built {
    let a = wordplay::anagram::find(ctx, key);
    let count = a.single.len() + a.two.len() + a.three.len();
    let groups: [(&str, Vec<String>); 3] = [
        (
            "One word",
            a.single.iter().map(|w| w.text.clone()).collect(),
        ),
        ("Two words", a.two.iter().map(|ws| join(ws)).collect()),
        ("Three words", a.three.iter().map(|ws| join(ws)).collect()),
    ];
    let options = groups
        .into_iter()
        .filter(|(_, items)| !items.is_empty())
        .map(|(label, items)| {
            let reads_as = format!("letters of “{}”, mixed", items[0]);
            let letters = Column {
                wide: true,
                groups: vec![(label.into(), items)],
                ..column("Letters to mix", Tone::P2)
            };
            DeviceOption {
                label: label.into(),
                tiles: whole(key),
                reads_as,
                columns: vec![
                    letters,
                    definition(th, key),
                    indicators("Mixing words", ind, &[("Anagram", "anagram")]),
                ],
            }
        })
        .collect();
    (count, options)
}

fn join(ws: &[wordplay::WordRef]) -> String {
    ws.iter()
        .map(|w| w.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn change_a_letter(ctx: Ctx, ind: &Indicators, th: &Thesaurus, key: &str) -> Built {
    let subs = wordplay::substitution::find(ctx, key);
    let options = subs
        .iter()
        .take(MAX_OPTIONS)
        .map(|s| {
            let src = s.source.key.to_uppercase();
            let i = s.index;
            let label = format!(
                "{}({}→{}){}",
                &src[..i],
                s.from.to_uppercase(),
                s.to.to_uppercase(),
                &src[i + 1..]
            );
            let mut from = part_column(
                th,
                format!("{} (out)", s.from.to_uppercase()),
                None,
                &s.from,
                Tone::P1,
            );
            from.groups = pieces_groups(&s.from_as, &s.from);
            let mut to = part_column(
                th,
                format!("{} (in)", s.to.to_uppercase()),
                None,
                &s.to,
                Tone::P3,
            );
            to.groups = pieces_groups(&s.to_as, &s.to);
            DeviceOption {
                label,
                tiles: vec![
                    tile(&key[..i], Some(Tone::P2)),
                    tile(&key[i..i + 1], Some(Tone::P3)),
                    tile(&key[i + 1..], Some(Tone::P2)),
                ]
                .into_iter()
                .filter(|t| !t.letters.is_empty())
                .collect(),
                reads_as: format!(
                    "{src} with {} instead of {}",
                    s.to.to_uppercase(),
                    s.from.to_uppercase()
                ),
                columns: vec![
                    word_column(th, src.clone(), &s.source.text, Tone::P2),
                    from,
                    to,
                    definition(th, key),
                    indicators("Swap words", ind, &[("Substitution", "shift")]),
                ],
            }
        })
        .collect();
    (subs.len(), options)
}

fn pieces_groups(pieces: &[Piece], letters: &str) -> Vec<(String, Vec<String>)> {
    let mut groups = vec![];
    for p in pieces {
        match p {
            Piece::Abbrev { meanings } => groups.push(("Abbreviation".into(), meanings.clone())),
            Piece::Word { word } | Piece::Reversed { word } => {
                groups.push(("Word".into(), vec![word.text.clone()]))
            }
        }
    }
    if groups.is_empty() {
        groups.push(("Letters".into(), vec![letters.to_uppercase()]));
    }
    groups
}

fn charade(ctx: Ctx, ind: &Indicators, th: &Thesaurus, key: &str) -> Built {
    let found = wordplay::charade::find(ctx, key);
    let options = found
        .iter()
        .take(MAX_OPTIONS)
        .map(|c| {
            let mut columns: Vec<Column> = c
                .parts
                .iter()
                .enumerate()
                .map(|(n, p)| {
                    part_column(
                        th,
                        p.letters.to_uppercase(),
                        Some(p),
                        &p.letters,
                        Tone::part(n),
                    )
                })
                .collect();
            columns.push(definition(th, key));
            columns.push(indicators("Joining words", ind, &[("Charade", "charade")]));
            DeviceOption {
                label: c
                    .parts
                    .iter()
                    .map(|p| p.letters.to_uppercase())
                    .collect::<Vec<_>>()
                    .join(" + "),
                tiles: c
                    .parts
                    .iter()
                    .enumerate()
                    .map(|(n, p)| tile(&p.letters, Some(Tone::part(n))))
                    .collect(),
                reads_as: c
                    .parts
                    .iter()
                    .map(first_label)
                    .collect::<Vec<_>>()
                    .join(" + "),
                columns,
            }
        })
        .collect();
    (found.len(), options)
}

fn container(ctx: Ctx, ind: &Indicators, th: &Thesaurus, key: &str) -> Built {
    let found = wordplay::container::find(ctx, key);
    let options = found
        .iter()
        .take(MAX_OPTIONS)
        .map(|c| {
            let outer = c.outer.letters.to_uppercase();
            let inner = c.inner.letters.to_uppercase();
            let (pre, post) = outer.split_at(c.at);
            DeviceOption {
                label: format!("{pre}({inner}){post}"),
                tiles: [
                    tile(pre, Some(Tone::P1)),
                    tile(&inner, Some(Tone::P2)),
                    tile(post, Some(Tone::P1)),
                ]
                .into_iter()
                .filter(|t| !t.letters.is_empty())
                .collect(),
                reads_as: format!("{} around {}", first_label(&c.outer), first_label(&c.inner)),
                columns: vec![
                    part_column(
                        th,
                        format!("{outer} (outside)"),
                        Some(&c.outer),
                        &c.outer.letters,
                        Tone::P1,
                    ),
                    part_column(
                        th,
                        format!("{inner} (inside)"),
                        Some(&c.inner),
                        &c.inner.letters,
                        Tone::P2,
                    ),
                    definition(th, key),
                    indicators(
                        "Putting-in words",
                        ind,
                        &[
                            ("Outer holds inner", "container"),
                            ("Inner goes in", "insertion"),
                        ],
                    ),
                ],
            }
        })
        .collect();
    (found.len(), options)
}

fn deletion(ctx: Ctx, ind: &Indicators, th: &Thesaurus, key: &str) -> Built {
    use wordplay::deletion::Position;
    let d = wordplay::deletion::find(ctx, key);
    let count = d.literal.len() + d.anagram.len();
    let removal = |pos: Position, mixed: bool| {
        let (label, list) = match pos {
            Position::Start => ("First letter", "deletion_first"),
            Position::End => ("Last letter", "deletion_last"),
            Position::Middle => ("Middle", "deletion_middle"),
        };
        let mut lists = vec![(label, list), ("Named part", "deletion_general")];
        if mixed {
            lists.push(("Then mixed", "anagram"));
        }
        indicators("Removal words", ind, &lists)
    };
    let struck_tiles = |src: &str, i: usize, n: usize| -> Vec<Tile> {
        [
            tile(&src[..i], Some(Tone::P2)),
            Tile {
                struck: true,
                ..tile(&src[i..i + n], None)
            },
            tile(&src[i + n..], Some(Tone::P2)),
        ]
        .into_iter()
        .filter(|t| !t.letters.is_empty())
        .collect()
    };
    let mut options: Vec<DeviceOption> = d
        .literal
        .iter()
        .map(|x| {
            let src = x.source.key.to_uppercase();
            let rem = x.removed.to_uppercase();
            let n = x.removed.len();
            let label = format!(
                "{}({}){} − {rem}",
                &src[..x.index],
                &src[x.index..x.index + n],
                &src[x.index + n..]
            );
            let mut removed = column(format!("{rem} (taken away)"), Tone::P1);
            removed.groups = pieces_groups(&x.removed_as, &x.removed);
            DeviceOption {
                label,
                tiles: struck_tiles(&x.source.key, x.index, n),
                reads_as: format!("{src} without {rem}"),
                columns: vec![
                    word_column(th, src.clone(), &x.source.text, Tone::P2),
                    removed,
                    definition(th, key),
                    removal(x.position, false),
                ],
            }
        })
        .collect();
    options.extend(d.anagram.iter().map(|x| {
        let src = x.source.key.to_uppercase();
        let rem = x.removed.to_uppercase();
        let mut removed = column(format!("{rem} (taken away)"), Tone::P1);
        removed.groups = pieces_groups(
            &ctx.part(&x.removed, false)
                .map(|p| p.options)
                .unwrap_or_default(),
            &x.removed,
        );
        DeviceOption {
            label: format!("{src} − {rem}, mixed"),
            tiles: struck_tiles(&x.source.key, x.index, 1),
            reads_as: format!("{src} without {rem}, letters mixed"),
            columns: vec![
                word_column(th, src.clone(), &x.source.text, Tone::P2),
                removed,
                definition(th, key),
                removal(x.position, true),
            ],
        }
    }));
    (count, options)
}

fn first_letters(ctx: Ctx, ind: &Indicators, th: &Thesaurus, key: &str) -> Built {
    let letters: Vec<char> = key.chars().collect();
    let make = |first: bool| {
        let mut columns: Vec<Column> = letters
            .iter()
            .enumerate()
            .map(|(n, &l)| {
                let (title, words) = if first {
                    (
                        format!("{}…", l.to_ascii_uppercase()),
                        wordplay::initials::starting_with(ctx, l, 40),
                    )
                } else {
                    (
                        format!("…{}", l.to_ascii_uppercase()),
                        wordplay::initials::ending_with(ctx, l, 40),
                    )
                };
                Column {
                    groups: vec![("Words".into(), words.into_iter().map(|w| w.text).collect())],
                    ..column(title, Tone::part(n))
                }
            })
            .collect();
        columns.push(definition(th, key));
        columns.push(if first {
            indicators("Initial words", ind, &[("First letters", "initial")])
        } else {
            indicators("Final words", ind, &[("Last letters", "final")])
        });
        let label = if first {
            "First letters"
        } else {
            "Last letters"
        };
        DeviceOption {
            label: label.into(),
            tiles: letters
                .iter()
                .enumerate()
                .map(|(n, l)| tile(&l.to_string(), Some(Tone::part(n))))
                .collect(),
            reads_as: format!(
                "{} of words spell {}",
                label.to_lowercase(),
                key.to_uppercase()
            ),
            columns,
        }
    };
    (2, vec![make(true), make(false)])
}

fn hidden(ctx: Ctx, ind: &Indicators, th: &Thesaurus, key: &str) -> Built {
    let h = wordplay::hidden::find(ctx, key);
    let n = key.len();
    let count =
        h.in_word.len() + h.across.len() + h.reversed_in_word.len() + h.reversed_across.len();
    let across = |items: &[wordplay::hidden::HiddenAcross]| -> Vec<(String, Vec<(usize, usize)>)> {
        items
            .iter()
            .map(|x| {
                let text = x
                    .words
                    .iter()
                    .map(|w| w.key.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                (text.clone(), letter_ranges(&text, x.start, n))
            })
            .collect()
    };
    let inside = |items: &[wordplay::hidden::HiddenIn]| -> Vec<(String, Vec<(usize, usize)>)> {
        items
            .iter()
            .map(|x| (x.word.key.clone(), vec![(x.start, x.start + n)]))
            .collect()
    };
    let sets = [
        ("Across words", across(&h.across), false),
        ("Inside one word", inside(&h.in_word), false),
        ("Reversed, across words", across(&h.reversed_across), true),
        (
            "Reversed, inside one word",
            inside(&h.reversed_in_word),
            true,
        ),
    ];
    let options = sets
        .into_iter()
        .filter(|(_, items, _)| !items.is_empty())
        .map(|(label, items, reversed)| {
            let col = Column {
                wide: true,
                groups: vec![(label.into(), items.iter().map(|(t, _)| t.clone()).collect())],
                marks: items.clone(),
                ..column("Words that hide it", Tone::P2)
            };
            let mut lists = vec![("Hidden", "hidden")];
            if reversed {
                lists.push(("Backwards", "reversal"));
            }
            DeviceOption {
                label: label.into(),
                tiles: whole(key),
                reads_as: format!("hidden in “{}”", items[0].0),
                columns: vec![
                    col,
                    definition(th, key),
                    indicators("Hiding words", ind, &lists),
                ],
            }
        })
        .collect();
    (count, options)
}

/// Byte ranges covering `len` letters starting at letter offset `start`, skipping spaces.
fn letter_ranges(text: &str, start: usize, len: usize) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = vec![];
    let mut n = 0;
    for (i, c) in text.char_indices() {
        if c == ' ' {
            continue;
        }
        if n >= start && n < start + len {
            match out.last_mut() {
                Some(r) if r.1 == i => r.1 = i + c.len_utf8(),
                _ => out.push((i, i + c.len_utf8())),
            }
        }
        n += 1;
    }
    out
}

fn reversal(ctx: Ctx, ind: &Indicators, th: &Thesaurus, key: &str) -> Built {
    let r = wordplay::reversal::find(ctx, key);
    let options = r
        .words
        .iter()
        .map(|w| DeviceOption {
            label: w.key.to_uppercase(),
            tiles: vec![
                tile(key, Some(Tone::P2)),
                tile(&format!("← {}", w.key), None),
            ],
            reads_as: format!("{} backwards", w.text),
            columns: vec![
                word_column(th, w.key.to_uppercase(), &w.text, Tone::P2),
                definition(th, key),
                indicators(
                    "Backwards words",
                    ind,
                    &[("Across", "reversal"), ("Down only", "reversal_down")],
                ),
            ],
        })
        .collect();
    (r.words.len(), options)
}

fn sounds_like(ctx: Ctx, ind: &Indicators, th: &Thesaurus, key: &str) -> Built {
    let h = wordplay::homophone::find(ctx, key);
    let sound = || indicators("Sound words", ind, &[("Homophone", "homophone")]);
    let mut options: Vec<DeviceOption> = h
        .single
        .iter()
        .map(|w| DeviceOption {
            label: w.key.to_uppercase(),
            tiles: whole(key),
            reads_as: format!("sounds like {}", w.text),
            columns: vec![
                word_column(th, w.key.to_uppercase(), &w.text, Tone::P2),
                definition(th, key),
                sound(),
            ],
        })
        .collect();
    options.extend(h.pairs.iter().map(|ws| {
        DeviceOption {
            label: ws
                .iter()
                .map(|w| w.key.to_uppercase())
                .collect::<Vec<_>>()
                .join(" + "),
            tiles: whole(key),
            reads_as: format!("sounds like {}", join(ws)),
            columns: ws
                .iter()
                .enumerate()
                .map(|(n, w)| word_column(th, w.key.to_uppercase(), &w.text, Tone::part(n)))
                .chain([definition(th, key), sound()])
                .collect(),
        }
    }));
    (h.single.len() + h.pairs.len(), options)
}

fn spoonerism(ctx: Ctx, ind: &Indicators, th: &Thesaurus, key: &str) -> Built {
    let found = wordplay::spoonerism::find(ctx, key);
    let options = found
        .iter()
        .take(MAX_OPTIONS)
        .map(|s| {
            let from = format!(
                "{} {}",
                s.from[0].key.to_uppercase(),
                s.from[1].key.to_uppercase()
            );
            DeviceOption {
                label: from.clone(),
                tiles: vec![
                    tile(&s.answer_parts[0], Some(Tone::P1)),
                    tile(&s.answer_parts[1], Some(Tone::P2)),
                ],
                reads_as: format!("{from} with first sounds swapped"),
                columns: vec![
                    word_column(th, s.from[0].key.to_uppercase(), &s.from[0].text, Tone::P1),
                    word_column(th, s.from[1].key.to_uppercase(), &s.from[1].text, Tone::P2),
                    definition(th, key),
                    indicators("Spooner words", ind, &[("Spoonerism", "spoonerism")]),
                ],
            }
        })
        .collect();
    (found.len(), options)
}

fn cryptic_definition(ctx: Ctx, th: &Thesaurus, key: &str) -> Built {
    let linked = th.linked(key, 80);
    if !ctx.lex.contains(key) || linked.is_empty() {
        return (0, vec![]);
    }
    let col = Column {
        wide: true,
        groups: linked,
        ..column("Things linked to it", Tone::P1)
    };
    (
        1,
        vec![DeviceOption {
            label: "Misleading meaning".into(),
            tiles: whole(key),
            reads_as: String::new(),
            columns: vec![col, definition(th, key)],
        }],
    )
}

/// Most senses shown for a double definition.
const MAX_SENSES: usize = 6;

fn double_definition(ctx: Ctx, th: &Thesaurus, key: &str) -> Built {
    let senses: Vec<_> = th
        .senses(key, MAX_SENSES * 2)
        .into_iter()
        .filter(|s| !s.words.is_empty())
        .take(MAX_SENSES)
        .collect();
    if !ctx.lex.contains(key) || senses.len() < 2 {
        return (0, vec![]);
    }
    let columns = senses
        .iter()
        .enumerate()
        .map(|(n, s)| {
            let short: Vec<&str> = s.gloss.split_whitespace().take(5).collect();
            Column {
                groups: vec![("Words".into(), s.words.clone())],
                ..column(format!("{}: {}", s.pos, short.join(" ")), Tone::part(n))
            }
        })
        .collect();
    (
        senses.len(),
        vec![DeviceOption {
            label: "Two meanings".into(),
            tiles: whole(key),
            reads_as: String::new(),
            columns,
        }],
    )
}
