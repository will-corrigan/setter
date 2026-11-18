//! Autosave: every edit is kept in the config folder, and recovered on start
//! if it's newer than the last saved file.

use std::fs;
use std::path::{Path, PathBuf};
use xw_engine::puzzle::Puzzle;

pub fn autosave_path(dir: &Path) -> PathBuf {
    dir.join("autosave.xword")
}

/// Beside the autosave: the file the work was a change to, if it had one.
fn source_path(dir: &Path) -> PathBuf {
    dir.join("autosave.from")
}

/// What autosave should do now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The puzzle differs from how it was last opened or saved: write it.
    Write,
    /// Nothing unsaved: an old autosave would only bring back stale work.
    Remove,
    Nothing,
}

/// Compare the puzzle on screen with how it was when last shown or saved.
/// `pristine` is `None` when there is no clean copy, e.g. recovered work.
pub fn step(puzzle: Option<&Puzzle>, pristine: Option<&Puzzle>) -> Step {
    match puzzle {
        None => Step::Nothing,
        Some(_) if !crate::state::unsaved(puzzle, pristine) => Step::Remove,
        Some(_) => Step::Write,
    }
}

pub fn write_autosave_in(dir: &Path, p: &Puzzle, source: Option<&Path>) -> Result<(), String> {
    crate::files::save(&autosave_path(dir), p)?;
    match source {
        Some(path) => {
            crate::files::write_whole(&source_path(dir), path.to_string_lossy().as_bytes())
                .map_err(|e| format!("Couldn't autosave: {e}"))
        }
        None => {
            let _ = fs::remove_file(source_path(dir));
            Ok(())
        }
    }
}

pub fn remove_autosave() {
    remove_autosave_in(&crate::files::config_dir());
}

pub fn remove_autosave_in(dir: &Path) {
    let _ = fs::remove_file(autosave_path(dir));
    let _ = fs::remove_file(source_path(dir));
}

/// Unsaved work, and the file it was a change to.
pub type Recovered = (Puzzle, Option<PathBuf>);

pub fn recovery() -> Option<Recovered> {
    recovery_in(&crate::files::config_dir())
}

/// The autosaved work, unless the file it was a change to has since been
/// saved with the same contents, or written again after it.
pub fn recovery_in(dir: &Path) -> Option<Recovered> {
    let path = autosave_path(dir);
    let auto = crate::files::load(&path).ok()?;
    let source = fs::read_to_string(source_path(dir))
        .ok()
        .map(|s| PathBuf::from(s.trim()))
        .filter(|p| !p.as_os_str().is_empty());
    let Some(saved_path) = source.clone() else {
        return Some((auto, None));
    };
    let Ok(saved) = crate::files::load(&saved_path) else {
        // The file has gone: the work is all there is.
        return Some((auto, None));
    };
    let modified = |p: &Path| fs::metadata(p).and_then(|m| m.modified()).ok();
    let newer = match (modified(&path), modified(&saved_path)) {
        (Some(a), Some(s)) => a >= s,
        _ => true,
    };
    (newer && auto != saved).then_some((auto, source))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{Duration, SystemTime};
    use xw_engine::puzzle::Puzzle;

    fn dir(name: &str) -> std::path::PathBuf {
        let d = crate::files::scratch_dir(&format!("autosave-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn touch(path: &std::path::Path, secs_ago: u64) {
        let f = fs::File::options().write(true).open(path).unwrap();
        f.set_modified(SystemTime::now() - Duration::from_secs(secs_ago))
            .unwrap();
    }

    #[test]
    fn writes_only_what_differs_from_the_clean_copy() {
        let clean = Puzzle::new(3, 3);
        let mut edited = clean.clone();
        edited.title = "Edited".into();
        assert_eq!(step(None, None), Step::Nothing);
        assert_eq!(step(Some(&clean), Some(&clean)), Step::Remove);
        assert_eq!(step(Some(&edited), Some(&clean)), Step::Write);
        // Recovered work has no clean copy until it is saved.
        assert_eq!(step(Some(&edited), None), Step::Write);
    }

    #[test]
    fn removing_clears_the_autosave() {
        let d = dir("remove");
        write_autosave_in(&d, &Puzzle::new(3, 3), Some(&d.join("a.xword"))).unwrap();
        remove_autosave_in(&d);
        assert!(recovery_in(&d).is_none());
        assert!(!source_path(&d).exists());
        remove_autosave_in(&d); // nothing there: no fuss
    }

    #[test]
    fn nothing_to_recover_without_an_autosave() {
        assert!(recovery_in(&dir("none")).is_none());
    }

    #[test]
    fn work_on_a_new_puzzle_is_recovered_without_a_file() {
        let d = dir("new");
        let mut p = Puzzle::new(3, 3);
        p.title = "Draft".into();
        write_autosave_in(&d, &p, None).unwrap();
        let (got, from) = recovery_in(&d).unwrap();
        assert_eq!((got.title.as_str(), from), ("Draft", None));
    }

    #[test]
    fn changes_to_a_file_are_recovered_with_that_file() {
        let d = dir("newer");
        let saved = d.join("mine.xword");
        crate::files::save(&saved, &Puzzle::new(3, 3)).unwrap();
        touch(&saved, 60);
        let mut p = Puzzle::new(3, 3);
        p.title = "Draft".into();
        write_autosave_in(&d, &p, Some(&saved)).unwrap();
        let (got, from) = recovery_in(&d).unwrap();
        assert_eq!(got.title, "Draft");
        assert_eq!(from, Some(saved));
    }

    #[test]
    fn work_since_saved_or_overtaken_is_not_recovered() {
        let d = dir("older");
        let saved = d.join("mine.xword");
        let mut newer = Puzzle::new(3, 3);
        newer.title = "Saved".into();
        // Saved again after the autosave.
        write_autosave_in(&d, &Puzzle::new(3, 3), Some(&saved)).unwrap();
        touch(&autosave_path(&d), 60);
        crate::files::save(&saved, &newer).unwrap();
        assert!(recovery_in(&d).is_none(), "older autosave");
        // The same as the file.
        write_autosave_in(&d, &newer, Some(&saved)).unwrap();
        assert!(recovery_in(&d).is_none(), "identical to the saved file");
    }
}
