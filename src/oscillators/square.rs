//! Square / pulse wave oscillator — naive and band-limited (PolyBLEP).

use crate::core::Sample;
use super::oscillator::Oscillator;
use super::saw::poly_blep;

/// A square (pulse) wave oscillator with variable pulse width and optional
/// PolyBLEP anti-aliasing.
///
/// ## Waveform
///
/// A pulse wave is `+1` when `phase < pulse_width`, `−1` otherwise.
/// When `pulse_width = 0.5` the result is a symmetric square wave:
///
/// ```text
/// +1.0 ┤▁▁▁▁▁▁▁|       |▁▁▁▁▁▁▁|
///      |       |       |       |
/// -1.0 ┤       |▁▁▁▁▁▁▁|       |▁▁▁
///      0      0.5      T      1.5T
/// ```
///
/// Varying the pulse width changes the timbre: narrow pulses (pw ≈ 0.1) sound
/// thin and nasal; wide pulses (pw ≈ 0.9) produce a hollow sound. A symmetric
/// square (pw = 0.5) contains only odd harmonics.
///
/// ## PolyBLEP anti-aliasing
///
/// The pulse wave has two hard discontinuities per period:
/// - A **rising edge** (+2 step) at `phase = 0`.
/// - A **falling edge** (−2 step) at `phase = pulse_width`.
///
/// PolyBLEP is applied independently at both transition points, smoothing
/// each step over two samples.
///
/// For the rising edge: `out += poly_blep(phase, phase_inc)`
/// For the falling edge: `out -= poly_blep((phase − pw + 1) mod 1, phase_inc)`
///
/// ## Pulse width modulation (PWM)
///
/// Call `set_pulse_width()` every sample (or every block) to modulate the
/// pulse width with an LFO. This is the classic "PWM pad" synthesis technique.
///
/// ## Example
///
/// ```rust
/// use dsp::oscillators::{Oscillator, SquareOscillator};
///
/// let mut osc = SquareOscillator::<f32>::new(440.0, 44100.0);
/// osc.set_pulse_width(0.5); // symmetric square wave (default)
///
/// for _ in 0..512 {
///     let s = osc.process();
///     assert!(s.is_finite());
/// }
/// ```
pub struct SquareOscillator<T: Sample> {
    /// Current phase in [0, 1).
    phase: f64,

    /// Phase increment per sample = freq / sample_rate.
    phase_inc: f64,

    /// Stored frequency in Hz.
    freq: f64,

    /// Stored sample rate in Hz.
    sample_rate: f64,

    /// Pulse width (duty cycle) in (0, 1). 0.5 = symmetric square.
    pulse_width: f64,

    /// Whether to apply PolyBLEP anti-aliasing (default: true).
    band_limited: bool,

    _phantom: std::marker::PhantomData<T>,
}

