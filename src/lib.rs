//! # Rust DSP Library
//!
//! A modern, type-safe DSP library for real-time audio processing.
//!
//! ## Features
//!
//! - Generic over sample types (f32/f64)
//! - Zero-cost abstractions
//! - Real-time safe (no allocations in hot paths)
//! - Comprehensive DSP primitives
//!
//! ## Example
//!
//! ```rust
//! use dsp::core::{Sample, SampleRate};
//!
//! let sample_rate = SampleRate::new(44100.0).unwrap();
//! let freq = 440.0; // A4
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod core;
pub mod buffer;
pub mod math;
pub mod oscillators;

// Re-export commonly used types
pub use core::{Sample, SampleRate};
