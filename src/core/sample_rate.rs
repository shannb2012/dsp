//! Sample rate type and conversions.

use std::fmt;

/// Type-safe wrapper around sample rate.
///
/// Ensures sample rate is always positive and provides
/// useful conversions and calculations.
///
/// # Example
///
/// ```rust
/// use dsp::core::SampleRate;
///
/// let sr = SampleRate::new(44100.0).unwrap();
/// assert_eq!(sr.hz(), 44100.0);
/// assert_eq!(sr.nyquist(), 22050.0);
///
/// // Convert seconds to samples
/// let one_second = sr.seconds_to_samples(1.0);
/// assert_eq!(one_second, 44100.0);
/// ```
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SampleRate(f64);

/// Error type for invalid sample rates
#[derive(Debug, Clone, PartialEq)]
pub enum SampleRateError {
    /// Sample rate must be positive
    NonPositive,
    /// Sample rate is unrealistically low
    TooLow,
    /// Sample rate is unrealistically high
    TooHigh,
}

impl fmt::Display for SampleRateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonPositive => write!(f, "Sample rate must be positive"),
            Self::TooLow => write!(f, "Sample rate is too low (minimum 1000 Hz)"),
            Self::TooHigh => write!(f, "Sample rate is too high (maximum 1000000 Hz)"),
        }
    }
}

impl std::error::Error for SampleRateError {}

impl SampleRate {
    /// Minimum valid sample rate (1 kHz)
    pub const MIN: f64 = 1000.0;

    /// Maximum valid sample rate (1 MHz)
    pub const MAX: f64 = 1_000_000.0;

    /// Standard CD quality sample rate (44.1 kHz)
    pub const CD: Self = Self(44100.0);

    /// Professional audio sample rate (48 kHz)
    pub const PROFESSIONAL: Self = Self(48000.0);

    /// High resolution audio sample rate (96 kHz)
    pub const HIGH_RES: Self = Self(96000.0);

    /// Create a new sample rate.
    ///
    /// # Errors
    ///
    /// Returns an error if the sample rate is not positive or outside
    /// reasonable bounds.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::core::SampleRate;
    ///
    /// let sr = SampleRate::new(48000.0).unwrap();
    /// assert_eq!(sr.hz(), 48000.0);
    /// ```
    pub fn new(rate: f64) -> Result<Self, SampleRateError> {
        if rate <= 0.0 {
            Err(SampleRateError::NonPositive)
        } else if rate < Self::MIN {
            Err(SampleRateError::TooLow)
        } else if rate > Self::MAX {
            Err(SampleRateError::TooHigh)
        } else {
            Ok(Self(rate))
        }
    }

    /// Create a new sample rate without validation.
    ///
    /// # Safety
    ///
    /// The caller must ensure the sample rate is positive.
    /// This is primarily for use with constant values known to be valid.
    #[inline]
    pub const fn new_unchecked(rate: f64) -> Self {
        Self(rate)
    }

    /// Get the sample rate in Hz.
    #[inline]
    pub fn hz(self) -> f64 {
        self.0
    }

    /// Get the Nyquist frequency (half the sample rate).
    ///
    /// This is the maximum frequency that can be represented
    /// without aliasing.
    #[inline]
    pub fn nyquist(self) -> f64 {
        self.0 * 0.5
    }

    /// Get the period of one sample in seconds.
    #[inline]
    pub fn period(self) -> f64 {
        1.0 / self.0
    }

    /// Convert seconds to samples.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::core::SampleRate;
    ///
    /// let sr = SampleRate::new(44100.0).unwrap();
    /// let samples = sr.seconds_to_samples(0.1); // 100ms
    /// assert_eq!(samples, 4410.0);
    /// ```
    #[inline]
    pub fn seconds_to_samples(self, seconds: f64) -> f64 {
        seconds * self.0
    }

    /// Convert samples to seconds.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::core::SampleRate;
    ///
    /// let sr = SampleRate::new(44100.0).unwrap();
    /// let seconds = sr.samples_to_seconds(44100.0);
    /// assert_eq!(seconds, 1.0);
    /// ```
    #[inline]
    pub fn samples_to_seconds(self, samples: f64) -> f64 {
        samples / self.0
    }

    /// Convert milliseconds to samples.
    #[inline]
    pub fn ms_to_samples(self, ms: f64) -> f64 {
        ms * 0.001 * self.0
    }