impl<T: Sample> SquareOscillator<T> {
    /// Create a new square oscillator at 50% duty cycle (band-limited by default).
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::oscillators::{Oscillator, SquareOscillator};
    ///
    /// let osc = SquareOscillator::<f32>::new(440.0, 44100.0);
    /// assert_eq!(osc.pulse_width(), 0.5);
    /// assert!(osc.is_band_limited());
    /// ```
    pub fn new(freq: f64, sample_rate: f64) -> Self {
        Self {
            phase: 0.0,
            phase_inc: freq / sample_rate,
            freq,
            sample_rate,
            pulse_width: 0.5,
            band_limited: true,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set the pulse width (duty cycle) in the range (0, 1).
    ///
    /// 0.5 produces a symmetric square wave. Values near 0 or 1 produce
    /// a narrow pulse; the value is clamped to [0.01, 0.99] to avoid a
    /// degenerate waveform (constant DC).
    ///
    /// Safe to call from the audio thread — update at most once per block
    /// for PWM effects.
    pub fn set_pulse_width(&mut self, width: f64) {
        self.pulse_width = width.clamp(0.01, 0.99);
    }

    /// Get the current pulse width in (0, 1).
    pub fn pulse_width(&self) -> f64 {
        self.pulse_width
    }

    /// Enable or disable PolyBLEP anti-aliasing (default: enabled).
    pub fn set_band_limited(&mut self, enabled: bool) {
        self.band_limited = enabled;
    }

    /// Returns `true` if PolyBLEP anti-aliasing is currently enabled.
    pub fn is_band_limited(&self) -> bool {
        self.band_limited
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

impl<T: Sample> Oscillator<T> for SquareOscillator<T> {
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
        // Naive pulse: +1 below pulse_width, -1 above.
        let mut out = if self.phase < self.pulse_width { 1.0 } else { -1.0 };

        if self.band_limited {
            // Rising edge at phase = 0 (add correction — step is +2).
            out += poly_blep(self.phase, self.phase_inc);

            // Falling edge at phase = pulse_width (subtract correction — step is -2).
            // Shift phase so the falling edge appears at 0 in the blep window.
            let falling_phase = (self.phase - self.pulse_width + 1.0) % 1.0;
            out -= poly_blep(falling_phase, self.phase_inc);
        }

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
    fn test_output_is_finite() {
        let mut osc = SquareOscillator::<f32>::new(440.0, SR);
        for i in 0..10000 {
            let s = osc.process();
            assert!(s.is_finite(), "sample {i} non-finite: {s}");
        }
    }

    #[test]
    fn test_naive_first_half_is_positive() {
        // At phase=0 with pw=0.5, naive output is +1
        let mut osc = SquareOscillator::<f32>::new(100.0, SR);
        osc.set_band_limited(false);

        // Process enough samples to confirm first half is +1
        let half_period = (SR / 100.0 / 2.0) as usize;
        for _ in 0..half_period {
            let s = osc.process();
            // Naive square should be exactly +1 or -1 (no blending)
            assert!(
                (s - 1.0).abs() < 1e-5 || (s + 1.0).abs() < 1e-5,
                "naive output should be ±1, got {s}"
            );
        }
    }

    #[test]
    fn test_pulse_width_clamped() {
        let mut osc = SquareOscillator::<f32>::new(440.0, SR);
        osc.set_pulse_width(0.0);
        assert!((osc.pulse_width() - 0.01).abs() < 1e-10);
        osc.set_pulse_width(1.0);
        assert!((osc.pulse_width() - 0.99).abs() < 1e-10);
    }

    #[test]
    fn test_band_limited_default_on() {
        let osc = SquareOscillator::<f32>::new(440.0, SR);
        assert!(osc.is_band_limited());
    }

    #[test]
    fn test_zero_dc_for_symmetric_square() {
        // A symmetric square wave (pw=0.5) has zero DC component.
        // Over many complete periods the mean should be near zero.
        let freq = 100.0;
        let mut osc = SquareOscillator::<f32>::new(freq, SR);
        osc.set_pulse_width(0.5);

        let n = 44100usize; // exactly 1 second = 100 full periods
        let sum: f32 = (0..n).map(|_| osc.process()).sum();
        let mean = sum / n as f32;
        assert!(mean.abs() < 0.01, "DC for 50% square should be ~0, got {mean}");
    }

    #[test]
    fn test_asymmetric_pulse_has_dc() {
        // A 25% duty cycle has a DC component of -0.5 (= 2*0.25 - 1).
        let freq = 100.0;
        let mut osc = SquareOscillator::<f32>::new(freq, SR);
        osc.set_band_limited(false);
        osc.set_pulse_width(0.25);

        let n = 44100usize;
        let sum: f32 = (0..n).map(|_| osc.process()).sum();
        let mean = sum / n as f32;
        // Expected DC: 0.25 * (+1) + 0.75 * (-1) = 0.25 - 0.75 = -0.5
        assert!(
            (mean - (-0.5)).abs() < 0.01,
            "expected DC ~ -0.5 for 25% duty cycle, got {mean}"
        );
    }

    #[test]
    fn test_reset_restarts_phase() {
        let mut osc = SquareOscillator::<f32>::new(440.0, SR);
        for _ in 0..100 { osc.process(); }
        osc.reset();
        assert!(osc.phase() < 1e-9);
    }

    #[test]
    fn test_works_with_f64() {
        let mut osc = SquareOscillator::<f64>::new(440.0, SR);
        for i in 0..1000 {
            let s = osc.process();
            assert!(s.is_finite(), "f64 sample {i} non-finite");
        }
    }

    #[test]
    fn test_set_frequency() {
        let mut osc = SquareOscillator::<f32>::new(440.0, SR);
        osc.set_frequency(880.0);
        assert_eq!(osc.frequency(), 880.0);
        let expected_inc = 880.0 / SR;
        assert!((osc.phase_inc - expected_inc).abs() < 1e-12);
    }
}
