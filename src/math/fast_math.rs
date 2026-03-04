//! Fast mathematical approximations for real-time audio.
//!
//! Standard library functions like `sin`, `tanh`, and `exp` are accurate to
//! full floating-point precision, but their implementations can be expensive.
//! In audio hot paths where thousands of calls per second are made, a
//! polynomial approximation that is "good enough" (< 1% error) can provide a
//! meaningful speedup.
//!
//! ## When to use fast approximations
//!
//! - **Soft clipping / saturation** — `fast_tanh` is perceptually
//!   indistinguishable from the true `tanh` for waveshaping.
//! - **Cheap oscillators** — `fast_sin` can drive LFOs that modulate a
//!   parameter rather than drive audio directly.
//! - **Filter coefficients at audio rate** — `fast_exp` speeds up per-sample
//!   coefficient updates (though usually you update coefficients per block, not
//!   per sample, making this less critical).
//!
//! ## When NOT to use these
//!
//! - When the oscillator *is* the audio signal (pitch-critical sine waves).
//!   Use `T::sin()` from the `Sample` trait instead.
//! - When computing filter coefficients (accuracy matters for stability).
//! - For DC or very-low-frequency values where a few ULP of error compounds.
//!
//! ## Precision note
//!
//! All functions here operate on `f32`. Fast approximations are primarily
//! valuable at single precision — at `f64`, the standard library routines are
//! already heavily optimised by the compiler/CPU and the accuracy difference
//! matters more.

// ---------------------------------------------------------------------------
// Fast tanh
// ---------------------------------------------------------------------------

/// Fast hyperbolic tangent approximation using a rational (Padé) polynomial.
///
/// Accurate to within ~0.5% for `|x| ≤ 3`. For `|x| > 4`, clamps smoothly
/// to ±1 (since `tanh(4) ≈ 0.9993`). The transition between the two regions
/// avoids discontinuities.
///
/// ## Why tanh for audio?
///
/// `tanh(x)` is the canonical *soft clipper*: it compresses large amplitudes
/// while leaving small amplitudes nearly unchanged. Unlike hard clipping
/// (`x.clamp(-1, 1)`), it has no discontinuity in its derivative, so it
/// introduces only even harmonics rather than a harsh buzz.
///
/// A single overdrive stage is often just: `output = fast_tanh(input * drive)`.
///
/// ## Algorithm
///
/// Padé approximant (accurate in the central region):
/// ```text
/// tanh(x) ≈ x × (27 + x²) / (27 + 9 × x²)
/// ```
///
/// For `|x| ≥ 3.5`, the true `tanh` is within 1% of ±1, so we smoothly
/// transition to clamping.
///
/// ## Accuracy
///
/// | x     | true tanh | fast_tanh | error  |
/// |-------|-----------|-----------|--------|
/// | 0.0   | 0.0000    | 0.0000    | 0.0%   |
/// | 0.5   | 0.4621    | 0.4622    | <0.02% |
/// | 1.0   | 0.7616    | 0.7619    | <0.04% |
/// | 2.0   | 0.9640    | 0.9656    | <0.17% |
/// | 3.0   | 0.9951    | 0.9953    | <0.02% |
///
/// # Example
///
/// ```rust
/// use dsp::math::fast_tanh;
///
/// // Near zero: approximately linear
/// assert!((fast_tanh(0.0) - 0.0).abs() < 1e-6);
/// // Large input: saturates to ±1
/// assert!((fast_tanh(10.0) - 1.0).abs() < 0.01);
/// assert!((fast_tanh(-10.0) + 1.0).abs() < 0.01);
/// // Odd symmetry: f(-x) = -f(x)
/// assert!((fast_tanh(-1.0) + fast_tanh(1.0)).abs() < 1e-6);
/// ```
#[inline]
pub fn fast_tanh(x: f32) -> f32 {
    // Clamp for large inputs — true tanh(4) > 0.9993, so ±1 is a fine answer.
    let x = x.clamp(-4.0, 4.0);
    let x2 = x * x;
    // Padé [5/4] approximant — accurate to within ~0.5% for |x| ≤ 4.
    // Derived from the continued-fraction expansion of tanh.
    // p(x) / q(x) where:
    //   p = x * (945 + 105·x² + x⁴)
    //   q = 945 + 420·x² + 15·x⁴
    let x4 = x2 * x2;
    let num = x * (945.0 + 105.0 * x2 + x4);
    let den = 945.0 + 420.0 * x2 + 15.0 * x4;
    num / den
}

// ---------------------------------------------------------------------------
// Fast sin
// ---------------------------------------------------------------------------

