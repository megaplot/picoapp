/// Generates "nice" (1, 2, or 5 times a power of 10) tick values covering
/// `[min, max]`, aiming for roughly `target_count` ticks.
pub fn nice_ticks(min: f64, max: f64, target_count: usize) -> Vec<f64> {
    if !(max > min) {
        return vec![min];
    }
    let span = max - min;
    let raw_step = span / (target_count.max(1) as f64);
    let step = nice_step(raw_step);

    let start = (min / step).ceil() * step;
    let mut ticks = Vec::new();
    let mut t = start;
    // Snap to avoid float-accumulation drift over many steps.
    let mut i = 0i64;
    while t <= max + step * 1e-9 {
        ticks.push(round_to_step(t, step));
        i += 1;
        t = start + step * i as f64;
    }
    ticks
}

fn nice_step(raw_step: f64) -> f64 {
    let magnitude = 10f64.powf(raw_step.log10().floor());
    let normalized = raw_step / magnitude;
    // Standard "1-2-5" thresholds (as used by D3 and most nice-tick
    // algorithms): geometric-ish midpoints between 1, 2, 5 and 10, not the
    // arithmetic midpoints. Using <=1/<=2/<=5 instead would pick a step
    // that overshoots the target tick count (e.g. it would pick 5 instead
    // of 2 for a normalized value of 2.25).
    let nice = if normalized < 1.5 {
        1.0
    } else if normalized < 3.0 {
        2.0
    } else if normalized < 7.0 {
        5.0
    } else {
        10.0
    };
    nice * magnitude
}

fn round_to_step(value: f64, step: f64) -> f64 {
    let decimals = (-step.log10()).ceil().max(0.0) as i32;
    let factor = 10f64.powi(decimals);
    (value * factor).round() / factor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evenly_spaced_range_picks_round_step() {
        let ticks = nice_ticks(0.0, 10.0, 5);
        assert_eq!(ticks, vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
    }

    #[test]
    fn negative_range() {
        let ticks = nice_ticks(-10.0, 10.0, 4);
        assert_eq!(ticks, vec![-10.0, -5.0, 0.0, 5.0, 10.0]);
    }

    #[test]
    fn degenerate_range_returns_single_tick() {
        assert_eq!(nice_ticks(3.0, 3.0, 5), vec![3.0]);
    }

    #[test]
    fn small_fractional_range() {
        let ticks = nice_ticks(0.0, 0.09, 4);
        // step should be 0.02 (nearest 1-2-5 step for a target of ~4 ticks
        // over a span of 0.09), giving 0.0, 0.02, 0.04, 0.06, 0.08
        assert_eq!(ticks, vec![0.0, 0.02, 0.04, 0.06, 0.08]);
    }
}
