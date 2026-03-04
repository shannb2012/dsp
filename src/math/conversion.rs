//! Musical unit conversions not covered by the core parameter types.
//!
//! The `core` module handles general audio types (`Decibels`, `FrequencyHz`,
//! `SampleRate`). This module adds the *musical* math that synthesizers and
//! effects need: pitch intervals (semitones, cents), tempo conversions (BPM),
//! and related utilities.
//!
//! All functions here work in `f64` for maximum precision. Convert to `f32`
//! at the call site if needed (e.g. with `Sample::from_f64`).

use std::f64::consts;

// ---------------------------------------------------------------------------
// Pitch interval conversions
// ---------------------------------------------------------------------------

/// Convert a number of semitones to a frequency ratio.
///
/// Twelve semitones equal one octave (ratio of 2.0). This is the standard
/// equal-tempered tuning system used in Western music.
///
/// ```text
/// ratio = 2^(semitones / 12)
/// ```
///
/// ## Common values
///
/// | Semitones | Ratio  | Interval        |
/// |-----------|--------|-----------------|
/// | 0         | 1.000  | Unison          |
/// | 1         | 1.059  | Minor 2nd       |
/// | 3         | 1.189  | Minor 3rd       |
/// | 7         | 1.498  | Perfect 5th     |
/// | 12        | 2.000  | Octave up       |
/// | −12       | 0.500  | Octave down     |
///
/// ## Use in synthesis
///
/// To pitch-shift a sound by N semitones, multiply its frequency by
/// `semitones_to_ratio(N)`:
///
/// ```rust
/// use dsp::math::semitones_to_ratio;
///
/// let a4 = 440.0_f64;
/// let a5 = a4 * semitones_to_ratio(12.0); // 880 Hz
/// assert!((a5 - 880.0).abs() < 0.001);
///
/// let a3 = a4 * semitones_to_ratio(-12.0); // 220 Hz
/// assert!((a3 - 220.0).abs() < 0.001);
/// ```
#[inline]
pub fn semitones_to_ratio(semitones: f64) -> f64 {
    2.0_f64.powf(semitones / 12.0)
}

/// Convert a frequency ratio to semitones.
///
/// Inverse of [`semitones_to_ratio`].
///
/// ```text
/// semitones = 12 × log₂(ratio)
/// ```
///
/// # Example
///
/// ```rust
/// use dsp::math::ratio_to_semitones;
///
/// // An octave (ratio 2.0) should equal exactly 12 semitones
/// assert!((ratio_to_semitones(2.0) - 12.0).abs() < 1e-10);
///
/// // Unison
/// assert!((ratio_to_semitones(1.0) - 0.0).abs() < 1e-10);
/// ```
#[inline]
pub fn ratio_to_semitones(ratio: f64) -> f64 {
    12.0 * ratio.log2()
}

/// Convert cents to a frequency ratio.
///
/// One cent is 1/100th of a semitone. Cents are used for fine-tuning
/// (detune knobs, vibrato depth) where full-semitone resolution is too coarse.
///
/// ```text
/// ratio = 2^(cents / 1200)
/// ```
///
/// ## Common uses
///
/// - **Chorus/detune**: ±15–50 cents makes copies sound "wider"
/// - **Vibrato depth**: ±25–100 cents (musically audible, not harsh)
/// - **Global tune**: ±100 cents (±1 semitone) for A440 vs A415 tuning
///
/// # Example
///
/// ```rust
/// use dsp::math::cents_to_ratio;
///
/// // 100 cents = 1 semitone
/// let ratio = cents_to_ratio(100.0);
/// assert!((ratio - 2.0_f64.powf(1.0 / 12.0)).abs() < 1e-12);
///
/// // 1200 cents = 1 octave
/// assert!((cents_to_ratio(1200.0) - 2.0).abs() < 1e-10);
/// ```
#[inline]
pub fn cents_to_ratio(cents: f64) -> f64 {
    2.0_f64.powf(cents / 1200.0)
}

/// Convert a frequency ratio to cents.
///
/// Inverse of [`cents_to_ratio`].
///
/// # Example
///
/// ```rust
/// use dsp::math::ratio_to_cents;
///
/// // One octave = 1200 cents
/// assert!((ratio_to_cents(2.0) - 1200.0).abs() < 1e-9);
/// ```
#[inline]
pub fn ratio_to_cents(ratio: f64) -> f64 {
    1200.0 * ratio.log2()
}

// ---------------------------------------------------------------------------
// Tempo / rhythm conversions
// ---------------------------------------------------------------------------

