//! Window functions for spectral analysis and granular synthesis.
//!
//! A **window function** is a finite-length sequence of weights, typically
//! tapering smoothly to zero at both ends. Multiplying a block of audio by
//! a window before analysis (FFT) or resynthesis (granular) prevents
//! discontinuities at block boundaries that would corrupt the result.
//!
//! ## Why windows are necessary
//!
//! The FFT assumes the input signal is *periodic* — that the block repeats
//! indefinitely. If the signal doesn't start and end at exactly the same value,
//! the hard edge looks like a sharp transient to the FFT, smearing energy across
//! all frequencies (spectral leakage). A window function gently fades the
//! signal to zero at both edges, eliminating the discontinuity.
//!
//! ## Choosing a window
//!
//! | Window      | Main lobe | Sidelobe | Best used for                          |
//! |-------------|-----------|----------|----------------------------------------|
//! | Rectangular | Narrowest | Highest  | When you can guarantee periodicity     |
//! | Hann        | Moderate  | Low      | General-purpose analysis & synthesis   |
//! | Hamming     | Moderate  | Moderate | Slightly better stopband than Hann     |
//! | Blackman    | Widest    | Lowest   | Maximum sidelobe rejection             |
//!
//! **For most audio analysis, Hann is the right choice.** It has a good balance
//! between frequency resolution and leakage rejection.
//!
//! ## Usage pattern
//!
//! ```rust
//! use dsp::math::{fill_hann, apply_window};
//!
//! let mut window = vec![0.0f32; 1024];
//! fill_hann(&mut window);
//!
//! // Apply to a block of audio before FFT
//! let mut block = vec![0.5f32; 1024];
//! apply_window(&mut block, &window);
//! ```

use crate::core::Sample;

// ---------------------------------------------------------------------------
// Single-coefficient functions
// ---------------------------------------------------------------------------

/// Compute a single Hann window coefficient at position `n` in a window of
/// size `size`.
///
/// The Hann window is defined as:
/// ```text
/// w[n] = 0.5 × (1 − cos(2π × n / (N − 1)))
/// ```
///
/// Values at the edges (`n=0` and `n=N-1`) are exactly 0. The peak is 1.0 at
/// the centre.
///
/// # Panics
///
/// Panics if `size == 0`.
#[inline]
pub fn hann_coefficient<T: Sample>(n: usize, size: usize) -> T {
    assert!(size > 0, "Window size must be > 0");
    if size == 1 {
        return T::ONE;
    }
    let phase = std::f64::consts::TAU * n as f64 / (size - 1) as f64;
    T::from_f64(0.5 * (1.0 - phase.cos()))
}

/// Compute a single Hamming window coefficient at position `n`.
///
/// The Hamming window is:
/// ```text
/// w[n] = 0.54 − 0.46 × cos(2π × n / (N − 1))
/// ```
///
/// Unlike Hann, the Hamming window does **not** reach zero at the edges
/// (the edges are ~0.08). This gives a slightly lower sidelobe level than Hann
/// when the signal genuinely starts and ends at small-but-nonzero values.
///
/// # Panics
///
/// Panics if `size == 0`.
#[inline]
pub fn hamming_coefficient<T: Sample>(n: usize, size: usize) -> T {
    assert!(size > 0, "Window size must be > 0");
    if size == 1 {
        return T::ONE;
    }
    let phase = std::f64::consts::TAU * n as f64 / (size - 1) as f64;
    T::from_f64(0.54 - 0.46 * phase.cos())
}

/// Compute a single Blackman window coefficient at position `n`.
///
/// The Blackman window is:
/// ```text
/// w[n] = 0.42 − 0.5 × cos(2π × n / (N−1)) + 0.08 × cos(4π × n / (N−1))
/// ```
///
/// Uses a second cosine harmonic to push sidelobes down further (~−74 dBc vs
/// ~−44 dBc for Hann). The tradeoff is a wider main lobe (reduced frequency
/// resolution). Good choice when adjacent-frequency isolation is critical.
///
/// # Panics
///
/// Panics if `size == 0`.
#[inline]
pub fn blackman_coefficient<T: Sample>(n: usize, size: usize) -> T {
    assert!(size > 0, "Window size must be > 0");
    if size == 1 {
        return T::ONE;
    }
    let phase  = std::f64::consts::TAU * n as f64 / (size - 1) as f64;
    T::from_f64(0.42 - 0.5 * phase.cos() + 0.08 * (2.0 * phase).cos())
}

