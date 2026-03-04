//! Sawtooth wave oscillator — naive and band-limited (PolyBLEP).

use crate::core::Sample;
use super::oscillator::Oscillator;

/// A sawtooth wave oscillator with optional PolyBLEP anti-aliasing.
///
/// ## Waveform
///
/// The sawtooth rises linearly from −1 to +1 over one period, then jumps
/// discontinuously back to −1:
///
/// ```text
/// 1.0 ┤         /|        /|
///     |        / |       / |
///     |       /  |      /  |
/// 0.0 ┤------/---|-----/-  |
///     |     /    |    /    |
///     |    /     |   /     |
///-1.0 ┤___/      |__/      |__
///     0          T         2T
/// ```
///
/// ## Aliasing and the PolyBLEP correction
///
/// The naive saw (formula `2×phase − 1`) contains a hard discontinuity at
/// the end of each period. This creates harmonics that alias back into the
/// audible range, producing a harsh "digital" sound at frequencies above
/// ~1 kHz.
///
/// **PolyBLEP** (Polynomial Band-Limited stEP) corrects for the
/// discontinuity by blending a small polynomial correction near the
/// transition. The result is audibly indistinguishable from a truly
/// band-limited sawtooth at the cost of just a few extra arithmetic
/// operations per sample.
///
/// Enable band-limiting with [`SawOscillator::set_band_limited`] (default:
/// **on**). For LFOs or sub-audio rates where aliasing is inaudible, you
/// can disable it for a tiny CPU savings.
///
/// ## Algorithm
///
/// ```text
/// naive(phase) = 2 × phase − 1             range [−1, +1]
/// output       = naive − poly_blep(phase, phase_inc)
/// ```
///
/// The `poly_blep` function returns a correction that smooths the step at
/// `phase = 0` over two samples (one before and one after the wrap).
///
/// Reference: Välimäki & Pakarinen, "Discrete-Time Modelling of Musical
/// Instruments", *Proceedings of the IEEE*, 2006.
///
/// ## Example
///
/// ```rust
/// use dsp::oscillators::{Oscillator, SawOscillator};
///
/// let mut osc = SawOscillator::<f32>::new(220.0, 44100.0);
/// // Band-limited is on by default
///
/// for _ in 0..512 {
///     let s = osc.process();
///     assert!(s.is_finite());
/// }
/// ```
pub struct SawOscillator<T: Sample> {
    /// Current phase in [0, 1).
    phase: f64,

    /// Phase increment per sample = freq / sample_rate.
    phase_inc: f64,

    /// Stored frequency in Hz.
    freq: f64,

    /// Stored sample rate in Hz.
    sample_rate: f64,

    /// Whether to apply PolyBLEP anti-aliasing (default: true).
    band_limited: bool,

    _phantom: std::marker::PhantomData<T>,
}

