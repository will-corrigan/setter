//! The command palette's contents and its matching: every command with its
//! keys, every answer ("14a", "trees"), and recent puzzles, ranked against
//! what's typed. Pure; the shell runs the chosen target.

use crate::answers::Answer;
use crate::export::ExportFormat;
use crate::files::Theme;
use crate::state::Shortcut;
use std::path::PathBuf;
use xw_engine::puzzle::Puzzle;

#[derive(Clone, PartialEq, Debug)]
pub enum Command {
    Home,
    NewPuzzle,
    NewSample,
    Open,
    SaveAs,
    ShowInFolder,
    Export(ExportFormat),
    BuildGrid,
    WriteClues,
    Undo,
    Redo,
    FillRest,
    NextUnclued,
    PreviousAnswer,
    NextAnswer,
    Theme(Theme),
    About,
    Settings,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Target {
    Command(Command),
    /// An answer, by light id.
    Answer(String),
    Puzzle(PathBuf),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Group {
    Commands,
    Answers,
    Puzzles,
}

impl Group {
    pub fn title(self) -> &'static str {
        match self {
            Group::Commands => "Commands",
            Group::Answers => "Answers",
            Group::Puzzles => "Puzzles",
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Entry {
    pub group: Group,
    pub label: String,
    /// Small text after the label: "(5) · no clue".
    pub detail: String,
    pub keys: Option<String>,
    /// Other words that find it ("14a" for 14 across, "print" for export).
    pub aliases: Vec<String>,
    pub target: Target,
}

fn command(label: &str, keys_: Option<Shortcut>, aliases: &[&str], c: Command) -> Entry {
    Entry {
        group: Group::Commands,
        label: label.to_string(),
        detail: String::new(),
        keys: keys_.map(crate::bindings::label),
        aliases: aliases.iter().map(|a| a.to_string()).collect(),
        target: Target::Command(c),
    }
}

/// Every command, given what can be done right now.
pub fn commands(has_puzzle: bool, has_file: bool, can_undo: bool, can_redo: bool) -> Vec<Entry> {
    let mut out = vec![
        command("Home", None, &["puzzles", "library"], Command::Home),
        command(
            "New puzzle…",
            Some(Shortcut::New),
            &["grid", "template"],
            Command::NewPuzzle,
        ),
        command("New filled sample", None, &[], Command::NewSample),
        command("Open…", Some(Shortcut::Open), &["file"], Command::Open),
        command(
            "Theme: match the system",
            None,
            &["appearance"],
            Command::Theme(Theme::System),
        ),
        command(
            "Theme: light",
            None,
            &["appearance"],
            Command::Theme(Theme::Light),
        ),
        command(
            "Theme: dark",
            None,
            &["appearance", "night"],
            Command::Theme(Theme::Dark),
        ),
        command(
            "About Setter",
            None,
            &["credits", "licences"],
            Command::About,
        ),
        command(
            "Settings",
            Some(Shortcut::Settings),
            &[
                "preferences",
                "word lists",
                "abbreviations",
                "indicators",
                "shortcuts",
                "keys",
            ],
            Command::Settings,
        ),
    ];
    if has_puzzle {
        out.extend([
            command("Build grid", None, &["fill", "page"], Command::BuildGrid),
            command("Write clues", None, &["page"], Command::WriteClues),
            command(
                "Next unclued answer",
                Some(Shortcut::NextUnclued),
                &["skip"],
                Command::NextUnclued,
            ),
            command(
                "Previous answer",
                Some(Shortcut::PreviousAnswer),
                &["back"],
                Command::PreviousAnswer,
            ),
            command(
                "Next answer",
                Some(Shortcut::NextAnswer),
                &["forward"],
                Command::NextAnswer,
            ),
            command("Fill the rest", None, &["autofill"], Command::FillRest),
            command(
                "Save as…",
                Some(Shortcut::SaveAs),
                &["rename", "copy", "move"],
                Command::SaveAs,
            ),
            command(
                "Export web page…",
                Some(Shortcut::Export),
                &["html", "share", "print"],
                Command::Export(ExportFormat::WebPage),
            ),
            command(
                "Export Across Lite (.puz)…",
                None,
                &["puz"],
                Command::Export(ExportFormat::Puz),
            ),
            command(
                "Export ipuz…",
                None,
                &["ipuz"],
                Command::Export(ExportFormat::Ipuz),
            ),
        ]);
    }
    if has_file {
        out.push(command(
            "Show in folder",
            None,
            &["reveal", "finder", "explorer"],
            Command::ShowInFolder,
        ));
    }
    if can_undo {
        out.push(command("Undo", Some(Shortcut::Undo), &[], Command::Undo));
    }
    if can_redo {
        out.push(command("Redo", Some(Shortcut::Redo), &[], Command::Redo));
    }
    out
}

/// "10a" → "10 across".
fn spoken(label: &str) -> String {
    let (n, d) = label.split_at(label.len().saturating_sub(1));
    match d {
        "a" => format!("{n} across"),
        "d" => format!("{n} down"),
        _ => label.to_string(),
    }
}

/// An entry for each answer: "14 across · WHO", found by "14a", "14 across"
/// or its letters.
pub fn answer_entries(list: &[Answer], p: &Puzzle) -> Vec<Entry> {
    list.iter()
        .map(|a| {
            let clued = p
                .clues
                .get(&a.light_id)
                .is_some_and(|c| !c.text.trim().is_empty());
            let letters = a.pattern.to_uppercase().replace('?', "·");
            Entry {
                group: Group::Answers,
                label: format!("{} · {}", spoken(&a.label), letters),
                detail: if clued {
                    format!("({})", a.enumeration)
                } else {
                    format!("({}) · no clue", a.enumeration)
                },
                keys: None,
                aliases: vec![a.label.clone(), a.pattern.replace('?', "")],
                target: Target::Answer(a.light_id.clone()),
            }
        })
        .collect()
}

pub fn puzzle_entries(recent: &[PathBuf]) -> Vec<Entry> {
    recent
        .iter()
        .map(|path| Entry {
            group: Group::Puzzles,
            label: path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            detail: String::new(),
            keys: None,
            aliases: vec![],
            target: Target::Puzzle(path.clone()),
        })
        .collect()
}

/// How well `text` matches `query`, and which byte ranges of `text` to show
/// bold. Whole-word starts beat the middle of words, which beat scattered
/// letters; `None` if the letters aren't all there in order.
pub fn score(text: &str, query: &str) -> Option<(i32, Vec<(usize, usize)>)> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Some((0, vec![]));
    }
    let t = text.to_lowercase();
    if t.len() != text.len() {
        // Lowercasing changed lengths (rare letters): match without marks.
        return t.contains(&q).then(|| (40, vec![]));
    }
    if let Some(at) = t.find(&q) {
        let word_start = at == 0 || !t.as_bytes()[at - 1].is_ascii_alphanumeric();
        let exact = q.len() == t.len();
        let s = if exact {
            150
        } else if at == 0 {
            120
        } else if word_start {
            100
        } else {
            60
        } - at.min(40) as i32;
        return Some((s, vec![(at, at + q.len())]));
    }
    // Scattered letters, in order.
    let mut marks = vec![];
    let mut from = 0;
    for qc in q.chars() {
        let found = t[from..].char_indices().find(|(_, c)| *c == qc)?;
        let at = from + found.0;
        marks.push((at, at + qc.len_utf8()));
        from = at + qc.len_utf8();
    }
    Some((20 - marks.len() as i32, marks))
}

/// An entry that matched, how well, and what to show bold in its label.
#[derive(Clone, PartialEq, Debug)]
pub struct Hit {
    pub entry: Entry,
    pub score: i32,
    pub marks: Vec<(usize, usize)>,
}

fn rate(e: &Entry, query: &str) -> Option<Hit> {
    let hit = |score: i32, marks: Vec<(usize, usize)>| Hit {
        entry: e.clone(),
        score,
        marks,
    };
    let label = score(&e.label, query);
    // Only a real match on another name counts, not scattered letters.
    let alias = e
        .aliases
        .iter()
        .filter_map(|a| score(a, query).map(|(s, _)| s))
        .filter(|s| *s >= 60)
        .max();
    match (label, alias) {
        (Some((s, marks)), Some(a)) if a > s => Some(hit(a, marks)),
        (Some((s, marks)), _) => Some(hit(s, marks)),
        (None, Some(a)) => Some(hit(a, vec![])),
        (None, None) => None,
    }
}

/// What the palette shows: matching entries grouped (commands, answers,
/// puzzles), best first within each, at most `per_group` each. With nothing
/// typed: the recent puzzles and a few everyday commands.
pub fn search(entries: &[Entry], query: &str, per_group: usize) -> Vec<(Group, Vec<Hit>)> {
    let mut hits: Vec<Hit> = if query.trim().is_empty() {
        const EVERYDAY: [&str; 4] = [
            "Next unclued answer",
            "New puzzle…",
            "Export web page…",
            "Open…",
        ];
        entries
            .iter()
            .filter(|e| e.group == Group::Puzzles || EVERYDAY.contains(&e.label.as_str()))
            .map(|e| Hit {
                entry: e.clone(),
                score: 0,
                marks: vec![],
            })
            .collect()
    } else {
        entries.iter().filter_map(|e| rate(e, query)).collect()
    };
    hits.sort_by_key(|h| std::cmp::Reverse(h.score));
    let mut out: Vec<(Group, Vec<Hit>)> = vec![];
    for g in [Group::Commands, Group::Answers, Group::Puzzles] {
        let in_group: Vec<Hit> = hits
            .iter()
            .filter(|h| h.entry.group == g)
            .take(per_group)
            .cloned()
            .collect();
        if !in_group.is_empty() {
            out.push((g, in_group));
        }
    }
    // The group holding the best match comes first.
    out.sort_by_key(|(_, hits)| -hits[0].score);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::answers;
    use xw_engine::grid::Grid;

    fn puzzle() -> Puzzle {
        let mut p = Puzzle::new(3, 3);
        p.grid = Grid::from_rows(&["cat", "a#o", "bye"]);
        p.clues.entry("0,0,a".into()).or_default().text = "Pet".into();
        p
    }

    fn entries() -> Vec<Entry> {
        let p = puzzle();
        let mut all = commands(true, true, true, false);
        all.extend(answer_entries(&answers::answers(&p), &p));
        all.extend(puzzle_entries(&[PathBuf::from("/p/Sunday extra.xword")]));
        all
    }

    fn labels(groups: &[(Group, Vec<Hit>)]) -> Vec<(Group, Vec<String>)> {
        groups
            .iter()
            .map(|(g, hits)| (*g, hits.iter().map(|h| h.entry.label.clone()).collect()))
            .collect()
    }

    #[test]
    fn word_starts_beat_middles_beat_scattered_letters() {
        let start = score("Export web page…", "ex").unwrap();
        let word = score("Sunday extra", "ex").unwrap();
        let middle = score("Next answer", "ex").unwrap();
        let scattered = score("Open existing", "oxt").unwrap();
        assert!(start.0 > word.0 && word.0 > middle.0 && middle.0 > scattered.0);
        assert_eq!(start.1, vec![(0, 2)]);
        assert_eq!(score("Open…", "zz"), None);
    }

    #[test]
    fn typing_finds_commands_answers_and_puzzles() {
        let found = labels(&search(&entries(), "ex", 5));
        let commands = &found.iter().find(|(g, _)| *g == Group::Commands).unwrap().1;
        assert_eq!(commands[0], "Export web page…");
        assert!(found
            .iter()
            .any(|(g, l)| *g == Group::Puzzles && l.contains(&"Sunday extra".to_string())));
    }

    #[test]
    fn an_answer_is_found_by_its_number_or_its_letters() {
        let by_number = labels(&search(&entries(), "1a", 5));
        assert_eq!(by_number[0].0, Group::Answers);
        assert_eq!(
            by_number[0].1[0], "1 across · CAT",
            "the exact number first"
        );
        let by_letters = labels(&search(&entries(), "bye", 5));
        assert!(
            by_letters
                .iter()
                .any(|(g, l)| *g == Group::Answers && l[0] == "3 across · BYE"),
            "{by_letters:?}"
        );
        let e = answer_entries(&answers::answers(&puzzle()), &puzzle());
        assert_eq!(e[0].detail, "(3)");
        assert_eq!(e[1].detail, "(3) · no clue");
    }

    #[test]
    fn nothing_typed_shows_recent_puzzles_and_everyday_commands() {
        let shown = labels(&search(&entries(), "", 5));
        let all: Vec<String> = shown.iter().flat_map(|(_, l)| l.clone()).collect();
        assert!(all.contains(&"Sunday extra".to_string()));
        assert!(all.contains(&"Next unclued answer".to_string()));
        assert!(
            !all.iter().any(|l| l.contains("across")),
            "no answers until asked"
        );
    }

    #[test]
    fn commands_follow_what_can_be_done() {
        let fresh = commands(false, false, false, false);
        let names: Vec<&str> = fresh.iter().map(|e| e.label.as_str()).collect();
        assert!(
            !names.contains(&"Undo")
                && !names.contains(&"Save as…")
                && !names.contains(&"Show in folder")
        );
        assert!(names.contains(&"Open…"));
        let keys_shown = commands(true, true, true, true)
            .into_iter()
            .find(|e| e.label == "Undo")
            .and_then(|e| e.keys);
        assert_eq!(keys_shown, Some(crate::keys::label("Ctrl+Z")));
    }

    #[test]
    fn aliases_find_commands_under_other_names() {
        let found = labels(&search(&entries(), "credits", 5));
        assert_eq!(found[0].1[0], "About Setter");
    }
}