/// Convert a tempo in BPM to a frequency in Hz.
///
/// A quarter-note at 120 BPM occurs 2 times per second, so its frequency
/// is 2 Hz.
///
/// ```text
/// hz = bpm / 60
/// ```
///
/// Useful for syncing LFOs or delay times to the host tempo.
///
/// # Example
///
/// ```rust
/// use dsp::math::bpm_to_hz;
///
/// assert!((bpm_to_hz(120.0) - 2.0).abs() < 1e-12);
/// assert!((bpm_to_hz(60.0)  - 1.0).abs() < 1e-12);
/// ```
#[inline]
pub fn bpm_to_hz(bpm: f64) -> f64 {
    bpm / 60.0
}

/// Convert a tempo in BPM to a quarter-note period in seconds.
///
/// ```text
/// seconds = 60 / bpm
/// ```
///
/// This is the delay time for a single quarter-note. For other note values:
/// - Half note: `bpm_to_seconds(bpm) * 2.0`
/// - Eighth note: `bpm_to_seconds(bpm) / 2.0`
/// - Dotted quarter: `bpm_to_seconds(bpm) * 1.5`
///
/// # Example
///
/// ```rust
/// use dsp::math::bpm_to_seconds;
///
/// assert!((bpm_to_seconds(120.0) - 0.5).abs() < 1e-12);  // 0.5s per beat
/// assert!((bpm_to_seconds(60.0)  - 1.0).abs() < 1e-12);  // 1.0s per beat
/// ```
#[inline]
pub fn bpm_to_seconds(bpm: f64) -> f64 {
    60.0 / bpm
}

/// Convert a frequency in Hz to BPM.
///
/// Inverse of [`bpm_to_hz`]. Useful when you know the clock frequency and
/// want to express it as a tempo.
///
/// # Example
///
/// ```rust
/// use dsp::math::hz_to_bpm;
///
/// assert!((hz_to_bpm(2.0) - 120.0).abs() < 1e-12);
/// ```
#[inline]
pub fn hz_to_bpm(hz: f64) -> f64 {
    hz * 60.0
}

// ---------------------------------------------------------------------------
// Frequency / MIDI
// ---------------------------------------------------------------------------

/// Convert a MIDI note number to frequency in Hz.
///
/// Uses the standard equal-temperament formula with A4 = 440 Hz (MIDI note 69):
///
/// ```text
/// freq = 440 × 2^((note − 69) / 12)
/// ```
///
/// This is a standalone function version of `FrequencyHz::from_midi_note`.
/// Useful when you don't need the `FrequencyHz` wrapper.
///
/// # Example
///
/// ```rust
/// use dsp::math::midi_to_freq;
///
/// assert!((midi_to_freq(69.0) - 440.0).abs() < 0.001); // A4
/// assert!((midi_to_freq(60.0) - 261.626).abs() < 0.01); // Middle C
/// assert!((midi_to_freq(57.0) - 220.0).abs() < 0.01);  // A3
/// ```
#[inline]
pub fn midi_to_freq(midi_note: f64) -> f64 {
    // 69 is the MIDI number for A4 (440 Hz)
    440.0 * 2.0_f64.powf((midi_note - 69.0) / 12.0)
}

/// Convert a frequency in Hz to a (possibly fractional) MIDI note number.
///
/// Inverse of [`midi_to_freq`].
///
/// ```text
/// note = 69 + 12 × log₂(freq / 440)
/// ```
///
/// The result can be fractional — for example, 432 Hz gives approximately
/// MIDI note 68.76.
///
/// # Example
///
/// ```rust
/// use dsp::math::freq_to_midi;
///
/// assert!((freq_to_midi(440.0) - 69.0).abs() < 1e-10); // A4 = MIDI 69
/// assert!((freq_to_midi(220.0) - 57.0).abs() < 1e-10); // A3 = MIDI 57
/// ```
#[inline]
pub fn freq_to_midi(freq_hz: f64) -> f64 {
    69.0 + 12.0 * (freq_hz / 440.0).log2()
}

// ---------------------------------------------------------------------------
// Phase / angle utilities
// ---------------------------------------------------------------------------

/// Convert a frequency in Hz and a sample rate to a per-sample phase increment.
///
/// The phase increment is normalised to `[0, 1)` per cycle — the same
/// convention used by our oscillators. A phase of `1.0` represents one
/// complete revolution.
///
/// ```text
/// increment = freq_hz / sample_rate
/// ```
///
/// This is a standalone version of `SampleRate::freq_to_phase_increment`.
///
/// # Example
///
/// ```rust
/// use dsp::math::freq_to_phase_increment;
///
/// let inc = freq_to_phase_increment(440.0, 44100.0);
/// assert!((inc - 440.0 / 44100.0).abs() < 1e-12);
/// ```
#[inline]
pub fn freq_to_phase_increment(freq_hz: f64, sample_rate: f64) -> f64 {
    freq_hz / sample_rate
}

