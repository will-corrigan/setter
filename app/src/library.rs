//! The puzzles Home lists: every puzzle in the Setter folder, plus recent
//! ones kept elsewhere, each summarised for its row (size, progress, when it
//! was last edited). Pure apart from reading the files.

use crate::answers;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use xw_engine::puzzle::Puzzle;

/// One puzzle as Home shows it.
#[derive(Clone, PartialEq, Debug)]
pub struct Summary {
    pub path: PathBuf,
    /// The puzzle's title, or the file's name if it has none.
    pub title: String,
    pub author: String,
    pub width: usize,
    pub height: usize,
    /// Each square: true for a block. For the thumbnail.
    pub blocks: Vec<bool>,
    /// (answers with every square filled, all answers)
    pub filled: (usize, usize),
    /// (answers with a clue, all answers)
    pub clued: (usize, usize),
    pub modified: SystemTime,
}

pub fn summarise(path: &Path, p: &Puzzle, modified: SystemTime) -> Summary {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    Summary {
        path: path.to_path_buf(),
        title: if p.title.trim().is_empty() {
            stem
        } else {
            p.title.trim().to_string()
        },
        author: p.author.trim().to_string(),
        width: p.grid.width,
        height: p.grid.height,
        blocks: p.grid.cells.iter().map(|c| c.block).collect(),
        filled: answers::filled_count(p),
        clued: answers::clued_count(p),
        modified,
    }
}

/// Read one puzzle file for its row; `None` if it can't be read.
pub fn read(path: &Path) -> Option<Summary> {
    let p = crate::files::load(path).ok()?;
    let modified = fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    Some(summarise(path, &p, modified))
}

/// Every puzzle in `folder` and every recent one that still exists, newest
/// edit first, each once.
pub fn list(folder: &Path, recent: &[PathBuf]) -> Vec<Summary> {
    let mut paths: Vec<PathBuf> = fs::read_dir(folder)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.extension()
                        .is_some_and(|x| x.eq_ignore_ascii_case("xword"))
                })
                .collect()
        })
        .unwrap_or_default();
    paths.extend(recent.iter().cloned());
    let mut seen = HashSet::new();
    let mut out: Vec<Summary> = paths
        .into_iter()
        .filter(|p| seen.insert(fs::canonicalize(p).unwrap_or_else(|_| p.clone())))
        .filter_map(|p| read(&p))
        .collect();
    out.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.title.cmp(&b.title))
    });
    out
}

/// Does a row match what's typed in the filter? Title or setter, any case.
pub fn matches(s: &Summary, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    q.is_empty() || s.title.to_lowercase().contains(&q) || s.author.to_lowercase().contains(&q)
}

/// When something was last edited, in words: "just now", "5 minutes ago",
/// "yesterday", "3 days ago", then "12 Mar".
pub fn ago(then: SystemTime, now: SystemTime) -> String {
    let secs = now.duration_since(then).unwrap_or(Duration::ZERO).as_secs();
    let mins = secs / 60;
    let hours = mins / 60;
    let days = hours / 24;
    match () {
        _ if mins < 1 => "just now".to_string(),
        _ if mins == 1 => "a minute ago".to_string(),
        _ if hours < 1 => format!("{mins} minutes ago"),
        _ if hours == 1 => "an hour ago".to_string(),
        _ if days < 1 => format!("{hours} hours ago"),
        _ if days == 1 => "yesterday".to_string(),
        _ if days < 7 => format!("{days} days ago"),
        _ if days < 14 => "last week".to_string(),
        _ => format!("{} weeks ago", days / 7),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xw_engine::grid::Grid;

    fn puzzle(title: &str) -> Puzzle {
        let mut p = Puzzle::new(3, 3);
        p.grid = Grid::from_rows(&["c?t", "a#o", "bye"]);
        p.title = title.into();
        p.author = "Quixote".into();
        p.clues.entry("2,0,a".into()).or_default().text = "Farewell".into();
        p
    }

    #[test]
    fn a_summary_counts_progress_and_keeps_the_shape() {
        let s = summarise(
            Path::new("/x/Prize.xword"),
            &puzzle("Prize 12"),
            SystemTime::UNIX_EPOCH,
        );
        assert_eq!(s.title, "Prize 12");
        assert_eq!((s.width, s.height), (3, 3));
        assert_eq!(s.blocks.iter().filter(|b| **b).count(), 1);
        assert_eq!(s.filled, (3, 4));
        assert_eq!(s.clued, (1, 4));
        let untitled = summarise(
            Path::new("/x/Draft 2.xword"),
            &puzzle(" "),
            SystemTime::UNIX_EPOCH,
        );
        assert_eq!(untitled.title, "Draft 2", "the file name stands in");
    }

    #[test]
    fn the_list_has_the_folder_and_recent_files_newest_first_once_each() {
        let dir = crate::files::scratch_dir(&format!("library-list-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("elsewhere")).unwrap();
        let old = dir.join("Old.xword");
        let new = dir.join("New.xword");
        let away = dir.join("elsewhere").join("Away.xword");
        for (path, title) in [(&old, "Old"), (&new, "New"), (&away, "Away")] {
            crate::files::save(path, &puzzle(title)).unwrap();
        }
        let set = |p: &Path, secs: u64| {
            let f = fs::File::options().write(true).open(p).unwrap();
            f.set_modified(SystemTime::now() - Duration::from_secs(secs))
                .unwrap();
        };
        set(&old, 3000);
        set(&new, 10);
        set(&away, 500);
        fs::write(dir.join("notes.txt"), "not a puzzle").unwrap();
        let recent = vec![away.clone(), new.clone(), dir.join("gone.xword")];
        let titles: Vec<String> = list(&dir, &recent).into_iter().map(|s| s.title).collect();
        assert_eq!(titles, ["New", "Away", "Old"]);
    }

    #[test]
    fn the_filter_matches_title_or_setter() {
        let s = summarise(
            Path::new("/x/a.xword"),
            &puzzle("Prize 12"),
            SystemTime::UNIX_EPOCH,
        );
        assert!(matches(&s, "prize"));
        assert!(matches(&s, "quix"));
        assert!(matches(&s, ""));
        assert!(!matches(&s, "sunday"));
    }

    #[test]
    fn edit_times_read_naturally() {
        let now = SystemTime::now();
        let before = |secs: u64| now - Duration::from_secs(secs);
        assert_eq!(ago(before(5), now), "just now");
        assert_eq!(ago(before(5 * 60), now), "5 minutes ago");
        assert_eq!(ago(before(3600 + 60), now), "an hour ago");
        assert_eq!(ago(before(5 * 3600), now), "5 hours ago");
        assert_eq!(ago(before(30 * 3600), now), "yesterday");
        assert_eq!(ago(before(3 * 86400), now), "3 days ago");
        assert_eq!(ago(before(9 * 86400), now), "last week");
        assert_eq!(ago(before(30 * 86400), now), "4 weeks ago");
    }
}