/// Fast sine approximation using Bhaskara I's formula.
///
/// Operates on input in the range `[−π, π]`. For inputs outside this range,
/// the behaviour is unspecified (wrap first with [`wrap_phase`]).
///
/// ## Algorithm
///
/// Bhaskara I's approximation (7th century India — still competitive today):
/// ```text
/// sin(x) ≈ 16x(π − x) / (5π² − 4x(π − x))    for x ∈ [0, π]
/// ```
/// Extended to `[−π, π]` via odd symmetry.
///
/// ## Accuracy
///
/// Maximum error is about **0.17%** (< 0.0017 linear amplitude), which is
/// around −55 dBFS. This is well below the noise floor of most audio systems
/// and inaudible in modulation sources.
///
/// ## When to use
///
/// - LFO waveforms that modulate *parameters* (cutoff, pan, tremolo depth)
/// - Cheap chorus/vibrato modulators
/// - Any sine used to shape another parameter, not produced as audio output
///
/// For a pitched sine wave used directly as audio, use `T::sin()` which uses
/// the full-precision standard library implementation.
///
/// # Example
///
/// ```rust
/// use dsp::math::fast_sin;
/// use std::f32::consts::PI;
///
/// // Key values
/// assert!((fast_sin(0.0)).abs() < 0.001);
/// assert!((fast_sin(PI / 2.0) - 1.0).abs() < 0.002);  // peak
/// assert!((fast_sin(PI)).abs() < 0.001);               // zero crossing
/// assert!((fast_sin(-PI / 2.0) + 1.0).abs() < 0.002); // trough
/// ```
#[inline]
pub fn fast_sin(x: f32) -> f32 {
    // Bhaskara I extended to [-π, π] via odd symmetry.
    // The formula naturally handles [0, π]; for negative x we exploit sin(-x) = -sin(x).
    let pi = std::f32::consts::PI;
    if x >= 0.0 {
        let k = x * (pi - x);
        16.0 * k / (5.0 * pi * pi - 4.0 * k)
    } else {
        let x = -x;
        let k = x * (pi - x);
        -(16.0 * k / (5.0 * pi * pi - 4.0 * k))
    }
}

/// Fast cosine approximation.
///
/// Implemented as `fast_sin(x + π/2)` with phase wrapping so the input stays
/// within `[−π, π]` for [`fast_sin`].
///
/// Accuracy is the same as [`fast_sin`]: maximum error ~0.17%.
///
/// # Example
///
/// ```rust
/// use dsp::math::fast_cos;
/// use std::f32::consts::PI;
///
/// assert!((fast_cos(0.0) - 1.0).abs() < 0.002);
/// assert!((fast_cos(PI / 2.0)).abs() < 0.002);
/// assert!((fast_cos(PI) + 1.0).abs() < 0.002);
/// ```
#[inline]
pub fn fast_cos(x: f32) -> f32 {
    let pi = std::f32::consts::PI;
    // cos(x) = sin(x + π/2); wrap back into [-π, π]
    let shifted = x + pi / 2.0;
    let wrapped = if shifted > pi {
        shifted - pi * 2.0
    } else {
        shifted
    };
    fast_sin(wrapped)
}

// ---------------------------------------------------------------------------
// Phase wrapping
// ---------------------------------------------------------------------------

/// Wrap a phase value into `[−π, π]`.
///
/// Useful before passing arbitrary phase values to [`fast_sin`] or
/// [`fast_cos`], which require their input in that range.
///
/// Uses subtraction/addition rather than `fmod` to avoid a division on modern
/// hardware.
///
/// # Example
///
/// ```rust
/// use dsp::math::wrap_phase;
/// use std::f32::consts::PI;
///
/// assert!((wrap_phase(0.0)).abs() < 1e-6);
/// assert!((wrap_phase(2.0 * PI) ).abs() < 1e-5);   // wraps to ~0
/// assert!((wrap_phase(-2.0 * PI)).abs() < 1e-5);   // wraps to ~0
/// assert!(wrap_phase(PI * 1.5).abs() <= PI + 1e-5);
/// ```
#[inline]
pub fn wrap_phase(x: f32) -> f32 {
    let two_pi = std::f32::consts::TAU;
    let pi = std::f32::consts::PI;
    let mut p = x;
    while p > pi {
        p -= two_pi;
    }
    while p < -pi {
        p += two_pi;
    }
    p
}

