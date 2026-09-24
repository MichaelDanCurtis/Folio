//! Opt-in per-stage timing for the edit-path benchmark. Recording is off unless
//! the current thread enables it, so production edits pay one flag read.
use std::cell::RefCell;
use std::time::Instant;

thread_local! {
    static STAGES: RefCell<Option<Vec<(&'static str, f64)>>> = const { RefCell::new(None) };
}

#[cfg(test)]
pub fn enable(on: bool) {
    STAGES.with(|s| *s.borrow_mut() = on.then(Vec::new));
}

pub fn time<T>(label: &'static str, f: impl FnOnce() -> T) -> T {
    if STAGES.with(|s| s.borrow().is_none()) {
        return f();
    }
    let started = Instant::now();
    let result = f();
    record(label, started);
    result
}

/// Records time since `started`, for stages that don't fit in a closure.
pub fn record(label: &'static str, started: Instant) {
    STAGES.with(|s| {
        if let Some(stages) = s.borrow_mut().as_mut() {
            stages.push((label, started.elapsed().as_secs_f64() * 1000.0));
        }
    });
}

/// Returns and clears the stages recorded since the last call.
#[cfg(test)]
pub fn take() -> Vec<(&'static str, f64)> {
    STAGES.with(|s| {
        s.borrow_mut()
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_nothing_while_disabled() {
        enable(false);
        assert_eq!(time("a", || 1), 1);
        assert!(take().is_empty());
    }

    #[test]
    fn records_labels_in_order_while_enabled() {
        enable(true);
        time("a", || ());
        time("b", || time("c", || ()));
        let labels: Vec<_> = take().into_iter().map(|(l, _)| l).collect();
        assert_eq!(labels, ["a", "c", "b"]);
        assert!(take().is_empty());
        enable(false);
    }
}
