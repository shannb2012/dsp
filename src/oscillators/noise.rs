//! Pseudo-random noise generators: white, pink, and brown.

/// Spectral character of the generated noise.
///
/// | Color | Spectrum | Character                    |
/// |-------|----------|------------------------------|
/// | White | Flat      | Hiss; equal energy per bin   |
/// | Pink  | 1/f       | Natural; equal energy/octave |
/// | Brown | 1/f²      | Deep rumble                  |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseColor {
    /// **White noise** — flat power spectral density.
    ///
    /// Each frequency bin carries equal energy. Sounds like a television
    /// hiss or a waterfall. Useful for percussion synthesis and testing.
    White,

    /// **Pink noise** — power falls off at 3 dB per octave (1/f spectrum).
    ///
    /// Equal energy per octave, which matches the sensitivity of the human
    /// ear far better than white noise. Pink noise is ubiquitous in nature
    /// (music, ocean waves, biological rhythms) and is the standard
    /// calibration signal in audio engineering.
    Pink,

    /// **Brown noise** — power falls off at 6 dB per octave (1/f² spectrum).
    ///
    /// Also called *red* noise or Brownian noise (random walk). Sounds like
    /// a deep ocean rumble. Obtained by integrating white noise.
    Brown,
}

/// A pseudo-random noise generator producing white, pink, or brown noise.
///
/// Uses an XOR-shift 64-bit PRNG — period 2⁶⁴−1, no heap allocations,
/// deterministic from a given seed, and fast enough for audio use.
///
/// ## Choosing a color
///
/// - **White** — synthesis (percussion, FM operator noise), testing.
/// - **Pink** — calibration, natural ambience, random modulation sources.
/// - **Brown** — sub-bass textures, slow random modulation.
///
/// ## Real-time safety
///
/// `process()` makes no allocations and runs in O(1) time.
/// All state is embedded in the struct.
///
/// ## Example
///
/// ```rust
/// use dsp::oscillators::{NoiseGenerator, NoiseColor};
///
/// let mut noise = NoiseGenerator::new(NoiseColor::Pink, 12345);
///
/// for _ in 0..512 {
///     let s = noise.process();
///     assert!(s.is_finite());
/// }
/// ```
pub struct NoiseGenerator {
    /// XOR-shift 64 PRNG state. Must never be zero.
    state: u64,

    /// Noise color (spectral shaping).
    color: NoiseColor,

    /// Pink noise: Kellet filter running sums (6 first-order IIR filters).
    ///
    /// Each entry models noise energy in roughly one octave band.
    pink_b: [f64; 6],

    /// Brown noise: leaky integrator state.
    brown_last: f64,
}

