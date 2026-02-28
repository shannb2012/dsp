//! Core types and traits for the DSP library.
//!
//! This module defines the fundamental abstractions used throughout
//! the library, including the `Sample` trait, sample rate handling,
//! and parameter types.

mod sample;
mod sample_rate;
mod parameter;
mod constants;

pub use sample::Sample;
pub use sample_rate::SampleRate;
pub use parameter::{
    NormalizedParam,
    FrequencyHz,
    TimeSeconds,
    TimeSamples,
    Decibels,
};
pub use constants::*;
