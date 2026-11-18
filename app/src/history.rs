//! Undo/redo for whole-puzzle snapshots. Changes of the same kind in a quick
//! run (typing one clue) group into a single step.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// A pause longer than this starts a new step, even while typing the same clue.
pub const GROUP_PAUSE: Duration = Duration::from_millis(1200);

/// Snapshots to undo to, and to redo to, capped in size.
pub struct History<T> {
    undo: VecDeque<T>,
    redo: Vec<T>,
    cap: usize,
    /// The group of the last recorded change and when it was made.
    last: Option<(String, Instant)>,
}

impl<T> History<T> {
    pub fn new(cap: usize) -> Self {
        History {
            undo: VecDeque::new(),
            redo: vec![],
            cap,
            last: None,
        }
    }

    /// Record the state before a change, as a step of its own.
    pub fn push(&mut self, before: T) {
        self.last = None;
        self.store(before);
    }

    /// Record the state before a change that belongs to `group` (e.g. typing
    /// in one clue). A change in the same group soon after the last one joins
    /// that step instead of making a new one.
    pub fn record(&mut self, before: T, group: &str, at: Instant) {
        let joins = self.last.as_ref().is_some_and(|(g, when)| {
            g == group && at.saturating_duration_since(*when) <= GROUP_PAUSE
        });
        self.last = Some((group.to_string(), at));
        if !joins {
            self.store(before);
        }
    }

    fn store(&mut self, before: T) {
        self.undo.push_back(before);
        if self.undo.len() > self.cap {
            self.undo.pop_front();
        }
        self.redo.clear();
    }

    pub fn undo(&mut self, current: T) -> Option<T> {
        let prev = self.undo.pop_back()?;
        self.last = None;
        self.redo.push(current);
        Some(prev)
    }

    pub fn redo(&mut self, current: T) -> Option<T> {
        let next = self.redo.pop()?;
        self.last = None;
        self.undo.push_back(current);
        Some(next)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_redo_and_cap() {
        let mut h = History::new(2);
        h.push(1);
        h.push(2);
        h.push(3); // 1 falls off
        assert_eq!(h.undo(4), Some(3));
        assert_eq!(h.undo(3), Some(2));
        assert_eq!(h.undo(2), None);
        assert_eq!(h.redo(2), Some(3));
        h.push(10);
        assert_eq!(h.redo(10), None, "a new change clears redo");
    }

    #[test]
    fn a_quick_run_of_typing_is_one_step() {
        let t = Instant::now();
        let ms = |n| t + Duration::from_millis(n);
        let mut h = History::new(50);
        // "G", "Go", "Gol", "Gold" typed quickly in one clue.
        h.record("", "clue:1a", ms(0));
        h.record("G", "clue:1a", ms(200));
        h.record("Go", "clue:1a", ms(400));
        h.record("Gol", "clue:1a", ms(600));
        assert_eq!(h.undo("Gold"), Some(""), "back to before the run");
        assert!(!h.can_undo());
    }

    #[test]
    fn a_pause_or_another_clue_starts_a_new_step() {
        let t = Instant::now();
        let ms = |n| t + Duration::from_millis(n);
        let mut h = History::new(50);
        h.record("a", "clue:1a", ms(0));
        h.record("ab", "clue:1a", ms(5000)); // after a pause
        h.record("x", "clue:2d", ms(5100)); // another clue
        h.push("grid"); // anything else is its own step
        h.record("y", "clue:2d", ms(5200)); // after another change: new step
        assert_eq!(h.undo("now"), Some("y"));
        assert_eq!(h.undo("y"), Some("grid"));
        assert_eq!(h.undo("grid"), Some("x"));
        assert_eq!(h.undo("x"), Some("ab"));
        assert_eq!(h.undo("ab"), Some("a"));
    }

    #[test]
    fn after_undo_typing_starts_afresh() {
        let t = Instant::now();
        let mut h = History::new(50);
        h.record("a", "clue:1a", t);
        assert_eq!(h.undo("ab"), Some("a"));
        h.record("a", "clue:1a", t + Duration::from_millis(100));
        assert!(h.can_undo(), "the new typing is a step");
        assert!(!h.can_redo());
    }
}
