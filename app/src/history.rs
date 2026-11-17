//! Undo/redo for whole-puzzle snapshots.

/// Snapshots to undo to, and to redo to, capped in size.
pub struct History<T> {
    undo: Vec<T>,
    redo: Vec<T>,
    cap: usize,
}

impl<T> History<T> {
    pub fn new(cap: usize) -> Self {
        History {
            undo: vec![],
            redo: vec![],
            cap,
        }
    }

    /// Record the state before a change.
    pub fn push(&mut self, before: T) {
        self.undo.push(before);
        if self.undo.len() > self.cap {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self, current: T) -> Option<T> {
        let prev = self.undo.pop()?;
        self.redo.push(current);
        Some(prev)
    }

    pub fn redo(&mut self, current: T) -> Option<T> {
        let next = self.redo.pop()?;
        self.undo.push(current);
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
}
