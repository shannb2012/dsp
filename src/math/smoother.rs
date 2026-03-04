//! Parameter smoother for click-free automation.

use crate::core::Sample;

/// A one-pole exponential parameter smoother.
///
/// When a user moves a knob (say, filter cutoff), the parameter value jumps
/// instantly. Applying that jump directly to the audio computation creates a
/// *discontinuity* — a sharp step in the signal that sounds like a click or a
/// zipper noise.
///
/// `ParamSmoother` solves this by exponentially moving toward the target value
/// over a configurable time window, making the transition inaudible.
///
/// ## Algorithm
///
/// This is a first-order IIR lowpass (exponential moving average):
///
/// ```text
/// y[n] = y[n-1] + (target - y[n-1]) × coefficient
/// ```
///
/// The `coefficient` controls how fast the smoother tracks the target.
/// It is derived from the desired time constant τ (in seconds):
///
/// ```text
/// coefficient = 1 − e^(−1 / (τ × sample_rate))
/// ```
///
/// With a 20 ms time constant at 44,100 Hz:
/// - After 20 ms (882 samples): output is at 63% of the target
/// - After 60 ms (2,646 samples): output is at 95% of the target
/// - After 100 ms (4,410 samples): output is at 99.3% of the target
///
/// ## Usage pattern
///
/// ```rust
/// use dsp::math::ParamSmoother;
///
/// let sample_rate = 44100.0;
/// let mut smoother = ParamSmoother::<f32>::new(1.0, 0.02, sample_rate); // 20 ms
///
/// // User moves a knob — set the new target
/// smoother.set_target(0.5);
///
/// // In process(), call once per sample to get the smoothed value
/// for _ in 0..512 {
///     let gain = smoother.process();
///     // use `gain` to scale audio samples...
///     let _ = gain;
/// }
/// ```
///
/// ## Real-time safety
///
/// `process()` and `process_block()` make no allocations and run in O(1) and
/// O(n) time respectively. Only `set_smoothing_time()` performs a transcendental
/// function call (`exp`); call it from `prepare()`, not `process()`.
pub struct ParamSmoother<T: Sample> {
    /// The current smoothed value — updated each call to `process()`.
    current: T,

    /// The value we are moving toward.
    target: T,

    /// How much of the gap `(target − current)` to close each sample.
    ///
    /// Derived from the smoothing time constant: `1 − e^(−1 / (τ × fs))`.
    /// A larger coefficient means faster tracking (shorter time constant).
    /// Range: (0.0, 1.0]. At 1.0 the smoother jumps instantly; at ~0 it never moves.
    coefficient: T,
}

impl<T: Sample> ParamSmoother<T> {
    /// Create a new smoother starting at `initial_value`.
    ///
    /// Both `current` and `target` are set to `initial_value`, so there is no
    /// initial ramp-up.
    ///
    /// # Arguments
    ///
    /// * `initial_value` — starting and target value.
    /// * `smoothing_secs` — desired time constant (τ). Typical range: 0.005–0.05 s.
    ///   Pass `0.0` for instant response (no smoothing).
    /// * `sample_rate` — the audio sample rate in Hz.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::math::ParamSmoother;
    ///
    /// let smoother = ParamSmoother::<f32>::new(1.0, 0.02, 44100.0);
    /// assert_eq!(smoother.current(), 1.0);
    /// assert_eq!(smoother.target(), 1.0);
    /// ```
    pub fn new(initial_value: T, smoothing_secs: f64, sample_rate: f64) -> Self {
        let coefficient = Self::compute_coefficient(smoothing_secs, sample_rate);
        Self {
            current: initial_value,
            target: initial_value,
            coefficient,
        }
    }

    /// Update the smoothing time constant without changing current or target.
    ///
    /// This involves an `exp()` call — call it from `prepare()`, not `process()`.
    pub fn set_smoothing_time(&mut self, smoothing_secs: f64, sample_rate: f64) {
        self.coefficient = Self::compute_coefficient(smoothing_secs, sample_rate);
    }

    /// Set a new target value. The smoother will gradually converge to it.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::math::ParamSmoother;
    ///
    /// let mut smoother = ParamSmoother::<f32>::new(0.0, 0.02, 44100.0);
    /// smoother.set_target(1.0);
    /// assert_eq!(smoother.target(), 1.0);
    /// ```
    #[inline]
    pub fn set_target(&mut self, target: T) {
        self.target = target;
    }