    /// Convert samples to milliseconds.
    #[inline]
    pub fn samples_to_ms(self, samples: f64) -> f64 {
        samples / self.0 * 1000.0
    }

    /// Convert frequency in Hz to angular frequency (radians per sample).
    ///
    /// This is useful for oscillators that work with phase accumulation.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::core::SampleRate;
    ///
    /// let sr = SampleRate::new(44100.0).unwrap();
    /// let omega = sr.freq_to_angular(440.0); // A4
    /// // omega = 2π * 440 / 44100
    /// ```
    #[inline]
    pub fn freq_to_angular(self, freq_hz: f64) -> f64 {
        2.0 * std::f64::consts::PI * freq_hz / self.0
    }

    /// Convert angular frequency (radians per sample) to Hz.
    #[inline]
    pub fn angular_to_freq(self, omega: f64) -> f64 {
        omega * self.0 / (2.0 * std::f64::consts::PI)
    }

    /// Get the phase increment per sample for a given frequency.
    ///
    /// Phase is normalized to [0, 1) where 1.0 represents one full cycle.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::core::SampleRate;
    ///
    /// let sr = SampleRate::new(44100.0).unwrap();
    /// let increment = sr.freq_to_phase_increment(440.0);
    /// // For a 440 Hz sine wave at 44.1 kHz, we advance by
    /// // 440/44100 ≈ 0.00997 of a cycle per sample
    /// ```
    #[inline]
    pub fn freq_to_phase_increment(self, freq_hz: f64) -> f64 {
        freq_hz / self.0
    }

    /// Convert phase increment to frequency in Hz.
    #[inline]
    pub fn phase_increment_to_freq(self, increment: f64) -> f64 {
        increment * self.0
    }
}

impl fmt::Display for SampleRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} Hz", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_valid() {
        let sr = SampleRate::new(44100.0).unwrap();
        assert_eq!(sr.hz(), 44100.0);
    }

    #[test]
    fn test_new_invalid() {
        assert!(SampleRate::new(0.0).is_err());
        assert!(SampleRate::new(-1.0).is_err());
        assert!(SampleRate::new(100.0).is_err()); // Too low
        assert!(SampleRate::new(2_000_000.0).is_err()); // Too high
    }

    #[test]
    fn test_constants() {
        assert_eq!(SampleRate::CD.hz(), 44100.0);
        assert_eq!(SampleRate::PROFESSIONAL.hz(), 48000.0);
    }

    #[test]
    fn test_nyquist() {
        let sr = SampleRate::new(44100.0).unwrap();
        assert_eq!(sr.nyquist(), 22050.0);
    }

    #[test]
    fn test_period() {
        let sr = SampleRate::new(1000.0).unwrap();
        assert_eq!(sr.period(), 0.001);
    }

    #[test]
    fn test_time_conversions() {
        let sr = SampleRate::new(44100.0).unwrap();

        // Seconds to samples
        assert_eq!(sr.seconds_to_samples(1.0), 44100.0);
        assert_eq!(sr.seconds_to_samples(0.5), 22050.0);

        // Samples to seconds
        assert_eq!(sr.samples_to_seconds(44100.0), 1.0);
        assert_eq!(sr.samples_to_seconds(22050.0), 0.5);

        // Milliseconds
        assert!((sr.ms_to_samples(1000.0) - 44100.0).abs() < 0.001);
        assert!((sr.samples_to_ms(44100.0) - 1000.0).abs() < 0.001);
    }

    #[test]
    fn test_frequency_conversions() {
        let sr = SampleRate::new(44100.0).unwrap();

        // 440 Hz (A4)
        let freq = 440.0;
        let omega = sr.freq_to_angular(freq);
        let freq_back = sr.angular_to_freq(omega);
        assert!((freq_back - freq).abs() < 0.001);

        // Phase increment
        let increment = sr.freq_to_phase_increment(freq);
        assert!((increment - (440.0 / 44100.0)).abs() < 0.0001);

        let freq_from_increment = sr.phase_increment_to_freq(increment);
        assert!((freq_from_increment - freq).abs() < 0.001);
    }

    #[test]
    fn test_display() {
        let sr = SampleRate::new(44100.0).unwrap();
        assert_eq!(format!("{}", sr), "44100 Hz");
    }
}
