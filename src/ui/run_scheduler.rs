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

    /// Records a UI-driven change to input `index`. Returns the full value
    /// vector to dispatch now if nothing is in flight, or `None` if the
    /// change was folded into the pending set for when the in-flight job
    /// finishes.
    pub fn on_change(&mut self, index: usize, value: InputValue) -> Option<Vec<InputValue>> {
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
}
