//! Triangle wave oscillator.

use crate::core::Sample;
use super::oscillator::Oscillator;

/// A triangle wave oscillator.
///
/// The triangle wave rises linearly from −1 to +1 over the first half of
/// each period, then falls linearly from +1 back to −1 over the second half:
///
/// ```text
/// +1.0 ┤      /\            /\
///      |     /  \          /  \
///  0.0 ┤----/----\--------/----\---
///      |   /      \      /      \
/// -1.0 ┤__/        \    /        \__
///      0           T/2           T
/// ```
///
/// ## Aliasing
///
/// The triangle waveform itself is continuous (no jumps), so its aliasing is
/// much weaker than the saw or square wave. The harmonics roll off as `1/n²`
/// (versus `1/n` for saw/square), which means the aliasing products fall
/// 40 dB per decade instead of 20 dB. In practice, a naive triangle sounds
/// clean up to several kHz without any band-limiting.
///
/// If you need a band-limited triangle at very high frequencies, it can be
/// derived by integrating a PolyBLEP square wave. That is left for a future
/// release; the naive implementation here is sufficient for Phase 1.
///
/// ## Example
///
/// ```rust
/// use dsp::oscillators::{Oscillator, TriangleOscillator};
///
/// let mut osc = TriangleOscillator::<f32>::new(440.0, 44100.0);
///
/// for _ in 0..512 {
///     let s = osc.process();
///     // Output is always in [-1, 1]
///     assert!(s >= -1.0 && s <= 1.0);
/// }
/// ```
pub struct TriangleOscillator<T: Sample> {
    /// Current phase in [0, 1).
    phase: f64,

    /// Phase increment per sample = freq / sample_rate.
    phase_inc: f64,

    /// Stored frequency in Hz.
    freq: f64,

    /// Stored sample rate in Hz.
    sample_rate: f64,

    _phantom: std::marker::PhantomData<T>,
}

impl<T: Sample> TriangleOscillator<T> {
    /// Create a new triangle oscillator starting at phase 0.
    ///
    /// At `phase = 0` the output is −1 (the trough of the triangle).
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::oscillators::{Oscillator, TriangleOscillator};
    ///
    /// let osc = TriangleOscillator::<f32>::new(440.0, 44100.0);
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

    /// Current oscillator frequency in Hz.
    pub fn frequency(&self) -> f64 {
        self.freq
    }

    /// Current normalized phase in [0, 1).
    pub fn phase(&self) -> f64 {
        self.phase
    }
}

impl<T: Sample> Oscillator<T> for TriangleOscillator<T> {
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
        self.phase = phase.to_f64().rem_euclid(1.0);
    }

    #[inline]
    fn process(&mut self) -> T {
        // Triangle: rise from -1 to +1 over [0, 0.5), fall from +1 to -1 over [0.5, 1).
        //   Rising half:  out = 4*phase - 1   → -1 at 0,  +1 at 0.5
        //   Falling half: out = -4*phase + 3  → +1 at 0.5, -1 at 1
        let out = if self.phase < 0.5 {
            4.0 * self.phase - 1.0
        } else {
            -4.0 * self.phase + 3.0
        };

        self.phase += self.phase_inc;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        T::from_f64(out)
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
    fn test_output_in_bounds() {
        let mut osc = TriangleOscillator::<f32>::new(440.0, SR);
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
        let mut osc = TriangleOscillator::<f32>::new(440.0, SR);
        for i in 0..10000 {
            let s = osc.process();
            assert!(s.is_finite(), "sample {i} non-finite: {s}");
        }
    }

    #[test]
    fn test_initial_output_is_negative_one() {
        // At phase=0: out = 4*0 - 1 = -1
        let mut osc = TriangleOscillator::<f32>::new(440.0, SR);
        let first = osc.process();
        assert!(
            (first - (-1.0)).abs() < 1e-5,
            "expected -1.0 at phase=0, got {first}"
        );
    }

    #[test]
    fn test_quarter_period_is_zero() {
        // At phase=0.25: rising half → out = 4*0.25 - 1 = 0
        let freq = 100.0;
        let mut osc = TriangleOscillator::<f32>::new(freq, SR);
        let quarter = (SR / freq / 4.0).round() as usize;
        for _ in 0..quarter { osc.process(); }
        let val = osc.process();
        assert!(val.abs() < 0.02, "expected ~0 at quarter period, got {val}");
    }

    #[test]
    fn test_half_period_is_positive_one() {
        // At phase=0.5: out = -4*0.5 + 3 = 1
        let freq = 100.0;
        let mut osc = TriangleOscillator::<f32>::new(freq, SR);
        let half = (SR / freq / 2.0).round() as usize;
        for _ in 0..half { osc.process(); }
        let val = osc.process();
        assert!(val > 0.98, "expected near +1 at half period, got {val}");
    }

    #[test]
    fn test_zero_dc() {
        // Triangle has zero DC by symmetry.
        let freq = 100.0;
        let mut osc = TriangleOscillator::<f32>::new(freq, SR);
        let n = 44100usize;
        let sum: f32 = (0..n).map(|_| osc.process()).sum();
        let mean = sum / n as f32;
        assert!(mean.abs() < 0.01, "DC should be ~0, got {mean}");
    }

    #[test]
    fn test_reset_restarts_phase() {
        let mut osc = TriangleOscillator::<f32>::new(440.0, SR);
        for _ in 0..100 { osc.process(); }
        osc.reset();
        assert_eq!(osc.phase(), 0.0);
        let first = osc.process();
        assert!((first - (-1.0)).abs() < 1e-5);
    }

    #[test]
    fn test_monotone_rising_in_first_half() {
        // Within the first half period the triangle must be monotonically rising.
        let freq = 100.0;
        let mut osc = TriangleOscillator::<f32>::new(freq, SR);
        let half = (SR / freq / 2.0) as usize - 1;
        let mut prev = osc.process();
        for _ in 0..half {
            let cur = osc.process();
            assert!(cur >= prev, "not monotone: {cur} < {prev}");
            prev = cur;
        }
    }

    #[test]
    fn test_works_with_f64() {
        let mut osc = TriangleOscillator::<f64>::new(440.0, SR);
        for i in 0..1000 {
            let s = osc.process();
            assert!(s.is_finite(), "f64 sample {i} non-finite");
            assert!(s >= -1.0 && s <= 1.0);
        }
    }
}
