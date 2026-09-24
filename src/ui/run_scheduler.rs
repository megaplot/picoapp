use crate::inputs::InputValue;

/// Latest-value-wins coalescing for one reactive level's inputs. Pure Rust:
/// no gpui or pyo3 types, so it's fully unit-testable and independently
/// reusable if the UI layer changes.
pub struct RunScheduler {
    values: Vec<InputValue>,
    in_flight: bool,
    dirty: bool,
}

impl RunScheduler {
    pub fn new(initial_values: Vec<InputValue>) -> Self {
        RunScheduler {
            values: initial_values,
            in_flight: false,
            dirty: false,
        }
    }

    pub fn current_values(&self) -> &[InputValue] {
        &self.values
    }

    /// Marks the level's very first job (auto-triggered on construction,
    /// not from a UI change) as in flight, and returns the values to run it
    /// with. Without this, a change arriving while that first job is still
    /// running would see `in_flight == false` and fire a second, concurrent
    /// job instead of coalescing into the pending set.
    pub fn start(&mut self) -> Vec<InputValue> {
        self.in_flight = true;
        self.values.clone()
    }

    /// Records a UI-driven change to input `index`. Returns the full value
    /// vector to dispatch now if nothing is in flight, or `None` if the
    /// change was folded into the pending set for when the in-flight job
    /// finishes.
    pub fn on_change(&mut self, index: usize, value: InputValue) -> Option<Vec<InputValue>> {
        // Widgets report events, not changes: a slider emits one on every
        // mouse move even while its (integer-rounded) value stays put.
        // Re-running the callback for an unchanged value is pure waste —
        // and for a callback that rebuilds nested inputs, harmful.
        if self.values[index] == value {
            return None;
        }
        self.values[index] = value;
        if self.in_flight {
            self.dirty = true;
            None
        } else {
            self.in_flight = true;
            Some(self.values.clone())
        }
    }

    /// Call when a dispatched job's result arrives. Returns the value
    /// vector for a follow-up job if changes were coalesced while the job
    /// was running, or `None` if the scheduler is now idle.
    pub fn on_result(&mut self) -> Option<Vec<InputValue>> {
        self.in_flight = false;
        if self.dirty {
            self.dirty = false;
            self.in_flight = true;
            Some(self.values.clone())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(v: f64) -> InputValue {
        InputValue::F64(v)
    }

    #[test]
    fn first_change_dispatches_immediately() {
        let mut s = RunScheduler::new(vec![f(0.0)]);
        let dispatched = s.on_change(0, f(1.0));
        assert_eq!(dispatched, Some(vec![f(1.0)]));
    }

    #[test]
    fn change_while_in_flight_is_coalesced_not_dispatched() {
        let mut s = RunScheduler::new(vec![f(0.0)]);
        assert!(s.on_change(0, f(1.0)).is_some()); // now in flight

        assert_eq!(s.on_change(0, f(2.0)), None);
        assert_eq!(s.on_change(0, f(3.0)), None); // overwrites, doesn't queue
    }

    #[test]
    fn result_dispatches_the_latest_coalesced_value() {
        let mut s = RunScheduler::new(vec![f(0.0)]);
        s.on_change(0, f(1.0)); // in flight with [1.0]
        s.on_change(0, f(2.0)); // coalesced
        s.on_change(0, f(3.0)); // coalesced, overwrites 2.0

        let followup = s.on_result();
        assert_eq!(followup, Some(vec![f(3.0)]));
    }

    #[test]
    fn result_with_no_pending_change_dispatches_nothing() {
        let mut s = RunScheduler::new(vec![f(0.0)]);
        s.on_change(0, f(1.0));
        assert_eq!(s.on_result(), None);
    }

    #[test]
    fn dispatch_after_result_can_be_coalesced_again() {
        let mut s = RunScheduler::new(vec![f(0.0)]);
        s.on_change(0, f(1.0));
        s.on_change(0, f(2.0));
        assert_eq!(s.on_result(), Some(vec![f(2.0)])); // still in flight now

        assert_eq!(s.on_change(0, f(3.0)), None); // coalesced again
        assert_eq!(s.on_result(), Some(vec![f(3.0)]));
        assert_eq!(s.on_result(), None); // idle now, nothing pending
    }

    #[test]
    fn multiple_inputs_change_independently() {
        let mut s = RunScheduler::new(vec![f(0.0), f(0.0)]);
        assert_eq!(s.on_change(1, f(9.0)), Some(vec![f(0.0), f(9.0)]));
        assert_eq!(s.on_change(0, f(5.0)), None);
        assert_eq!(s.on_result(), Some(vec![f(5.0), f(9.0)]));
    }

    #[test]
    fn unchanged_value_does_not_dispatch() {
        // An IntSlider emits a change event on every mouse move even while
        // its integer value stays the same; that must not re-run the callback.
        let mut s = RunScheduler::new(vec![f(3.0)]);
        assert_eq!(s.on_change(0, f(3.0)), None);
        // ...and it left the scheduler idle: a real change still dispatches.
        assert_eq!(s.on_change(0, f(4.0)), Some(vec![f(4.0)]));
    }

    #[test]
    fn unchanged_value_while_in_flight_does_not_queue_a_rerun() {
        let mut s = RunScheduler::new(vec![f(0.0)]);
        s.on_change(0, f(1.0)); // in flight with 1.0
        assert_eq!(s.on_change(0, f(1.0)), None);
        assert_eq!(s.on_result(), None); // nothing pending: no redundant re-run
    }

    #[test]
    fn start_marks_in_flight_so_a_change_during_it_coalesces() {
        let mut s = RunScheduler::new(vec![f(0.0)]);
        let dispatched = s.start();
        assert_eq!(dispatched, vec![f(0.0)]);

        // A change arriving while the level's very first (auto-triggered)
        // job is running must coalesce, not fire a second concurrent job.
        assert_eq!(s.on_change(0, f(1.0)), None);
        assert_eq!(s.on_result(), Some(vec![f(1.0)]));
    }
}