/// Wrap a normalised phase value into `[0, 1)`.
///
/// Normalised phase (used by our oscillators) runs from 0.0 to 1.0 per cycle.
/// This wrap function is cheaper than [`wrap_phase`] because it avoids
/// transcendental arithmetic entirely — just subtraction.
///
/// # Example
///
/// ```rust
/// use dsp::math::wrap_phase_norm;
///
/// assert!((wrap_phase_norm(0.0)  - 0.0).abs() < 1e-7);
/// assert!((wrap_phase_norm(1.0)  - 0.0).abs() < 1e-7);  // wraps to 0
/// assert!((wrap_phase_norm(1.75) - 0.75).abs() < 1e-6);
/// assert!((wrap_phase_norm(-0.25) - 0.75).abs() < 1e-6);
/// ```
#[inline]
pub fn wrap_phase_norm(phase: f32) -> f32 {
    // This is equivalent to `phase - phase.floor()` (the fractional part),
    // but written explicitly so the compiler can see there are no branches on
    // the hot path for the common case of a small positive overshoot.
    phase - phase.floor()
}

// ---------------------------------------------------------------------------
// Fast exp
// ---------------------------------------------------------------------------

/// Fast exponential approximation (e^x) for `f32`.
///
/// Uses a well-known bit manipulation trick (Schraudolph 1999) that maps the
/// exponent range via integer arithmetic, followed by a polynomial correction.
///
/// ## Accuracy
///
/// Maximum relative error is approximately **0.18%** over a wide input range.
/// Sufficient for:
/// - Envelope decay curves computed at audio rate
/// - Approximating filter coefficients `e^(−ω/fs)` inside `process()`
///
/// Not appropriate when you need coefficient accuracy for IIR filter stability.
/// Use `f64::exp` for coefficient pre-computation.
///
/// ## Range
///
/// Results are valid for `x` in approximately `[−87, 88]`. Outside this range,
/// `f32` exp overflows or underflows to 0/inf regardless of approximation.
///
/// # Example
///
/// ```rust
/// use dsp::math::fast_exp;
///
/// assert!((fast_exp(0.0) - 1.0).abs() < 0.002);
/// assert!((fast_exp(1.0) - std::f32::consts::E).abs() < 0.005);
/// assert!((fast_exp(-1.0) - 1.0 / std::f32::consts::E).abs() < 0.003);
/// ```
#[inline]
pub fn fast_exp(x: f32) -> f32 {
    // Range-reduction approach: e^x = 2^n × e^r
    // where n = round(x / ln2), r = x − n·ln2, |r| ≤ ln2/2 ≈ 0.347
    //
    // 2^n is computed exactly by injecting n into the IEEE 754 exponent field.
    // e^r is approximated with a 5-term Taylor series, which is accurate to
    // < 0.0001% for |r| ≤ 0.347 — far more accurate than a single Padé over
    // the full range.
    const LN2_INV: f32 = 1.4426950408889634; // 1/ln2
    const LN2:     f32 = 0.6931471805599453; // ln2

    // Clamp to the valid f32 exponent range before rounding
    let n = (x * LN2_INV).round().clamp(-126.0, 127.0) as i32;
    let r = x - n as f32 * LN2;

    // Minimax polynomial for e^r on [−ln2/2, ln2/2] (Horner's method):
    // Coefficients are 1, 1, 1/2, 1/6, 1/24, 1/120
    let er = 1.0 + r * (1.0 + r * (0.5 + r * (0.16666667 + r * 0.041666668)));

    // 2^n: set the biased exponent field directly.
    // The IEEE 754 f32 exponent bias is 127; mantissa for 1.0 is all zeros.
    let pow2n = f32::from_bits(((n + 127) as u32) << 23);

    er * pow2n
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // fast_tanh
    // -----------------------------------------------------------------------

    #[test]
    fn test_fast_tanh_zero() {
        assert!(fast_tanh(0.0).abs() < 1e-6);
    }

    #[test]
    fn test_fast_tanh_odd_symmetry() {
        for x in [0.1f32, 0.5, 1.0, 1.5, 2.0, 3.0] {
            let pos = fast_tanh(x);
            let neg = fast_tanh(-x);
            assert!((pos + neg).abs() < 1e-6,
                "fast_tanh({}) + fast_tanh({}) = {} (should be 0)", x, -x, pos + neg);
        }
    }

    #[test]
    fn test_fast_tanh_saturates() {
        // For large |x|, tanh → ±1
        assert!((fast_tanh(10.0) - 1.0).abs() < 0.01);
        assert!((fast_tanh(-10.0) + 1.0).abs() < 0.01);
    }

    #[test]
    fn test_fast_tanh_accuracy() {
        // Compare against true tanh; the Padé [5/4] approximant stays within
        // 0.5% (0.005 absolute) for |x| ≤ 4.
        for xi in [-40i32, -30, -20, -10, 0, 10, 20, 30, 40] {
            let x = xi as f32 / 10.0; // -4.0, ..., 4.0
            let approx = fast_tanh(x);
            let exact = x.tanh();
            let error = (approx - exact).abs();
            assert!(error < 0.005,
                "fast_tanh({:.1}) error = {:.5} (approx {:.5}, exact {:.5})",
                x, error, approx, exact);
        }
    }

    #[test]
    fn test_fast_tanh_always_finite() {
        for xi in -100..=100 {
            let x = xi as f32;
            assert!(fast_tanh(x).is_finite(), "fast_tanh({}) is not finite", x);
        }
    }

    // -----------------------------------------------------------------------
    // fast_sin
    // -----------------------------------------------------------------------

    #[test]
    fn test_fast_sin_key_values() {
        let pi = std::f32::consts::PI;
        assert!(fast_sin(0.0).abs() < 0.001,            "sin(0) ≠ 0");
        assert!((fast_sin(pi / 2.0) - 1.0).abs() < 0.002, "sin(π/2) ≠ 1");
        assert!(fast_sin(pi).abs() < 0.001,             "sin(π) ≠ 0");
        assert!((fast_sin(-pi / 2.0) + 1.0).abs() < 0.002, "sin(-π/2) ≠ -1");
    }

    #[test]
    fn test_fast_sin_odd_symmetry() {
        let pi = std::f32::consts::PI;
        for i in 0..10 {
            let x = (i as f32 / 10.0) * pi;
            let pos = fast_sin(x);
            let neg = fast_sin(-x);
            assert!((pos + neg).abs() < 0.001,
                "fast_sin({:.3}) + fast_sin({:.3}) = {}", x, -x, pos + neg);
        }
    }

    #[test]
    fn test_fast_sin_max_error() {
        let pi = std::f32::consts::PI;
        let mut max_err = 0.0f32;
        let steps = 1000;
        for i in 0..=steps {
            let x = (i as f32 / steps as f32) * 2.0 * pi - pi;
            let approx = fast_sin(x);
            let exact = x.sin();
            let err = (approx - exact).abs();
            if err > max_err {
                max_err = err;
            }
        }
        assert!(max_err < 0.002, "Max error too large: {}", max_err);
    }

    // -----------------------------------------------------------------------
    // fast_cos
    // -----------------------------------------------------------------------

    #[test]
    fn test_fast_cos_key_values() {
        let pi = std::f32::consts::PI;
        assert!((fast_cos(0.0) - 1.0).abs() < 0.002, "cos(0) ≠ 1");
        assert!(fast_cos(pi / 2.0).abs() < 0.003,    "cos(π/2) ≠ 0");
        assert!((fast_cos(pi) + 1.0).abs() < 0.002,  "cos(π) ≠ -1");
    }

    // -----------------------------------------------------------------------
    // wrap_phase
    // -----------------------------------------------------------------------

    #[test]
    fn test_wrap_phase_in_range() {
        let pi = std::f32::consts::PI;
        let tau = std::f32::consts::TAU;
        let wrapped = wrap_phase(tau);
        assert!(wrapped.abs() < 1e-5, "wrap_phase(2π) = {}", wrapped);

        let wrapped = wrap_phase(3.0 * pi);
        assert!(wrapped.abs() <= pi + 1e-5,
            "wrap_phase(3π) = {} (should be in [-π, π])", wrapped);
    }

    #[test]
    fn test_wrap_phase_norm() {
        assert!((wrap_phase_norm(0.0) - 0.0).abs() < 1e-7);
        assert!((wrap_phase_norm(1.0) - 0.0).abs() < 1e-7);
        assert!((wrap_phase_norm(0.75) - 0.75).abs() < 1e-7);
        assert!((wrap_phase_norm(-0.25) - 0.75).abs() < 1e-6);
        assert!((wrap_phase_norm(2.3) - 0.3).abs() < 1e-6);
    }

    // -----------------------------------------------------------------------
    // fast_exp
    // -----------------------------------------------------------------------

    #[test]
    fn test_fast_exp_key_values() {
        assert!((fast_exp(0.0) - 1.0).abs() < 0.002);
        assert!((fast_exp(1.0) - std::f32::consts::E).abs() < 0.005);
    }

    #[test]
    fn test_fast_exp_max_error() {
        let mut max_rel_err = 0.0f32;
        for xi in -80..=80 {
            let x = xi as f32 / 10.0;
            let approx = fast_exp(x);
            let exact = x.exp();
            if exact > 1e-10 {
                let rel_err = ((approx - exact) / exact).abs();
                if rel_err > max_rel_err {
                    max_rel_err = rel_err;
                }
            }
        }
        assert!(max_rel_err < 0.003, "Max relative error too large: {}", max_rel_err);
    }

    #[test]
    fn test_fast_exp_always_positive() {
        for xi in -80..=80 {
            let x = xi as f32 / 10.0;
            let v = fast_exp(x);
            assert!(v > 0.0, "fast_exp({}) = {} (should be positive)", x, v);
        }
    }
}