    /// Jump immediately to `value`, bypassing the smoothing ramp.
    ///
    /// Use this when starting a new note or resetting a plugin — you want
    /// silence to silence instantly, not to ramp from a previous value.
    #[inline]
    pub fn reset(&mut self, value: T) {
        self.current = value;
        self.target = value;
    }

    /// Process one sample and return the current smoothed value.
    ///
    /// Each call moves `current` a fraction of the way toward `target`.
    ///
    /// This is **real-time safe**: one multiply-add, no allocations.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::math::ParamSmoother;
    ///
    /// let mut smoother = ParamSmoother::<f32>::new(0.0, 0.02, 44100.0);
    /// smoother.set_target(1.0);
    ///
    /// // After many samples, current converges to target
    /// for _ in 0..10000 {
    ///     smoother.process();
    /// }
    /// assert!(smoother.current() > 0.99);
    /// ```
    #[inline]
    pub fn process(&mut self) -> T {
        // Exponential approach: move `coefficient` fraction of remaining gap.
        // Equivalent to a one-pole lowpass filter with the target as input.
        self.current = self.current + (self.target - self.current) * self.coefficient;
        self.current
    }

    /// Fill a slice with smoothed values, one per sample.
    ///
    /// More efficient than calling `process()` in a loop because the simple
    /// pattern allows auto-vectorization.
    ///
    /// Typical use: generate a per-sample gain envelope for an entire block
    /// before entering the sample-by-sample processing loop.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::math::ParamSmoother;
    ///
    /// let mut smoother = ParamSmoother::<f32>::new(0.0, 0.005, 44100.0);
    /// smoother.set_target(1.0);
    ///
    /// let mut gains = [0.0f32; 512];
    /// smoother.process_block(&mut gains);
    /// // gains[0] is near 0.0, gains[511] is closer to 1.0
    /// assert!(gains[0] < gains[511]);
    /// ```
    pub fn process_block(&mut self, output: &mut [T]) {
        for slot in output.iter_mut() {
            self.current = self.current + (self.target - self.current) * self.coefficient;
            *slot = self.current;
        }
    }

    /// Get the current smoothed value without advancing the smoother.
    #[inline]
    pub fn current(&self) -> T {
        self.current
    }

    /// Get the target value.
    #[inline]
    pub fn target(&self) -> T {
        self.target
    }

