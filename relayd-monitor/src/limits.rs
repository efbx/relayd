use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use relayd_storage::Retention;

#[derive(Clone, Debug)]
pub struct Limits(Arc<Mutex<Kept>>);

#[derive(Clone, Copy, Debug)]
struct Kept {
    history: Retention,
    captures: Retention,
    body: usize,
}

impl Limits {
    #[must_use]
    pub fn new(history: Retention, captures: Retention, body: usize) -> Self {
        Self(Arc::new(Mutex::new(Kept {
            history,
            captures,
            body,
        })))
    }

    pub fn set(&self, history: Retention, captures: Retention, body: usize) {
        *self.kept() = Kept {
            history,
            captures,
            body,
        };
    }

    pub(crate) fn history(&self) -> Option<Retention> {
        let history = self.kept().history;
        (history.keep > 0).then_some(history)
    }

    pub(crate) fn captures(&self) -> Option<Retention> {
        let Kept {
            history, captures, ..
        } = *self.kept();
        (history.keep > 0 && captures.keep > 0).then_some(captures)
    }

    pub(crate) fn body(&self) -> usize {
        self.kept().body
    }

    fn kept(&self) -> MutexGuard<'_, Kept> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HISTORY: Retention = Retention {
        keep: 100,
        every: 256,
    };
    const CAPTURES: Retention = Retention { keep: 10, every: 5 };
    const OFF: Retention = Retention { keep: 0, every: 5 };
    const BODY: usize = 1 << 20;

    #[test]
    fn both_are_on_when_both_keep_something() {
        let limits = Limits::new(HISTORY, CAPTURES, BODY);
        assert_eq!(limits.history(), Some(HISTORY));
        assert_eq!(limits.captures(), Some(CAPTURES));
    }

    #[test]
    fn zero_history_turns_contents_off_too() {
        let limits = Limits::new(OFF, CAPTURES, BODY);
        assert_eq!(limits.history(), None);
        assert_eq!(limits.captures(), None);
    }

    #[test]
    fn zero_contents_leaves_history_on() {
        let limits = Limits::new(HISTORY, OFF, BODY);
        assert_eq!(limits.history(), Some(HISTORY));
        assert_eq!(limits.captures(), None);
    }

    #[test]
    fn a_zero_body_limit_leaves_contents_on() {
        let limits = Limits::new(HISTORY, CAPTURES, 0);
        assert_eq!(limits.captures(), Some(CAPTURES));
        assert_eq!(limits.body(), 0);
    }

    #[test]
    fn new_limits_reach_every_clone() {
        let limits = Limits::new(HISTORY, CAPTURES, BODY);
        let other = limits.clone();
        other.set(CAPTURES, HISTORY, 7);
        assert_eq!(limits.history(), Some(CAPTURES));
        assert_eq!(limits.captures(), Some(HISTORY));
        assert_eq!(limits.body(), 7);
    }
}
