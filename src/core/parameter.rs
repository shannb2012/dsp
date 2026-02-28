//! Type-safe parameter types for DSP.
//!
//! These types help prevent common mistakes by encoding parameter
//! ranges and units in the type system.

use crate::core::Sample;
use std::fmt;

/// A normalized parameter value in the range [0.0, 1.0].
///
/// This is commonly used for interfacing with plugin hosts,
/// which typically work with normalized parameter values.
///
/// # Example
///
/// ```rust
/// use dsp::core::NormalizedParam;
///
/// let param = NormalizedParam::new(0.5).unwrap();
/// assert_eq!(param.value(), 0.5);
/// ```
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct NormalizedParam(f32);

impl NormalizedParam {
    /// Create a new normalized parameter.
    ///
    /// # Errors
    ///
    /// Returns `None` if the value is outside [0.0, 1.0].
    pub fn new(value: f32) -> Option<Self> {
        if (0.0..=1.0).contains(&value) {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Create a normalized parameter without validation.
    ///
    /// # Safety
    ///
    /// Caller must ensure value is in [0.0, 1.0].
    #[inline]
    pub const fn new_unchecked(value: f32) -> Self {
        Self(value)
    }

    /// Get the raw value.
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Map to a linear range.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::core::NormalizedParam;
    ///
    /// let param = NormalizedParam::new(0.5).unwrap();
    /// let mapped = param.map_linear(20.0, 20000.0);
    /// assert_eq!(mapped, 10010.0);
    /// ```
    #[inline]
    pub fn map_linear(self, min: f32, max: f32) -> f32 {
        min + self.0 * (max - min)
    }

    /// Map to a logarithmic range.
    ///
    /// Useful for frequency and gain parameters which are
    /// typically perceived logarithmically.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::core::NormalizedParam;
    ///
    /// let param = NormalizedParam::new(0.5).unwrap();
    /// let freq = param.map_log(20.0, 20000.0);
    /// // Middle value gives sqrt(20 * 20000) ≈ 632 Hz
    /// ```
    #[inline]
    pub fn map_log(self, min: f32, max: f32) -> f32 {
        min * (max / min).powf(self.0)
    }

    /// Map to an exponential range.
    ///
    /// Useful for time-based parameters like attack and release.
    #[inline]
    pub fn map_exp(self, min: f32, max: f32, curve: f32) -> f32 {
        min + (max - min) * self.0.powf(curve)
    }
}

impl Default for NormalizedParam {
    fn default() -> Self {
        Self(0.5)
    }
}

impl fmt::Display for NormalizedParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}%", self.0 * 100.0)
    }
}

/// A frequency value in Hertz.
///
/// # Example
///
/// ```rust
/// use dsp::core::FrequencyHz;
///
/// let freq = FrequencyHz::new(440.0); // A4
/// assert_eq!(freq.hz(), 440.0);
/// ```
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct FrequencyHz(f64);

impl FrequencyHz {
    /// Create a frequency value.
    #[inline]
    pub const fn new(hz: f64) -> Self {
        Self(hz)
    }

    /// Get the frequency in Hz.
    #[inline]
    pub fn hz(self) -> f64 {
        self.0
    }

    /// Convert to MIDI note number.
    ///
    /// Uses A4 = 440 Hz = MIDI note 69.
    #[inline]
    pub fn to_midi_note(self) -> f64 {
        69.0 + 12.0 * (self.0 / 440.0).log2()
    }

    /// Create from MIDI note number.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::core::FrequencyHz;
    ///
    /// let a4 = FrequencyHz::from_midi_note(69.0);
    /// assert!((a4.hz() - 440.0).abs() < 0.01);
    /// ```
    #[inline]
    pub fn from_midi_note(note: f64) -> Self {
        Self(440.0 * 2.0_f64.powf((note - 69.0) / 12.0))
    }

    /// Clamp frequency to be within the Nyquist limit.
    #[inline]
    pub fn clamp_nyquist(self, sample_rate_hz: f64) -> Self {
        let nyquist = sample_rate_hz * 0.5;
        Self(self.0.min(nyquist))
    }
}

impl fmt::Display for FrequencyHz {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 >= 1000.0 {
            write!(f, "{:.2} kHz", self.0 / 1000.0)
        } else {
            write!(f, "{:.2} Hz", self.0)
        }
    }
}

/// A time value in seconds.
///
/// # Example
///
/// ```rust
/// use dsp::core::TimeSeconds;
///
/// let time = TimeSeconds::new(0.5);
/// assert_eq!(time.seconds(), 0.5);
/// assert_eq!(time.ms(), 500.0);
/// ```
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct TimeSeconds(f64);

impl TimeSeconds {
    /// Create a time value in seconds.
    #[inline]
    pub const fn new(seconds: f64) -> Self {
        Self(seconds)
    }

    /// Create from milliseconds.
    #[inline]
    pub fn from_ms(ms: f64) -> Self {
        Self(ms * 0.001)
    }

    /// Get the time in seconds.
    #[inline]
    pub fn seconds(self) -> f64 {
        self.0
    }

    /// Get the time in milliseconds.
    #[inline]
    pub fn ms(self) -> f64 {
        self.0 * 1000.0
    }
}

impl fmt::Display for TimeSeconds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 >= 1.0 {
            write!(f, "{:.2} s", self.0)
        } else {
            write!(f, "{:.2} ms", self.0 * 1000.0)
        }
    }
}