impl NoiseGenerator {
    /// Create a new noise generator.
    ///
    /// # Arguments
    ///
    /// * `color` — spectral character of the output noise.
    /// * `seed` — initial PRNG seed. Use different seeds for independent
    ///   generators (e.g. left and right channels). A seed of `0` is
    ///   silently replaced with `1`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::oscillators::{NoiseGenerator, NoiseColor};
    ///
    /// let mut white = NoiseGenerator::new(NoiseColor::White, 0xDEAD_BEEF);
    /// let s = white.process();
    /// assert!(s.is_finite());
    /// ```
    pub fn new(color: NoiseColor, seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
            color,
            pink_b: [0.0; 6],
            brown_last: 0.0,
        }
    }

    /// Change the noise color without resetting the PRNG or filter state.
    pub fn set_color(&mut self, color: NoiseColor) {
        self.color = color;
    }

    /// Get the current noise color.
    pub fn color(&self) -> NoiseColor {
        self.color
    }

    /// Generate and return the next noise sample.
    ///
    /// Output is approximately in [−1, 1] for all colors.
    ///
    /// **Real-time safe**: no allocations; O(1) time.
    #[inline]
    pub fn process(&mut self) -> f32 {
        let white = self.next_white();
        match self.color {
            NoiseColor::White => white,
            NoiseColor::Pink  => self.apply_pink(white),
            NoiseColor::Brown => self.apply_brown(white),
        }
    }

    /// Generate a white noise sample directly, ignoring the current color setting.
    ///
    /// Useful when you need uncorrelated white noise in addition to the
    /// colored output (e.g. for a separate noise modulation path).
    #[inline]
    pub fn white(&mut self) -> f32 {
        self.next_white()
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Advance the XOR-shift PRNG and return white noise in [−1, 1].
    ///
    /// XOR-shift 64 has excellent statistical properties for audio use
    /// (passes all BigCrush tests) and is faster than LFSR variants.
    #[inline]
    fn next_white(&mut self) -> f32 {
        // Xorshift64 — Marsaglia (2003)
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        // Reinterpret bits as a signed i64, then normalise to [-1, 1].
        (x as i64 as f64 / i64::MAX as f64) as f32
    }

    /// Apply Kellet's parallel-IIR pink filter to a white noise sample.
    ///
    /// Kellet (1999) approximates a 1/f spectrum with 6 first-order IIR
    /// stages, each tuned to contribute energy in a different octave band.
    /// The sum is accurate to ±1.5 dB over the audible range.
    ///
    /// Reference: Paul Kellet, "A Compromise Pink Noise Algorithm",
    ///             music-dsp mailing list, 1999.
    #[inline]
    fn apply_pink(&mut self, white: f32) -> f32 {
        let w = white as f64;

        // Each coefficient pair: (pole, gain) tuned to one octave band.
        self.pink_b[0] = 0.99886 * self.pink_b[0] + w * 0.0555179;
        self.pink_b[1] = 0.99332 * self.pink_b[1] + w * 0.0750759;
        self.pink_b[2] = 0.96900 * self.pink_b[2] + w * 0.1538520;
        self.pink_b[3] = 0.86650 * self.pink_b[3] + w * 0.3104856;
        self.pink_b[4] = 0.55000 * self.pink_b[4] + w * 0.5329522;
        // Stage 5 uses a negative pole to correct for a slight DC tilt:
        self.pink_b[5] = -0.7616 * self.pink_b[5] + w * 0.0168980;

        let pink = self.pink_b[0] + self.pink_b[1] + self.pink_b[2]
                 + self.pink_b[3] + self.pink_b[4] + self.pink_b[5]
                 + w * 0.5362;

        // Scale to approximately [-1, 1] (empirical constant for this filter).
        (pink * 0.11) as f32
    }

    /// Apply a leaky integrator to white noise to produce brown (1/f²) noise.
    ///
    /// Integration of white noise raises the spectral density by 6 dB/oct,
    /// converting a flat spectrum to 1/f². A small leakage term prevents
    /// unbounded DC drift.
    ///
    /// The output is scaled to fit approximately within [−1, 1].
    #[inline]
    fn apply_brown(&mut self, white: f32) -> f32 {
        // Leaky integration: accumulate white noise with slow leak.
        // leak = 0.998 keeps the integrator stable indefinitely.
        self.brown_last = (self.brown_last + 0.02 * white as f64) * 0.998;

        // Empirical scale factor to keep output near [-1, 1].
        (self.brown_last * 3.5) as f32
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_white_is_finite() {
        let mut ng = NoiseGenerator::new(NoiseColor::White, 1);
        for i in 0..10000 {
            let s = ng.process();
            assert!(s.is_finite(), "sample {i} non-finite: {s}");
        }
    }

    #[test]
    fn test_pink_is_finite() {
        let mut ng = NoiseGenerator::new(NoiseColor::Pink, 1);
        for i in 0..10000 {
            let s = ng.process();
            assert!(s.is_finite(), "sample {i} non-finite: {s}");
        }
    }

    #[test]
    fn test_brown_is_finite() {
        let mut ng = NoiseGenerator::new(NoiseColor::Brown, 1);
        for i in 0..10000 {
            let s = ng.process();
            assert!(s.is_finite(), "sample {i} non-finite: {s}");
        }
    }

    #[test]
    fn test_white_is_in_range() {
        // White noise should be roughly in [-1, 1]
        let mut ng = NoiseGenerator::new(NoiseColor::White, 42);
        for i in 0..10000 {
            let s = ng.process();
            assert!(s >= -1.0 && s <= 1.0, "sample {i} out of range: {s}");
        }
    }

    #[test]
    fn test_white_zero_mean() {
        // Over many samples white noise should be approximately zero-mean.
        let mut ng = NoiseGenerator::new(NoiseColor::White, 12345);
        let n = 100_000;
        let sum: f64 = (0..n).map(|_| ng.process() as f64).sum();
        let mean = sum / n as f64;
        assert!(mean.abs() < 0.01, "white noise mean should be ~0, got {mean:.4}");
    }

    #[test]
    fn test_different_seeds_give_different_sequences() {
        let mut ng1 = NoiseGenerator::new(NoiseColor::White, 1);
        let mut ng2 = NoiseGenerator::new(NoiseColor::White, 2);

        // Draw 10 samples; they should not all be identical
        let seq1: Vec<f32> = (0..10).map(|_| ng1.process()).collect();
        let seq2: Vec<f32> = (0..10).map(|_| ng2.process()).collect();
        assert_ne!(seq1, seq2, "different seeds should produce different sequences");
    }

    #[test]
    fn test_zero_seed_replaced_with_one() {
        // seed=0 would make xorshift produce 0 forever; we force it to 1
        let mut ng = NoiseGenerator::new(NoiseColor::White, 0);
        // If state were stuck at 0 all samples would be 0.0
        let any_nonzero = (0..100).any(|_| ng.process() != 0.0);
        assert!(any_nonzero, "zero seed produced all-zero output");
    }

    #[test]
    fn test_deterministic_with_same_seed() {
        let mut ng1 = NoiseGenerator::new(NoiseColor::White, 9999);
        let mut ng2 = NoiseGenerator::new(NoiseColor::White, 9999);
        for i in 0..100 {
            assert_eq!(
                ng1.process(), ng2.process(),
                "same seed should produce identical output at sample {i}"
            );
        }
    }

    #[test]
    fn test_set_color_switches_behavior() {
        let mut ng = NoiseGenerator::new(NoiseColor::White, 1);
        ng.set_color(NoiseColor::Pink);
        assert_eq!(ng.color(), NoiseColor::Pink);
        let s = ng.process();
        assert!(s.is_finite());
    }

    #[test]
    fn test_white_helper_independent_of_color() {
        // white() should always return white noise regardless of color setting
        let mut ng = NoiseGenerator::new(NoiseColor::Pink, 7777);
        for i in 0..100 {
            let s = ng.white();
            assert!(s >= -1.0 && s <= 1.0, "white() sample {i} out of range: {s}");
        }
    }

    #[test]
    fn test_pink_noise_roughly_bounded() {
        // Pink noise should stay roughly within [-1, 1] after warm-up.
        let mut ng = NoiseGenerator::new(NoiseColor::Pink, 42);
        // Warm up the filter state
        for _ in 0..1000 { ng.process(); }
        let max_abs = (0..10000)
            .map(|_| ng.process().abs())
            .fold(0.0f32, f32::max);
        assert!(max_abs < 2.0, "pink noise exceeded 2.0: max_abs={max_abs}");
    }

    #[test]
    fn test_brown_noise_roughly_bounded() {
        // Brown noise should stay roughly within [-1, 1] in steady state.
        let mut ng = NoiseGenerator::new(NoiseColor::Brown, 42);
        for _ in 0..1000 { ng.process(); }
        let max_abs = (0..10000)
            .map(|_| ng.process().abs())
            .fold(0.0f32, f32::max);
        assert!(max_abs < 2.0, "brown noise exceeded 2.0: max_abs={max_abs}");
    }
}