    /// Returns `true` if the smoother has settled within `epsilon` of its target.
    ///
    /// Useful for deciding whether to skip processing silent/settled voices.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::math::ParamSmoother;
    ///
    /// let mut smoother = ParamSmoother::<f32>::new(1.0, 0.02, 44100.0);
    /// smoother.set_target(0.0);
    ///
    /// assert!(!smoother.is_settled(0.001));  // Just set target, not settled yet
    /// ```
    #[inline]
    pub fn is_settled(&self, epsilon: T) -> bool {
        (self.target - self.current).abs() < epsilon
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Derive the per-sample step coefficient from a time constant.
    ///
    /// Formula: `coefficient = 1 − exp(−1 / (τ × fs))`
    ///
    /// Special case: `smoothing_secs ≤ 0.0` → coefficient = 1.0 (instant response).
    fn compute_coefficient(smoothing_secs: f64, sample_rate: f64) -> T {
        if smoothing_secs <= 0.0 || sample_rate <= 0.0 {
            // Instant response: jump to target immediately on first process() call
            T::ONE
        } else {
            let tau_samples = smoothing_secs * sample_rate;
            T::from_f64(1.0 - (-1.0_f64 / tau_samples).exp())
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f64 = 44100.0;

    #[test]
    fn test_initial_state() {
        let smoother = ParamSmoother::<f32>::new(0.5, 0.02, SR);
        assert_eq!(smoother.current(), 0.5);
        assert_eq!(smoother.target(), 0.5);
        assert!(smoother.is_settled(1e-6));
    }

    #[test]
    fn test_converges_to_target() {
        let mut smoother = ParamSmoother::<f32>::new(0.0, 0.02, SR);
        smoother.set_target(1.0);

        // Run for 5 time constants (≈100ms at 44100 Hz) — should be >99% there
        let tau_samples = (0.02 * SR) as usize;
        for _ in 0..tau_samples * 5 {
            smoother.process();
        }

        assert!(smoother.current() > 0.99, "Expected >99%, got {}", smoother.current());
    }

    #[test]
    fn test_63_percent_at_one_tau() {
        // One time constant should bring us to ~63.2% of target
        let mut smoother = ParamSmoother::<f32>::new(0.0, 0.02, SR);
        smoother.set_target(1.0);

        let tau_samples = (0.02 * SR) as usize;
        for _ in 0..tau_samples {
            smoother.process();
        }

        let val = smoother.current();
        assert!(val > 0.60 && val < 0.67,
            "Expected ~63% at one tau, got {:.4}", val);
    }

    #[test]
    fn test_zero_smoothing_is_instant() {
        let mut smoother = ParamSmoother::<f32>::new(0.0, 0.0, SR);
        smoother.set_target(1.0);
        let out = smoother.process();
        assert_eq!(out, 1.0, "Zero smoothing time should be instant");
    }

    #[test]
    fn test_reset_jumps_immediately() {
        let mut smoother = ParamSmoother::<f32>::new(0.0, 0.02, SR);
        smoother.set_target(1.0);
        smoother.process(); // Advance partway
        assert!(smoother.current() < 0.01);

        smoother.reset(0.5);
        assert_eq!(smoother.current(), 0.5);
        assert_eq!(smoother.target(), 0.5);
    }

    #[test]
    fn test_process_block_matches_process() {
        let mut smoother_a = ParamSmoother::<f32>::new(0.0, 0.02, SR);
        let mut smoother_b = ParamSmoother::<f32>::new(0.0, 0.02, SR);
        smoother_a.set_target(1.0);
        smoother_b.set_target(1.0);

        let mut block = [0.0f32; 64];
        smoother_a.process_block(&mut block);

        for val in block.iter() {
            let expected = smoother_b.process();
            assert!((val - expected).abs() < 1e-6,
                "process_block diverged from process(): {} vs {}", val, expected);
        }
    }

    #[test]
    fn test_process_block_is_monotonic_toward_target() {
        let mut smoother = ParamSmoother::<f32>::new(0.0, 0.02, SR);
        smoother.set_target(1.0);

        let mut block = [0.0f32; 256];
        smoother.process_block(&mut block);

        // Each value should be >= the previous (monotonically increasing toward 1.0)
        for window in block.windows(2) {
            assert!(window[1] >= window[0],
                "Expected monotonic increase: {} then {}", window[0], window[1]);
        }
    }

    #[test]
    fn test_is_settled() {
        let mut smoother = ParamSmoother::<f32>::new(1.0, 0.02, SR);
        smoother.set_target(0.0);

        // Not settled immediately
        assert!(!smoother.is_settled(1e-4));

        // Run for a very long time
        for _ in 0..1_000_000 {
            smoother.process();
        }

        assert!(smoother.is_settled(1e-4));
    }

    #[test]
    fn test_output_is_always_finite() {
        let mut smoother = ParamSmoother::<f32>::new(0.0, 0.02, SR);
        smoother.set_target(1.0);

        for i in 0..10000 {
            let out = smoother.process();
            assert!(out.is_finite(), "Sample {} produced non-finite value: {}", i, out);
        }
    }

    #[test]
    fn test_works_with_f64() {
        let mut smoother = ParamSmoother::<f64>::new(0.0, 0.01, 48000.0);
        smoother.set_target(1.0);
        for _ in 0..10000 {
            smoother.process();
        }
        assert!(smoother.current() > 0.99);
    }

    #[test]
    fn test_set_smoothing_time() {
        // Slow smoother (1s τ): process one sample toward target 1.0
        let mut smoother = ParamSmoother::<f32>::new(0.0, 1.0, SR);
        smoother.set_target(1.0);
        smoother.process();
        let slow = smoother.current();

        // Fast smoother (1ms τ): reset then process one sample toward same target.
        // `reset` sets both current AND target to 0, so we must call set_target again.
        smoother.reset(0.0);
        smoother.set_smoothing_time(0.001, SR);
        smoother.set_target(1.0); // re-establish target after reset
        smoother.process();
        let fast = smoother.current();

        assert!(fast > slow, "Shorter time constant should track faster: slow={}, fast={}", slow, fast);
    }
}
