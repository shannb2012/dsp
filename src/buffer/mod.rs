//! Audio buffer types for real-time audio processing.
//!
//! This module provides two complementary buffer types:
//!
//! | Type                        | Owns memory? | Primary use                         |
//! |-----------------------------|--------------|-------------------------------------|
//! | [`AudioBuffer<'a, T>`]      | No           | Wrapping host-provided channel ptrs |
//! | [`OwnedAudioBuffer<T>`]     | Yes          | Scratch buffers, delay lines, etc.  |
//!
//! ## Choosing the right type
//!
//! **Use `AudioBuffer`** when a plugin host calls your `process()` callback
//! and hands you raw pointers to audio data. You wrap those pointers in an
//! `AudioBuffer` for zero-copy, safe processing.
//!
//! **Use `OwnedAudioBuffer`** for any internal buffer you need to allocate
//! yourself — scratch space, feedback delay lines, synthesis output, etc.
//! Allocate in `prepare()` (before the audio thread starts), then borrow
//! it as an `AudioBuffer` view each block via [`OwnedAudioBuffer::as_audio_buffer`].
//!
//! ## Example: plugin-style processing
//!
//! ```rust
//! use dsp::buffer::{AudioBuffer, OwnedAudioBuffer};
//!
//! struct MyPlugin {
//!     scratch: OwnedAudioBuffer<f32>,
//! }
//!
//! impl MyPlugin {
//!     fn prepare(&mut self, block_size: usize) {
//!         // Allocate once before audio starts — this is fine!
//!         self.scratch = OwnedAudioBuffer::new(2, block_size);
//!     }
//!
//!     fn process(&mut self, output: &mut AudioBuffer<f32>) {
//!         // No allocations here — real-time safe.
//!         self.scratch.clear();
//!         let mut work = self.scratch.as_audio_buffer();
//!
//!         // ... fill `work` with synthesised audio ...
//!
//!         output.mix_from(&work, 1.0f32);
//!     }
//! }
//! ```

mod audio_buffer;

pub use audio_buffer::{AudioBuffer, BufferError, OwnedAudioBuffer};
