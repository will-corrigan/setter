//! Autosave: every edit is kept in the config folder, and recovered on start
//! if it's newer than the last saved file.

use std::fs;
use std::path::{Path, PathBuf};
use xw_engine::puzzle::Puzzle;

pub fn autosave_path(dir: &Path) -> PathBuf {
    dir.join("autosave.xword")
}

pub fn write_autosave(p: &Puzzle) -> Result<(), String> {
    write_autosave_in(&crate::files::config_dir(), p)
}

pub fn write_autosave_in(dir: &Path, p: &Puzzle) -> Result<(), String> {
    crate::files::save(&autosave_path(dir), p)
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
        Some(p) if Some(p) == pristine => Step::Remove,
        Some(_) => Step::Write,
    }
}

pub fn remove_autosave() {
    remove_autosave_in(&crate::files::config_dir());
}

pub fn remove_autosave_in(dir: &Path) {
    let _ = fs::remove_file(autosave_path(dir));
}

pub fn recovery(last_file: Option<&Path>) -> Option<Puzzle> {
    recovery_in(&crate::files::config_dir(), last_file)
}

/// The autosaved puzzle, if it holds work the last saved file doesn't:
/// there's no saved file, or the autosave is newer and different.
pub fn recovery_in(dir: &Path, last_file: Option<&Path>) -> Option<Puzzle> {
    let path = autosave_path(dir);
    let auto = crate::files::load(&path).ok()?;
    let Some(saved_path) = last_file else {
        return Some(auto);
    };
    let Ok(saved) = crate::files::load(saved_path) else {
        return Some(auto);
    };
    let modified = |p: &Path| fs::metadata(p).and_then(|m| m.modified()).ok();
    let newer = match (modified(&path), modified(saved_path)) {
        (Some(a), Some(s)) => a > s,
        _ => true,
    };
    (newer && auto != saved).then_some(auto)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{Duration, SystemTime};
    use xw_engine::puzzle::Puzzle;

    fn dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("setter-autosave-{name}-{}", std::process::id()));
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
        write_autosave_in(&d, &Puzzle::new(3, 3)).unwrap();
        remove_autosave_in(&d);
        assert!(recovery_in(&d, None).is_none());
        remove_autosave_in(&d); // nothing there: no fuss
    }

    #[test]
    fn nothing_to_recover_without_an_autosave() {
        assert!(recovery_in(&dir("none"), None).is_none());
    }

    #[test]
    fn newer_autosave_is_recovered() {
        let d = dir("newer");
        let mut p = Puzzle::new(3, 3);
        p.title = "Draft".into();
        write_autosave_in(&d, &p).unwrap();
        assert_eq!(recovery_in(&d, None).unwrap().title, "Draft");
        let saved = d.join("mine.xword");
        crate::files::save(&saved, &Puzzle::new(3, 3)).unwrap();
        touch(&saved, 60);
        assert_eq!(recovery_in(&d, Some(&saved)).unwrap().title, "Draft");
    }

    #[test]
    fn older_or_identical_autosave_is_ignored() {
        let d = dir("older");
        let p = Puzzle::new(3, 3);
        write_autosave_in(&d, &p).unwrap();
        touch(&autosave_path(&d), 60);
        let saved = d.join("mine.xword");
        let mut newer = Puzzle::new(3, 3);
        newer.title = "Saved".into();
        crate::files::save(&saved, &newer).unwrap();
        assert!(recovery_in(&d, Some(&saved)).is_none(), "older autosave");
        write_autosave_in(&d, &newer).unwrap();
        assert!(
            recovery_in(&d, Some(&saved)).is_none(),
            "identical to the saved file"
        );
    }
}