/// A time value in samples.
///
/// This is useful for delays and other time-based effects
/// that work directly with sample counts.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeSamples(usize);

impl TimeSamples {
    /// Create a time value in samples.
    #[inline]
    pub const fn new(samples: usize) -> Self {
        Self(samples)
    }

    /// Get the number of samples.
    #[inline]
    pub fn samples(self) -> usize {
        self.0
    }
}

impl fmt::Display for TimeSamples {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} samples", self.0)
    }
}

/// A gain value in decibels.
///
/// # Example
///
/// ```rust
/// use dsp::core::Decibels;
///
/// let gain = Decibels::new(-6.0);
/// assert_eq!(gain.db(), -6.0);
/// assert!((gain.linear() - 0.501187).abs() < 0.00001);
/// ```
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Decibels(f32);

impl Decibels {
    /// Negative infinity in dB (used for silence).
    pub const NEG_INFINITY: Self = Self(f32::NEG_INFINITY);

    /// 0 dB (unity gain).
    pub const ZERO: Self = Self(0.0);

    /// Create a gain value in decibels.
    #[inline]
    pub const fn new(db: f32) -> Self {
        Self(db)
    }

    /// Get the value in decibels.
    #[inline]
    pub fn db(self) -> f32 {
        self.0
    }

    /// Convert to linear gain.
    ///
    /// Uses the formula: linear = 10^(dB/20)
    #[inline]
    pub fn linear(self) -> f32 {
        if self.0 == f32::NEG_INFINITY {
            0.0
        } else {
            10.0_f32.powf(self.0 / 20.0)
        }
    }

    /// Create from linear gain.
    ///
    /// Uses the formula: dB = 20 * log10(linear)
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::core::Decibels;
    ///
    /// let db = Decibels::from_linear(0.5);
    /// assert!((db.db() - (-6.0206)).abs() < 0.001);
    /// ```
    #[inline]
    pub fn from_linear(linear: f32) -> Self {
        if linear <= 0.0 {
            Self::NEG_INFINITY
        } else {
            Self(20.0 * linear.log10())
        }
    }

    /// Convert to linear gain as a specific sample type.
    #[inline]
    pub fn to_sample<T: Sample>(self) -> T {
        T::from_f32(self.linear())
    }
}

impl fmt::Display for Decibels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == f32::NEG_INFINITY {
            write!(f, "-∞ dB")
        } else {
            write!(f, "{:.2} dB", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalized_param() {
        let param = NormalizedParam::new(0.5).unwrap();
        assert_eq!(param.value(), 0.5);

        // Invalid values
        assert!(NormalizedParam::new(-0.1).is_none());
        assert!(NormalizedParam::new(1.1).is_none());

        // Linear mapping
        let mapped = param.map_linear(0.0, 100.0);
        assert_eq!(mapped, 50.0);

        // Logarithmic mapping
        let log_mapped = param.map_log(1.0, 100.0);
        assert!((log_mapped - 10.0).abs() < 0.01); // sqrt(100) = 10
    }

    #[test]
    fn test_frequency_hz() {
        let freq = FrequencyHz::new(440.0);
        assert_eq!(freq.hz(), 440.0);

        // MIDI note conversion
        let midi_note = freq.to_midi_note();
        assert!((midi_note - 69.0).abs() < 0.01);

        let from_midi = FrequencyHz::from_midi_note(69.0);
        assert!((from_midi.hz() - 440.0).abs() < 0.01);
    }

    #[test]
    fn test_time_seconds() {
        let time = TimeSeconds::new(0.5);
        assert_eq!(time.seconds(), 0.5);
        assert_eq!(time.ms(), 500.0);

        let from_ms = TimeSeconds::from_ms(500.0);
        assert_eq!(from_ms.seconds(), 0.5);
    }

    #[test]
    fn test_time_samples() {
        let time = TimeSamples::new(1000);
        assert_eq!(time.samples(), 1000);
    }

    #[test]
    fn test_decibels() {
        // 0 dB = unity gain
        let zero_db = Decibels::new(0.0);
        assert!((zero_db.linear() - 1.0).abs() < 0.00001);

        // -6 dB ≈ 0.5 linear
        let minus_six = Decibels::new(-6.0);
        assert!((minus_six.linear() - 0.501187).abs() < 0.00001);

        // +6 dB ≈ 2.0 linear
        let plus_six = Decibels::new(6.0);
        assert!((plus_six.linear() - 1.995262).abs() < 0.00001);

        // From linear
        let from_half = Decibels::from_linear(0.5);
        assert!((from_half.db() - (-6.0206)).abs() < 0.001);

        // Negative infinity
        assert_eq!(Decibels::NEG_INFINITY.linear(), 0.0);
        assert_eq!(Decibels::from_linear(0.0), Decibels::NEG_INFINITY);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", NormalizedParam::new(0.5).unwrap()), "50.00%");
        assert_eq!(format!("{}", FrequencyHz::new(440.0)), "440.00 Hz");
        assert_eq!(format!("{}", FrequencyHz::new(1500.0)), "1.50 kHz");
        assert_eq!(format!("{}", TimeSeconds::new(0.5)), "500.00 ms");
        assert_eq!(format!("{}", TimeSeconds::new(2.0)), "2.00 s");
        assert_eq!(format!("{}", TimeSamples::new(1000)), "1000 samples");
        assert_eq!(format!("{}", Decibels::new(-6.0)), "-6.00 dB");
        assert_eq!(format!("{}", Decibels::NEG_INFINITY), "-∞ dB");
    }
}