impl<T: Sample> SawOscillator<T> {
    /// Create a new sawtooth oscillator (band-limited by default).
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::oscillators::{Oscillator, SawOscillator};
    ///
    /// let osc = SawOscillator::<f32>::new(440.0, 44100.0);
    /// assert!(osc.is_band_limited());
    /// ```
    pub fn new(freq: f64, sample_rate: f64) -> Self {
        Self {
            phase: 0.0,
            phase_inc: freq / sample_rate,
            freq,
            sample_rate,
            band_limited: true,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Enable or disable PolyBLEP anti-aliasing.
    ///
    /// - `true` (default) — band-limited, suitable for audio-rate oscillators.
    /// - `false` — naive sawtooth; suitable for LFOs or sub-audio modulation
    ///   where aliasing is below the audible range.
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

impl<T: Sample> Oscillator<T> for SawOscillator<T> {
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
        // Naive sawtooth: linear ramp from -1 to +1 over [0, 1).
        let mut out = 2.0 * self.phase - 1.0;

        if self.band_limited {
            // Subtract PolyBLEP correction at the wrap-point discontinuity.
            out -= poly_blep(self.phase, self.phase_inc);
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
// PolyBLEP helper
// ---------------------------------------------------------------------------

/// PolyBLEP correction for a unit step discontinuity at `phase = 0`.
///
/// Returns a correction value that should be **subtracted** from a naive
/// sawtooth near its wrap point, smoothing the step over two samples.
///
/// The function is non-zero only within one `phase_inc` of the wrap point:
/// - `phase < phase_inc` — approaching the wrap (blending out)
/// - `phase > 1 − phase_inc` — just after the wrap (blending in)
///
/// The polynomial is 2nd-order, giving C¹ continuity (continuous first
/// derivative) at the correction boundaries.
#[inline]
pub(super) fn poly_blep(phase: f64, phase_inc: f64) -> f64 {
    if phase < phase_inc {
        // Normalise to [0, 1) within the one-sample window before the wrap.
        let t = phase / phase_inc;
        // Polynomial: 2t - t² - 1 = -(1 - t)²
        2.0 * t - t * t - 1.0
    } else if phase > 1.0 - phase_inc {
        // Normalise to (-1, 0] within the one-sample window after the wrap.
        let t = (phase - 1.0) / phase_inc;
        // Polynomial: t² + 2t + 1 = (1 + t)²
        t * t + 2.0 * t + 1.0
    } else {
        0.0
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
        let mut osc = SawOscillator::<f32>::new(440.0, SR);
        for i in 0..10000 {
            let s = osc.process();
            assert!(s.is_finite(), "sample {i} is non-finite: {s}");
        }
    }

    #[test]
    fn test_naive_output_in_range() {
        // Naive saw: range is exactly [-1, 1] in steady state.
        // With PolyBLEP there is a slight overshoot, so test a looser bound.
        let mut osc = SawOscillator::<f32>::new(440.0, SR);
        osc.set_band_limited(false);

        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for _ in 0..10000 {
            let s = osc.process();
            min = min.min(s);
            max = max.max(s);
        }
        assert!(min >= -1.001, "min out of range: {min}");
        assert!(max <= 1.001, "max out of range: {max}");
    }

    #[test]
    fn test_naive_initial_value_is_negative_one() {
        // At phase=0, naive = 2*0 - 1 = -1
        let mut osc = SawOscillator::<f32>::new(440.0, SR);
        osc.set_band_limited(false);
        let first = osc.process();
        assert!(
            (first - (-1.0)).abs() < 1e-5,
            "expected -1.0 at phase=0, got {first}"
        );
    }

    #[test]
    fn test_blep_and_naive_similar_at_low_freq() {
        // At very low frequencies the discontinuity occupies a tiny fraction
        // of the period, so the RMS of naive and BL-saw should be similar.
        let mut naive = SawOscillator::<f32>::new(10.0, SR);
        let mut bl = SawOscillator::<f32>::new(10.0, SR);
        naive.set_band_limited(false);
        bl.set_band_limited(true);

        let n = 4410; // 0.1 s
        let rms = |osc: &mut SawOscillator<f32>| {
            let sum_sq: f32 = (0..n).map(|_| {
                let s = osc.process();
                s * s
            }).sum();
            (sum_sq / n as f32).sqrt()
        };

        let rms_naive = rms(&mut naive);
        let rms_bl    = rms(&mut bl);

        // Both should be close to the RMS of a unit sawtooth = 1/√3 ≈ 0.577
        assert!((rms_naive - rms_bl).abs() < 0.01,
            "RMS naive={rms_naive:.4}, bl={rms_bl:.4}");
    }

    #[test]
    fn test_reset_restarts_phase() {
        let mut osc = SawOscillator::<f32>::new(440.0, SR);
        osc.set_band_limited(false);
        for _ in 0..100 { osc.process(); }

        osc.reset();
        assert!(osc.phase() < 1e-9);
        let first = osc.process();
        assert!((first - (-1.0)).abs() < 1e-5);
    }

    #[test]
    fn test_set_frequency_changes_increment() {
        let mut osc = SawOscillator::<f32>::new(440.0, SR);
        let inc_440 = osc.phase_inc;
        osc.set_frequency(880.0);
        assert!((osc.phase_inc - 2.0 * inc_440).abs() < 1e-12);
    }

    #[test]
    fn test_poly_blep_zero_in_middle() {
        // Well away from the transition the correction must be exactly 0
        let phase_inc = 440.0 / SR;
        assert_eq!(poly_blep(0.5, phase_inc), 0.0);
    }

    #[test]
    fn test_works_with_f64() {
        let mut osc = SawOscillator::<f64>::new(440.0, SR);
        for i in 0..1000 {
            let s = osc.process();
            assert!(s.is_finite(), "f64 sample {i} non-finite");
        }
    }
}
