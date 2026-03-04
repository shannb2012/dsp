//! DSP mathematics: interpolation, windows, conversions, fast approximations,
//! and parameter smoothing.
//!
//! This module provides building blocks used throughout the DSP pipeline:
//!
//! | Sub-module        | Contents                                             |
//! |-------------------|------------------------------------------------------|
//! | [`smoother`]      | `ParamSmoother` — click-free parameter automation    |
//! | [`interpolation`] | `lerp`, `cubic_interp`, `hermite_interp`             |
//! | [`window`]        | Hann, Hamming, Blackman window functions             |
//! | [`conversion`]    | Semitones, cents, BPM, MIDI ↔ Hz, phase utilities   |
//! | [`fast_math`]     | Fast `tanh`, `sin`, `cos`, `exp` approximations      |
//!
//! ## Quick reference
//!
//! ```rust
//! use dsp::math::{
//!     ParamSmoother,
//!     lerp, hermite_interp,
//!     fill_hann, apply_window,
//!     semitones_to_ratio, bpm_to_seconds, midi_to_freq,
//!     fast_tanh,
//! };
//! ```

mod smoother;
mod interpolation;
mod window;
mod conversion;
mod fast_math;

// --- Smoothing ---
pub use smoother::ParamSmoother;

// --- Interpolation ---
pub use interpolation::{lerp, cubic_interp, hermite_interp};

// --- Window functions ---
pub use window::{
    hann_coefficient,
    hamming_coefficient,
    blackman_coefficient,
    fill_hann,
    fill_hamming,
    fill_blackman,
    apply_window,
};

// --- Unit conversions ---
pub use conversion::{
    semitones_to_ratio,
    ratio_to_semitones,
    cents_to_ratio,
    ratio_to_cents,
    bpm_to_hz,
    bpm_to_seconds,
    hz_to_bpm,
    midi_to_freq,
    freq_to_midi,
    freq_to_phase_increment,
    freq_to_angular,
};

// --- Fast approximations ---
pub use fast_math::{
    fast_tanh,
    fast_sin,
    fast_cos,
    fast_exp,
    wrap_phase,
    wrap_phase_norm,
};