// ---------------------------------------------------------------------------
// Slice-filling functions
// ---------------------------------------------------------------------------

/// Fill a slice with a Hann window.
///
/// Equivalent to calling [`hann_coefficient`] for each index, but more
/// ergonomic for the common case of pre-computing the window once.
///
/// # Example
///
/// ```rust
/// use dsp::math::fill_hann;
///
/// let mut window = vec![0.0f32; 8];
/// fill_hann(&mut window);
///
/// // Edges should be (near) zero
/// assert!(window[0].abs() < 1e-6);
/// assert!(window[7].abs() < 1e-6);
/// // Centre should be near 1.0
/// assert!(window[4] > 0.9);
/// ```
pub fn fill_hann<T: Sample>(output: &mut [T]) {
    let size = output.len();
    for (n, slot) in output.iter_mut().enumerate() {
        *slot = hann_coefficient(n, size);
    }
}

/// Fill a slice with a Hamming window.
///
/// # Example
///
/// ```rust
/// use dsp::math::fill_hamming;
///
/// let mut window = vec![0.0f32; 8];
/// fill_hamming(&mut window);
///
/// // Edges: Hamming does not reach exactly zero
/// assert!(window[0] > 0.05 && window[0] < 0.10);
/// ```
pub fn fill_hamming<T: Sample>(output: &mut [T]) {
    let size = output.len();
    for (n, slot) in output.iter_mut().enumerate() {
        *slot = hamming_coefficient(n, size);
    }
}

/// Fill a slice with a Blackman window.
///
/// # Example
///
/// ```rust
/// use dsp::math::fill_blackman;
///
/// let mut window = vec![0.0f32; 8];
/// fill_blackman(&mut window);
///
/// // Edges should be (near) zero
/// assert!(window[0].abs() < 1e-6);
/// assert!(window[7].abs() < 1e-6);
/// ```
pub fn fill_blackman<T: Sample>(output: &mut [T]) {
    let size = output.len();
    for (n, slot) in output.iter_mut().enumerate() {
        *slot = blackman_coefficient(n, size);
    }
}

// ---------------------------------------------------------------------------
// Application helper
// ---------------------------------------------------------------------------

