//! Common DSP constants and utility values.

/// Minimum non-zero value to prevent denormals.
///
/// Adding this small value to filter states and feedback paths
/// can prevent denormal numbers from causing performance issues.
pub const DENORMAL_OFFSET: f32 = 1e-25;

/// Minimum audible frequency (Hz).
///
/// Frequencies below this are typically not perceived as pitch.
pub const MIN_AUDIBLE_FREQ: f64 = 20.0;

/// Maximum audible frequency (Hz).
///
/// Most humans cannot hear above this frequency, though it varies with age.
pub const MAX_AUDIBLE_FREQ: f64 = 20000.0;

/// MIDI note number for middle C (C4).
pub const MIDI_MIDDLE_C: f64 = 60.0;

/// MIDI note number for A4 (440 Hz reference pitch).
pub const MIDI_A4: f64 = 69.0;

/// Reference frequency for A4 in Hz.
pub const A4_FREQUENCY: f64 = 440.0;

/// Speed of sound in air at sea level (m/s).
///
/// Useful for calculating delay times for physical modeling.
pub const SPEED_OF_SOUND: f64 = 343.0;

/// Default Q value for biquad filters (provides a gentle resonance).
pub const DEFAULT_FILTER_Q: f64 = 0.707; // 1/sqrt(2), Butterworth response

/// Minimum Q value to prevent filter instability.
pub const MIN_FILTER_Q: f64 = 0.01;

/// Maximum Q value to prevent extreme resonance.
pub const MAX_FILTER_Q: f64 = 100.0;

/// Silence threshold in linear amplitude.
///
/// Values below this can be considered effectively silent.
pub const SILENCE_THRESHOLD: f32 = 1e-6;

/// Silence threshold in decibels.
pub const SILENCE_THRESHOLD_DB: f32 = -120.0;

/// Maximum safe gain in dB to prevent clipping in most scenarios.
pub const MAX_SAFE_GAIN_DB: f32 = 12.0;

/// Default smoothing time for parameter changes (in seconds).
///
/// This provides a smooth transition that prevents clicks while
/// still being responsive to user input.
pub const DEFAULT_SMOOTHING_TIME: f64 = 0.02; // 20ms

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(MIN_AUDIBLE_FREQ, 20.0);
        assert_eq!(MAX_AUDIBLE_FREQ, 20000.0);
        assert_eq!(MIDI_A4, 69.0);
        assert_eq!(A4_FREQUENCY, 440.0);
    }
}
