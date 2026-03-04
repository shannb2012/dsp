//! Sine wave oscillator using phase accumulation.

use crate::core::Sample;
use super::oscillator::Oscillator;

/// A pure sine wave oscillator.
///
/// Phase accumulation is the standard approach for digital sine oscillators:
/// maintain an internal phase in [0, 1), advance it by `freq / sample_rate`
/// each sample, and evaluate `sin(2π × phase)`.
///
/// ## Precision
///
/// Phase is stored as `f64` regardless of the output type `T`. This prevents
/// the gradual drift that accumulates when using `f32` for very long runs or
/// at high frequencies — a 20 kHz oscillator at 44.1 kHz SR has a phase
/// increment of ~0.454, and `f32` only has ~7 decimal digits of precision.
///
/// ## Real-time safety
///
/// `process()` makes one `sin()` call and one add. The `sin()` call from the
/// standard library typically takes 10–20 ns. If you need something cheaper
/// in exchange for ~0.17% error, you can use [`crate::math::fast_sin`]
/// directly with `phase * TAU`.
///
/// ## Example
///
/// ```rust
/// use dsp::oscillators::{Oscillator, SineOscillator};
///
/// let mut osc = SineOscillator::<f32>::new(440.0, 44100.0);
///
/// // A4 should output samples in [-1, 1]
/// for _ in 0..100 {
///     let s = osc.process();
///     assert!(s >= -1.0 && s <= 1.0);
/// }
/// ```
pub struct SineOscillator<T: Sample> {
    /// Current phase in [0, 1).
    phase: f64,

    /// Phase increment per sample = freq / sample_rate.
    ///
    /// Stored as f64 to match the precision of `phase`.
    phase_inc: f64,

    /// Stored frequency in Hz, needed to recompute `phase_inc` on SR change.
    freq: f64,

    /// Stored sample rate in Hz.
    sample_rate: f64,

