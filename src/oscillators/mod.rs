//! Waveform generators: sine, sawtooth, square, triangle, and noise.
//!
//! All oscillators share the [`Oscillator`] trait for a uniform API. Phase is
//! always stored internally as `f64` to prevent accumulation error during long
//! runs, even when the output type `T` is `f32`.
//!
//! ## Oscillator summary
//!
//! | Type                    | Waveform          | Band-limited? | Notes                    |
//! |-------------------------|-------------------|---------------|--------------------------|
//! | [`SineOscillator`]      | Sine              | Inherently    | Phase accumulation + sin |
//! | [`SawOscillator`]       | Sawtooth          | Optional      | PolyBLEP anti-aliasing   |
//! | [`SquareOscillator`]    | Square / Pulse    | Optional      | Variable pulse width     |
//! | [`TriangleOscillator`]  | Triangle          | Naive         | Harmonics decay as 1/n²  |
//! | [`NoiseGenerator`]      | White/Pink/Brown  | N/A           | XOR-shift PRNG           |
//!
//! ## Quick example
//!
//! ```rust
//! use dsp::oscillators::{Oscillator, SineOscillator, SawOscillator,
//!                         SquareOscillator, TriangleOscillator,
//!                         NoiseGenerator, NoiseColor};
//!
//! let sample_rate = 44100.0;
//!
//! let mut sine  = SineOscillator::<f32>::new(440.0, sample_rate);
//! let mut saw   = SawOscillator::<f32>::new(220.0, sample_rate);
//! let mut sq    = SquareOscillator::<f32>::new(110.0, sample_rate);
//! let mut tri   = TriangleOscillator::<f32>::new(55.0, sample_rate);
//! let mut noise = NoiseGenerator::new(NoiseColor::Pink, 1234);
//!
//! // One sample of each
//! let _s = sine.process();
//! let _s = saw.process();
//! let _s = sq.process();
//! let _s = tri.process();
//! let _s = noise.process();
//! ```

mod oscillator;
mod sine;
mod saw;
mod square;
mod triangle;
mod noise;

// --- Trait ---
pub use oscillator::Oscillator;

// --- Oscillators ---
pub use sine::SineOscillator;
pub use saw::SawOscillator;
pub use square::SquareOscillator;
pub use triangle::TriangleOscillator;

// --- Noise ---
pub use noise::{NoiseGenerator, NoiseColor};