/// Multiply a buffer of audio samples in-place by a pre-computed window.
///
/// This is the core operation in windowed analysis and granular synthesis.
/// Both slices must have the same length.
///
/// # Panics
///
/// Panics if `audio` and `window` have different lengths.
///
/// # Real-time safety
///
/// No allocations. O(n) time.
///
/// # Example
///
/// ```rust
/// use dsp::math::{fill_hann, apply_window};
///
/// let mut window = vec![0.0f32; 8];
/// fill_hann(&mut window);
///
/// let mut audio = vec![1.0f32; 8]; // Constant signal
/// apply_window(&mut audio, &window);
///
/// // Edges of audio should now be silenced
/// assert!(audio[0].abs() < 1e-6);
/// assert!(audio[7].abs() < 1e-6);
/// // Centre should remain near 1.0
/// assert!(audio[4] > 0.9);
/// ```
pub fn apply_window<T: Sample>(audio: &mut [T], window: &[T]) {
    assert_eq!(
        audio.len(),
        window.len(),
        "apply_window: audio and window slices must have the same length"
    );
    for (sample, &w) in audio.iter_mut().zip(window.iter()) {
        *sample = *sample * w;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const N: usize = 256;

    // -----------------------------------------------------------------------
    // Hann window
    // -----------------------------------------------------------------------

    #[test]
    fn test_hann_edges_are_zero() {
        let first = hann_coefficient::<f32>(0, N);
        let last  = hann_coefficient::<f32>(N - 1, N);
        assert!(first.abs() < 1e-6, "Hann[0] should be 0, got {}", first);
        assert!(last.abs() < 1e-6,  "Hann[N-1] should be 0, got {}", last);
    }

    #[test]
    fn test_hann_peak_at_centre() {
        // For even N, the maximum is at N/2 and (N-2)/2
        let mid_left  = hann_coefficient::<f32>(N / 2 - 1, N);
        let mid_right = hann_coefficient::<f32>(N / 2, N);
        assert!(mid_left > 0.99 && mid_left <= 1.0 + 1e-6);
        assert!(mid_right > 0.99 && mid_right <= 1.0 + 1e-6);
    }

    #[test]
    fn test_hann_all_values_in_range() {
        for n in 0..N {
            let w = hann_coefficient::<f32>(n, N);
            assert!(w >= -1e-6 && w <= 1.0 + 1e-6,
                "Hann[{}] out of [0,1]: {}", n, w);
        }
    }

    #[test]
    fn test_hann_is_symmetric() {
        for n in 0..N / 2 {
            let left  = hann_coefficient::<f32>(n, N);
            let right = hann_coefficient::<f32>(N - 1 - n, N);
            assert!((left - right).abs() < 1e-6,
                "Hann not symmetric at n={}: {} vs {}", n, left, right);
        }
    }

    #[test]
    fn test_fill_hann_matches_coefficient() {
        let mut window = vec![0.0f32; N];
        fill_hann(&mut window);

        for (n, &w) in window.iter().enumerate() {
            let expected = hann_coefficient::<f32>(n, N);
            assert!((w - expected).abs() < 1e-7);
        }
    }

    // -----------------------------------------------------------------------
    // Hamming window
    // -----------------------------------------------------------------------

    #[test]
    fn test_hamming_edges_not_zero() {
        // Hamming does not reach zero — edges are ~0.08
        let first = hamming_coefficient::<f32>(0, N);
        assert!(first > 0.05 && first < 0.1,
            "Hamming[0] should be ~0.08, got {}", first);
    }

    #[test]
    fn test_hamming_is_symmetric() {
        for n in 0..N / 2 {
            let left  = hamming_coefficient::<f32>(n, N);
            let right = hamming_coefficient::<f32>(N - 1 - n, N);
            assert!((left - right).abs() < 1e-6,
                "Hamming not symmetric at n={}: {} vs {}", n, left, right);
        }
    }

    #[test]
    fn test_hamming_all_positive() {
        for n in 0..N {
            let w = hamming_coefficient::<f32>(n, N);
            assert!(w >= 0.0, "Hamming[{}] is negative: {}", n, w);
        }
    }

    // -----------------------------------------------------------------------
    // Blackman window
    // -----------------------------------------------------------------------

    #[test]
    fn test_blackman_edges_near_zero() {
        let first = blackman_coefficient::<f32>(0, N);
        let last  = blackman_coefficient::<f32>(N - 1, N);
        // 0.42 - 0.5 - 0.08 should be near zero (floating point)
        assert!(first.abs() < 1e-5, "Blackman[0] should be ~0, got {}", first);
        assert!(last.abs() < 1e-5,  "Blackman[N-1] should be ~0, got {}", last);
    }

    #[test]
    fn test_blackman_is_symmetric() {
        for n in 0..N / 2 {
            let left  = blackman_coefficient::<f32>(n, N);
            let right = blackman_coefficient::<f32>(N - 1 - n, N);
            assert!((left - right).abs() < 1e-6,
                "Blackman not symmetric at n={}: {} vs {}", n, left, right);
        }
    }

    // -----------------------------------------------------------------------
    // apply_window
    // -----------------------------------------------------------------------

    #[test]
    fn test_apply_window_zeroes_edges() {
        let size = 64;
        let mut window = vec![0.0f32; size];
        fill_hann(&mut window);

        let mut audio = vec![1.0f32; size]; // constant signal
        apply_window(&mut audio, &window);

        assert!(audio[0].abs() < 1e-6, "Edge not zeroed: {}", audio[0]);
        assert!(audio[size - 1].abs() < 1e-6, "Edge not zeroed: {}", audio[size - 1]);
        assert!(audio[size / 2] > 0.9);
    }

    #[test]
    fn test_apply_window_all_finite() {
        let size = 512;
        let mut window = vec![0.0f32; size];
        fill_blackman(&mut window);

        let mut audio: Vec<f32> = (0..size).map(|i| (i as f32 * 0.01).sin()).collect();
        apply_window(&mut audio, &window);

        for (i, &s) in audio.iter().enumerate() {
            assert!(s.is_finite(), "Non-finite at sample {}: {}", i, s);
        }
    }

    #[test]
    fn test_window_with_f64() {
        let w: f64 = hann_coefficient(32, 64);
        assert!(w > 0.9 && w <= 1.0 + 1e-12);
    }
}