/// Convert a frequency in Hz to angular frequency (radians per sample).
///
/// Used in filter coefficient calculations where the bilinear transform
/// requires the pre-warped angular frequency ω = 2π × f / fs.
///
/// # Example
///
/// ```rust
/// use dsp::math::freq_to_angular;
///
/// let omega = freq_to_angular(440.0, 44100.0);
/// assert!((omega - 2.0 * std::f64::consts::PI * 440.0 / 44100.0).abs() < 1e-12);
/// ```
#[inline]
pub fn freq_to_angular(freq_hz: f64, sample_rate: f64) -> f64 {
    consts::TAU * freq_hz / sample_rate
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Semitones / cents
    // -----------------------------------------------------------------------

    #[test]
    fn test_semitones_to_ratio_octave() {
        assert!((semitones_to_ratio(12.0) - 2.0).abs() < 1e-12);
        assert!((semitones_to_ratio(-12.0) - 0.5).abs() < 1e-12);
        assert!((semitones_to_ratio(0.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_semitones_roundtrip() {
        for s in [-24.0f64, -12.0, -7.0, -1.0, 0.0, 1.0, 7.0, 12.0, 24.0] {
            let ratio = semitones_to_ratio(s);
            let back = ratio_to_semitones(ratio);
            assert!((back - s).abs() < 1e-10, "Roundtrip failed for {} semitones", s);
        }
    }

    #[test]
    fn test_cents_to_ratio() {
        // 1200 cents = 1 octave
        assert!((cents_to_ratio(1200.0) - 2.0).abs() < 1e-10);
        // 0 cents = unison
        assert!((cents_to_ratio(0.0) - 1.0).abs() < 1e-12);
        // 100 cents = 1 semitone
        assert!((cents_to_ratio(100.0) - semitones_to_ratio(1.0)).abs() < 1e-12);
    }

    #[test]
    fn test_cents_roundtrip() {
        for c in [-1200.0f64, -100.0, -1.0, 0.0, 1.0, 50.0, 100.0, 1200.0] {
            let ratio = cents_to_ratio(c);
            let back = ratio_to_cents(ratio);
            assert!((back - c).abs() < 1e-9, "Cents roundtrip failed for {}", c);
        }
    }

    // -----------------------------------------------------------------------
    // BPM
    // -----------------------------------------------------------------------

    #[test]
    fn test_bpm_to_hz() {
        assert!((bpm_to_hz(60.0) - 1.0).abs() < 1e-12);
        assert!((bpm_to_hz(120.0) - 2.0).abs() < 1e-12);
        assert!((bpm_to_hz(240.0) - 4.0).abs() < 1e-12);
    }

    #[test]
    fn test_bpm_to_seconds() {
        assert!((bpm_to_seconds(60.0) - 1.0).abs() < 1e-12);
        assert!((bpm_to_seconds(120.0) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn test_bpm_hz_roundtrip() {
        for bpm in [60.0f64, 90.0, 120.0, 140.0, 200.0] {
            let hz = bpm_to_hz(bpm);
            let back = hz_to_bpm(hz);
            assert!((back - bpm).abs() < 1e-10, "BPM roundtrip failed for {}", bpm);
        }
    }

    // -----------------------------------------------------------------------
    // MIDI
    // -----------------------------------------------------------------------

    #[test]
    fn test_midi_a4() {
        assert!((midi_to_freq(69.0) - 440.0).abs() < 1e-10);
    }

    #[test]
    fn test_midi_middle_c() {
        // C4 (MIDI 60) = 261.626 Hz
        assert!((midi_to_freq(60.0) - 261.626).abs() < 0.001);
    }

    #[test]
    fn test_midi_octave_relationship() {
        // MIDI 57 (A3) should be half of MIDI 69 (A4)
        assert!((midi_to_freq(57.0) - 220.0).abs() < 0.001);
        // MIDI 81 (A5) should be double of MIDI 69 (A4)
        assert!((midi_to_freq(81.0) - 880.0).abs() < 0.001);
    }

    #[test]
    fn test_midi_freq_roundtrip() {
        for note in [21.0f64, 48.0, 60.0, 69.0, 84.0, 108.0] {
            let freq = midi_to_freq(note);
            let back = freq_to_midi(freq);
            assert!((back - note).abs() < 1e-10,
                "MIDI roundtrip failed for note {}", note);
        }
    }

    // -----------------------------------------------------------------------
    // Phase / angular
    // -----------------------------------------------------------------------

    #[test]
    fn test_phase_increment() {
        let inc = freq_to_phase_increment(440.0, 44100.0);
        assert!((inc - 440.0 / 44100.0).abs() < 1e-15);
    }

    #[test]
    fn test_freq_to_angular() {
        let omega = freq_to_angular(440.0, 44100.0);
        let expected = std::f64::consts::TAU * 440.0 / 44100.0;
        assert!((omega - expected).abs() < 1e-15);
    }
}