    /// Zero-size marker for the output sample type.
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Sample> SineOscillator<T> {
    /// Create a new sine oscillator starting at phase 0.
    ///
    /// # Arguments
    ///
    /// * `freq` — frequency in Hz. A4 = 440.0.
    /// * `sample_rate` — audio sample rate in Hz (e.g. 44100.0, 48000.0).
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::oscillators::{Oscillator, SineOscillator};
    ///
    /// let osc = SineOscillator::<f32>::new(440.0, 44100.0);
    /// assert_eq!(osc.frequency(), 440.0);
    /// ```
    pub fn new(freq: f64, sample_rate: f64) -> Self {
        Self {
            phase: 0.0,
            phase_inc: freq / sample_rate,
            freq,
            sample_rate,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get the current phase in [0, 1).
    #[inline]
    pub fn phase(&self) -> f64 {
        self.phase
    }

    /// Get the oscillator frequency in Hz.
    #[inline]
    pub fn frequency(&self) -> f64 {
        self.freq
    }
}

impl<T: Sample> Oscillator<T> for SineOscillator<T> {
    #[inline]
    fn set_frequency(&mut self, freq: f64) {
        self.freq = freq;
        self.phase_inc = freq / self.sample_rate;
    }

    #[inline]
    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
        self.phase_inc = self.freq / sample_rate;
    }

    #[inline]
    fn set_phase(&mut self, phase: T) {
        // rem_euclid wraps negative values correctly into [0, 1)
        self.phase = phase.to_f64().rem_euclid(1.0);
    }

    #[inline]
    fn process(&mut self) -> T {
        // Evaluate sin at current phase, then advance.
        let out = T::from_f64((self.phase * std::f64::consts::TAU).sin());

        self.phase += self.phase_inc;
        // Conditional subtraction is faster than rem_euclid for values near 1.
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        out
    }

    #[inline]
    fn reset(&mut self) {
        self.phase = 0.0;
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
    fn test_initial_output_is_zero() {
        // At phase=0, sin(0) = 0
        let mut osc = SineOscillator::<f32>::new(440.0, SR);
        let first = osc.process();
        assert!(first.abs() < 1e-6, "sin(0) should be ~0, got {first}");
    }

    #[test]
    fn test_output_in_bounds() {
        let mut osc = SineOscillator::<f32>::new(440.0, SR);
        for i in 0..10000 {
            let s = osc.process();
            assert!(
                s >= -1.0 && s <= 1.0,
                "sample {i} out of bounds: {s}"
            );
        }
    }

    #[test]
    fn test_output_is_finite() {
        let mut osc = SineOscillator::<f32>::new(440.0, SR);
        for i in 0..10000 {
            let s = osc.process();
            assert!(s.is_finite(), "sample {i} is non-finite: {s}");
        }
    }

    #[test]
    fn test_quarter_period_is_positive_peak() {
        // After exactly 1/4 period, sin(π/2) = 1.0
        let freq = 1000.0;
        let mut osc = SineOscillator::<f32>::new(freq, SR);
        let quarter_period = (SR / freq / 4.0).round() as usize;
        // Skip the 0th sample (phase=0), run to quarter period
        for _ in 0..quarter_period {
            osc.process();
        }
        let val = osc.process();
        // Should be close to 1.0 (exact timing depends on rounding of quarter_period)
        assert!(
            val > 0.95,
            "expected near +1.0 at quarter period, got {val}"
        );
    }

    #[test]
    fn test_half_period_returns_to_zero() {
        // After exactly 1/2 period the sine crosses zero again
        let freq = 100.0;
        let mut osc = SineOscillator::<f32>::new(freq, SR);
        let half_period = (SR / freq / 2.0).round() as usize;
        for _ in 0..half_period {
            osc.process();
        }
        let val = osc.process();
        assert!(
            val.abs() < 0.02,
            "expected near 0 at half period, got {val}"
        );
    }

    #[test]
    fn test_reset_snaps_to_zero() {
        let mut osc = SineOscillator::<f32>::new(440.0, SR);
        // Advance by 100 samples so phase is non-zero
        for _ in 0..100 {
            osc.process();
        }
        assert!(osc.phase() > 0.0);

        osc.reset();
        assert_eq!(osc.phase(), 0.0);
        let first = osc.process();
        assert!(first.abs() < 1e-6, "after reset, sin(0) should be ~0");
    }

    #[test]
    fn test_set_frequency_changes_rate() {
        // After half a period of the faster oscillator, its phase should be
        // near 0.5 while the slower one is still near 0 — a clear difference.
        // 1000 Hz half-period ≈ 22 samples; 100 Hz at 22 samples → phase ≈ 0.05.
        let half_period_1khz = (SR / 1000.0 / 2.0).round() as usize;

        let mut fast = SineOscillator::<f32>::new(1000.0, SR);
        let mut slow = SineOscillator::<f32>::new(100.0, SR);

        for _ in 0..half_period_1khz {
            fast.process();
            slow.process();
        }

        // fast phase ≈ 0.5, slow phase ≈ 0.05 → difference > 0.3
        let diff = (fast.phase() - slow.phase()).abs();
        assert!(
            diff > 0.3,
            "fast phase={:.3}, slow phase={:.3}, diff={diff:.3} (expected > 0.3)",
            fast.phase(), slow.phase()
        );
    }

    #[test]
    fn test_set_sample_rate_adjusts_increment() {
        let mut osc = SineOscillator::<f32>::new(440.0, SR);
        let inc_44 = osc.phase_inc;
        osc.set_sample_rate(48000.0);
        // At 48k the same frequency has a smaller phase increment
        assert!(
            osc.phase_inc < inc_44,
            "expected smaller phase_inc at higher SR"
        );
    }

    #[test]
    fn test_set_phase_wraps_correctly() {
        let mut osc = SineOscillator::<f32>::new(440.0, SR);
        osc.set_phase(1.5f32); // Should wrap to 0.5
        assert!((osc.phase() - 0.5).abs() < 1e-6);

        osc.set_phase(-0.25f32); // Should wrap to 0.75
        assert!((osc.phase() - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_works_with_f64() {
        let mut osc = SineOscillator::<f64>::new(440.0, SR);
        for i in 0..1000 {
            let s = osc.process();
            assert!(s.is_finite(), "f64 sample {i} non-finite");
            assert!(s >= -1.0 && s <= 1.0, "f64 sample {i} out of bounds: {s}");
        }
    }

    #[test]
    fn test_zero_frequency_produces_silence() {
        // At freq=0, phase_inc=0, phase never changes; sin(0)=0 forever
        let mut osc = SineOscillator::<f32>::new(0.0, SR);
        for i in 0..100 {
            let s = osc.process();
            assert!(s.abs() < 1e-6, "DC at 0 Hz should be 0, got {s} at sample {i}");
        }
    }
}
