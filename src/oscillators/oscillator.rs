//! Common oscillator trait.

use crate::core::Sample;

/// Common interface for all waveform generators.
///
/// All oscillators maintain internal phase in the normalized range [0, 1),
/// advancing by `frequency / sample_rate` each sample. This makes waveform
/// computation and phase-modulation straightforward.
///
/// ## Typical usage
///
/// ```rust
/// use dsp::oscillators::{Oscillator, SineOscillator};
///
/// let mut osc = SineOscillator::<f32>::new(440.0, 44100.0);
///
/// // In your process() callback, call once per sample:
/// let sample = osc.process();
/// assert!(sample >= -1.0 && sample <= 1.0);
/// ```
///
/// ## Plugin lifecycle
///
/// In a plugin context, the typical lifecycle is:
///
/// 1. **Construct** once (`new(freq, sample_rate)`).
/// 2. **`prepare()`** — call `set_sample_rate()` if the sample rate can change.
/// 3. **Note on** — call `reset()` to snap phase to 0.
/// 4. **`process()` loop** — call `process()` once per sample.
/// 5. **Automation** — call `set_frequency()` when a knob or MIDI pitch bend changes.
pub trait Oscillator<T: Sample> {
    /// Set the oscillator frequency in Hz.
    ///
    /// Recomputes the per-sample phase increment. Safe to call from the
    /// audio thread, but avoid doing so every sample — update at most once
    /// per block.
    fn set_frequency(&mut self, freq: f64);

    /// Set the sample rate in Hz and recompute the phase increment.
    ///
    /// Call this from `prepare()`, not from `process()`.
    fn set_sample_rate(&mut self, sample_rate: f64);

    /// Jump the phase to a specific value (normalized [0, 1)).
    ///
    /// Values outside [0, 1) are wrapped. Useful for hard sync
    /// (snapping a slave oscillator's phase to a master) or phase-modulation
    /// synthesis (FM).
    fn set_phase(&mut self, phase: T);

    /// Generate and return the next sample, advancing phase by one step.
    ///
    /// **Real-time safe**: one or two multiply-adds, no allocations.
    fn process(&mut self) -> T;

    /// Reset phase to 0.0.
    ///
    /// Call at note-on to avoid a phase offset left over from the previous
    /// note. Without this, the oscillator produces a click at note onset
    /// whose amplitude depends on where the phase happened to be.
    fn reset(&mut self);
}
