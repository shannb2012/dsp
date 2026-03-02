//! Core audio buffer types: borrowed view and owned storage.

use crate::core::Sample;
use std::fmt;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur when constructing or using audio buffers.
#[derive(Debug, Clone, PartialEq)]
pub enum BufferError {
    /// All channels must have the same number of samples.
    ///
    /// The plugin audio model requires every channel in a single buffer to
    /// be exactly the same length (the block/buffer size chosen by the host).
    ChannelLengthMismatch {
        /// The length of the first channel, which is used as the reference.
        expected: usize,
        /// The actual length of the offending channel.
        got: usize,
        /// Zero-based index of the channel with the wrong length.
        channel_index: usize,
    },

    /// Buffer must have at least one channel.
    ///
    /// A buffer with zero channels has no audio data and cannot be processed.
    NoChannels,

    /// Buffer must have at least one sample per channel.
    ///
    /// A block size of zero is not meaningful for audio processing.
    NoSamples,

    /// Channel index is out of bounds.
    ChannelIndexOutOfBounds {
        /// The requested channel index.
        index: usize,
        /// The actual number of channels in the buffer.
        num_channels: usize,
    },
}

impl fmt::Display for BufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChannelLengthMismatch { expected, got, channel_index } => write!(
                f,
                "Channel {} has {} samples but expected {} (matching channel 0)",
                channel_index, got, expected
            ),
            Self::NoChannels => write!(f, "Buffer must have at least one channel"),
            Self::NoSamples => write!(f, "Buffer must have at least one sample per channel"),
            Self::ChannelIndexOutOfBounds { index, num_channels } => write!(
                f,
                "Channel index {} is out of bounds (buffer has {} channels)",
                index, num_channels
            ),
        }
    }
}

impl std::error::Error for BufferError {}

// ---------------------------------------------------------------------------
// AudioBuffer — non-owning view
// ---------------------------------------------------------------------------

/// A non-owning, multi-channel audio buffer view.
///
/// `AudioBuffer` holds mutable references to externally-allocated channel
/// slices. It does **not** own the memory — it is a lightweight view that
/// enables zero-copy audio processing.
///
/// ## Why non-owning?
///
/// In real audio plugin development, the *plugin host* (the DAW) allocates
/// and manages audio buffers. When the host calls your plugin's `process()`
/// callback, it hands you raw pointers to those buffers. `AudioBuffer` wraps
/// those pointers in a safe, ergonomic Rust type without copying any data.
///
/// ## Memory layout
///
/// Audio data is stored in **non-interleaved** (planar) format: each channel
/// is a separate, contiguous slice.
///
/// ```text
/// Channel 0: [L0, L1, L2, ..., L511]
/// Channel 1: [R0, R1, R2, ..., R511]
/// ```
///
/// This layout is cache-friendly for per-channel operations (filtering,
/// panning, etc.) and is standard for VST3, AU, and CLAP plugin formats.
/// It is the **opposite** of interleaved format (`L0, R0, L1, R1, ...`)
/// used by some hardware and file formats.
///
/// ## Example
///
/// ```rust
/// use dsp::buffer::AudioBuffer;
///
/// let mut left  = vec![1.0f32; 512];
/// let mut right = vec![1.0f32; 512];
///
/// let mut buf = AudioBuffer::from_slices(vec![
///     left.as_mut_slice(),
///     right.as_mut_slice(),
/// ]).unwrap();
///
/// // Reduce gain by 6 dB (multiply by ≈0.501)
/// buf.apply_gain(0.501f32);
/// assert!(buf.channel(0)[0] < 1.0);
/// ```
// Debug is implemented manually because `Vec<&'a mut [T]>: Debug` requires
// `T: Debug`, which is already satisfied by the `Sample: Debug` bound.
#[derive(Debug)]
pub struct AudioBuffer<'a, T: Sample> {
    /// One `&'a mut [T]` per channel.
    ///
    /// The lifetime `'a` is tied to the original data source (e.g., host
    /// buffers or an `OwnedAudioBuffer`). Every slice is guaranteed — at
    /// construction time — to have the same length.
    channels: Vec<&'a mut [T]>,
}

impl<'a, T: Sample> AudioBuffer<'a, T> {
    /// Construct an `AudioBuffer` from a collection of channel slices.
    ///
    /// Validation guarantees:
    /// * At least one channel must be provided.
    /// * Every channel must have the same, non-zero length.
    ///
    /// # Errors
    ///
    /// Returns `BufferError::NoChannels` when `channels` is empty.
    /// Returns `BufferError::NoSamples` when the first channel is empty.
    /// Returns `BufferError::ChannelLengthMismatch` when channels differ in length.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::buffer::AudioBuffer;
    ///
    /// let mut ch0 = vec![0.0f32; 256];
    /// let mut ch1 = vec![0.0f32; 256];
    ///
    /// let buf = AudioBuffer::from_slices(vec![
    ///     ch0.as_mut_slice(),
    ///     ch1.as_mut_slice(),
    /// ]).unwrap();
    ///
    /// assert_eq!(buf.num_channels(), 2);
    /// assert_eq!(buf.num_samples(), 256);
    /// ```
    pub fn from_slices(channels: Vec<&'a mut [T]>) -> Result<Self, BufferError> {
        if channels.is_empty() {
            return Err(BufferError::NoChannels);
        }

        let expected_len = channels[0].len();

        if expected_len == 0 {
            return Err(BufferError::NoSamples);
        }

        // Validate that every subsequent channel matches the reference length.
        // We skip channel 0 because it defines `expected_len`.
        for (i, ch) in channels.iter().enumerate().skip(1) {
            if ch.len() != expected_len {
                return Err(BufferError::ChannelLengthMismatch {
                    expected: expected_len,
                    got: ch.len(),
                    channel_index: i,
                });
            }
        }

        Ok(Self { channels })
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Number of audio channels.
    ///
    /// Typical values: `1` (mono), `2` (stereo), `6` (5.1 surround).
    #[inline]
    pub fn num_channels(&self) -> usize {
        self.channels.len()
    }

    /// Number of samples per channel (the block/buffer size).
    ///
    /// Audio is processed in fixed-size blocks. Common block sizes are
    /// 64, 128, 256, 512, and 1024 samples. Smaller blocks give lower
    /// latency; larger blocks use CPU more efficiently.
    #[inline]
    pub fn num_samples(&self) -> usize {
        // All channels have the same length — validated at construction.
        // `first()` is always `Some` because `from_slices` requires ≥1 channel.
        self.channels.first().map_or(0, |ch| ch.len())
    }

    /// Returns `true` if the buffer has exactly one channel.
    #[inline]
    pub fn is_mono(&self) -> bool {
        self.channels.len() == 1
    }

    /// Returns `true` if the buffer has exactly two channels.
    #[inline]
    pub fn is_stereo(&self) -> bool {
        self.channels.len() == 2
    }

    /// Get an immutable reference to a channel's sample data.
    ///
    /// # Panics
    ///
    /// Panics if `index >= num_channels()`.
    #[inline]
    pub fn channel(&self, index: usize) -> &[T] {
        // The `&'a mut [T]` stored in the Vec coerces to `&[T]` via deref.
        // Rust's borrow rules guarantee this shared borrow doesn't alias any
        // concurrent mutable borrow.
        &*self.channels[index]
    }

    /// Get a mutable reference to a channel's sample data.
    ///
    /// # Panics
    ///
    /// Panics if `index >= num_channels()`.
    ///
    /// # Borrow note
    ///
    /// This reborrow ties the returned `&mut [T]` to `&mut self`, not to
    /// the original `'a` lifetime. That prevents you from holding two
    /// `channel_mut()` references simultaneously — use `iter_channels_mut()`
    /// if you need to process all channels at once.
    #[inline]
    pub fn channel_mut(&mut self, index: usize) -> &mut [T] {
        // `self.channels[index]` is `&'a mut [T]`.
        // `&mut *self.channels[index]` reborrow through it to get `&mut [T]`
        // tied to the lifetime of `&mut self`. This is safe: the borrow
        // checker prevents aliasing by requiring exclusive access to `self`.
        &mut *self.channels[index]
    }

    /// Iterate over all channels as immutable slices.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::buffer::AudioBuffer;
    ///
    /// let mut ch0 = vec![1.0f32; 4];
    /// let mut ch1 = vec![2.0f32; 4];
    /// let buf = AudioBuffer::from_slices(vec![ch0.as_mut_slice(), ch1.as_mut_slice()]).unwrap();
    ///
    /// for (i, ch) in buf.iter_channels().enumerate() {
    ///     println!("channel {}: {:?}", i, ch);
    /// }
    /// ```
    pub fn iter_channels(&self) -> impl Iterator<Item = &[T]> {
        // `ch` is `&&'a mut [T]`; double-deref yields `&[T]`.
        self.channels.iter().map(|ch| &**ch)
    }

    /// Iterate over all channels as mutable slices.
    ///
    /// Unlike `channel_mut()`, this yields all channels at once without
    /// conflicting borrows — because the iterator holds `&mut self` for
    /// its full lifetime.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::buffer::AudioBuffer;
    ///
    /// let mut ch0 = vec![1.0f32; 4];
    /// let mut ch1 = vec![1.0f32; 4];
    /// let mut buf = AudioBuffer::from_slices(vec![ch0.as_mut_slice(), ch1.as_mut_slice()]).unwrap();
    ///
    /// // Double every sample
    /// for ch in buf.iter_channels_mut() {
    ///     for sample in ch.iter_mut() {
    ///         *sample *= 2.0;
    ///     }
    /// }
    /// ```
    pub fn iter_channels_mut(&mut self) -> impl Iterator<Item = &mut [T]> {
        // `ch` is `&mut &'a mut [T]`.
        // `&mut **ch`: first `*ch` gives `&'a mut [T]`, then `**ch` gives `[T]`,
        // then `&mut` re-borrows mutably. The result lives for `'self` (the
        // mutable borrow of `self`), which is shorter than `'a`, preserving safety.
        self.channels.iter_mut().map(|ch| &mut **ch)
    }

    // -----------------------------------------------------------------------
    // In-place processing utilities
    // -----------------------------------------------------------------------

    /// Fill all channels with silence (zero out every sample).
    ///
    /// Typical usage: clear output buffers at the start of a `process()` block
    /// before accumulating results into them.
    ///
    /// This is **real-time safe**: no allocations, O(channels × samples) time.
    pub fn clear(&mut self) {
        for channel in self.channels.iter_mut() {
            channel.fill(T::ZERO);
        }
    }

    /// Scale every sample by a linear gain factor.
    ///
    /// | `gain` | Effect                   |
    /// |--------|--------------------------|
    /// | `1.0`  | Unity — unchanged        |
    /// | `0.0`  | Silence                  |
    /// | `0.5`  | -6 dB (half amplitude)   |
    /// | `2.0`  | +6 dB (double amplitude) |
    ///
    /// For smooth, click-free gain changes over a block, use the math
    /// module's `ParamSmoother` to ramp the gain sample-by-sample.
    ///
    /// This is **real-time safe**: no allocations.
    pub fn apply_gain(&mut self, gain: T) {
        for channel in self.channels.iter_mut() {
            for sample in channel.iter_mut() {
                *sample = *sample * gain;
            }
        }
    }

    /// Copy audio data from `other` into `self`, replacing existing content.
    ///
    /// Both buffers must have the same number of channels and samples.
    ///
    /// # Panics
    ///
    /// Panics on channel count or sample count mismatch.
    pub fn copy_from(&mut self, other: &AudioBuffer<'_, T>) {
        assert_eq!(
            self.num_channels(),
            other.num_channels(),
            "copy_from: channel count mismatch ({} vs {})",
            self.num_channels(),
            other.num_channels()
        );
        assert_eq!(
            self.num_samples(),
            other.num_samples(),
            "copy_from: sample count mismatch ({} vs {})",
            self.num_samples(),
            other.num_samples()
        );

        for (dst, src) in self.channels.iter_mut().zip(other.channels.iter()) {
            // `copy_from_slice` uses `memcpy` internally — very fast.
            dst.copy_from_slice(src);
        }
    }

    /// Add `other` into `self`, scaled by `gain`.
    ///
    /// Per-sample operation: `self[ch][n] += other[ch][n] * gain`
    ///
    /// This is the fundamental *mixing* operation. With `gain = 1.0`, it
    /// sums two signals. With `gain = 0.5`, the incoming signal is at -6 dB.
    ///
    /// Both buffers must have the same number of channels and samples.
    ///
    /// # Panics
    ///
    /// Panics on channel count or sample count mismatch.
    pub fn mix_from(&mut self, other: &AudioBuffer<'_, T>, gain: T) {
        assert_eq!(
            self.num_channels(),
            other.num_channels(),
            "mix_from: channel count mismatch ({} vs {})",
            self.num_channels(),
            other.num_channels()
        );
        assert_eq!(
            self.num_samples(),
            other.num_samples(),
            "mix_from: sample count mismatch ({} vs {})",
            self.num_samples(),
            other.num_samples()
        );

        for (dst_ch, src_ch) in self.channels.iter_mut().zip(other.channels.iter()) {
            for (dst, &src) in dst_ch.iter_mut().zip(src_ch.iter()) {
                // Fused multiply-add: dst += src * gain
                *dst = *dst + (src * gain);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Analysis utilities
    // -----------------------------------------------------------------------

    /// Find the peak (maximum absolute sample value) across all channels.
    ///
    /// Useful for metering, normalization, and gain staging.
    ///
    /// Returns `T::ZERO` if the buffer contains no samples (should not
    /// happen after construction, but is safe).
    pub fn peak(&self) -> T {
        let mut max = T::ZERO;
        for channel in self.channels.iter() {
            for &sample in channel.iter() {
                let abs = sample.abs();
                if abs > max {
                    max = abs;
                }
            }
        }
        max
    }

    /// Returns `true` if the buffer is effectively silent.
    ///
    /// A buffer is considered silent when its peak value is strictly below
    /// `threshold`. This is useful for CPU optimization — many DSP algorithms
    /// can be bypassed entirely when the input is silent.
    ///
    /// A common threshold is `1e-6` (roughly -120 dBFS), available as
    /// `dsp::core::SILENCE_THRESHOLD`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::buffer::AudioBuffer;
    /// use dsp::core::SILENCE_THRESHOLD;
    ///
    /// let mut ch0 = vec![0.0f32; 64];
    /// let mut buf = AudioBuffer::from_slices(vec![ch0.as_mut_slice()]).unwrap();
    /// assert!(buf.is_silent(SILENCE_THRESHOLD));
    /// ```
    pub fn is_silent(&self, threshold: T) -> bool {
        self.peak() < threshold
    }
}

// ---------------------------------------------------------------------------
// OwnedAudioBuffer — heap-allocated storage
// ---------------------------------------------------------------------------

/// A heap-allocated, multi-channel audio buffer.
///
/// Unlike `AudioBuffer<'a, T>`, which borrows externally-managed memory,
/// `OwnedAudioBuffer` allocates and owns its data. Common uses:
///
/// * **Scratch/work buffers** — temporary storage for intermediate DSP results.
/// * **Delay lines** — a long ring buffer for echo and reverb effects.
/// * **Synthesis output** — when there is no host buffer to borrow.
///
/// ## Memory layout
///
/// Samples are stored flat in a single `Vec<T>` in non-interleaved order:
///
/// ```text
/// [ch0[0], ch0[1], ..., ch0[N-1], ch1[0], ch1[1], ..., ch1[N-1], ...]
/// ```
///
/// This contiguous layout is cache-friendly and avoids pointer chasing.
///
/// ## Real-time usage pattern
///
/// ```text
/// fn prepare(&mut self, sample_rate: SampleRate, max_block: usize) {
///     // Allocate here — this is fine, we are NOT on the audio thread yet.
///     self.scratch = OwnedAudioBuffer::new(2, max_block);
/// }
///
/// fn process(&mut self, output: &mut AudioBuffer<f32>) {
///     // Use the pre-allocated buffer — zero allocations.
///     self.scratch.clear();
///     let view = self.scratch.as_audio_buffer();
///     // ... fill view, then mix into output ...
/// }
/// ```
///
/// ## Example
///
/// ```rust
/// use dsp::buffer::OwnedAudioBuffer;
///
/// let mut buf = OwnedAudioBuffer::<f32>::new(2, 512);
/// assert_eq!(buf.num_channels(), 2);
/// assert_eq!(buf.num_samples(),  512);
///
/// // Borrow as an AudioBuffer for processing
/// let view = buf.as_audio_buffer();
/// assert_eq!(view.num_channels(), 2);
/// ```
pub struct OwnedAudioBuffer<T: Sample> {
    /// Flat storage for all audio samples in non-interleaved order.
    ///
    /// Conceptually a 2-D array `data[channel][sample]`, linearized as:
    /// `data[channel * num_samples + sample]`.
    data: Vec<T>,
    num_channels: usize,
    num_samples: usize,
}

impl<T: Sample> OwnedAudioBuffer<T> {
    /// Allocate a zeroed buffer with the given channel count and block size.
    ///
    /// This is an **allocating** operation. Call it from `prepare()` or
    /// similar setup code, never from a real-time `process()` callback.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::buffer::OwnedAudioBuffer;
    ///
    /// // Stereo, 512-sample scratch buffer
    /// let buf = OwnedAudioBuffer::<f32>::new(2, 512);
    /// assert_eq!(buf.num_channels(), 2);
    /// assert_eq!(buf.num_samples(),  512);
    /// ```
    pub fn new(num_channels: usize, num_samples: usize) -> Self {
        // Total number of scalar values: channels × samples per channel.
        let total = num_channels.saturating_mul(num_samples);
        Self {
            data: vec![T::ZERO; total],
            num_channels,
            num_samples,
        }
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Number of audio channels.
    #[inline]
    pub fn num_channels(&self) -> usize {
        self.num_channels
    }

    /// Number of samples per channel.
    #[inline]
    pub fn num_samples(&self) -> usize {
        self.num_samples
    }

    /// Get an immutable slice for a single channel.
    ///
    /// # Panics
    ///
    /// Panics if `index >= num_channels()`.
    #[inline]
    pub fn channel(&self, index: usize) -> &[T] {
        assert!(
            index < self.num_channels,
            "channel index {} out of bounds (num_channels = {})",
            index,
            self.num_channels
        );
        let start = index * self.num_samples;
        &self.data[start..start + self.num_samples]
    }

    /// Get a mutable slice for a single channel.
    ///
    /// # Panics
    ///
    /// Panics if `index >= num_channels()`.
    #[inline]
    pub fn channel_mut(&mut self, index: usize) -> &mut [T] {
        assert!(
            index < self.num_channels,
            "channel index {} out of bounds (num_channels = {})",
            index,
            self.num_channels
        );
        let start = index * self.num_samples;
        let end = start + self.num_samples;
        &mut self.data[start..end]
    }

    // -----------------------------------------------------------------------
    // Operations
    // -----------------------------------------------------------------------

    /// Fill every sample with zero (silence the buffer).
    ///
    /// This is **real-time safe** once the buffer is allocated — it simply
    /// calls `fill(T::ZERO)` on the internal Vec's slice.
    pub fn clear(&mut self) {
        self.data.fill(T::ZERO);
    }

    /// Create a non-owning `AudioBuffer<'_, T>` view of this buffer.
    ///
    /// The view gives access to the full `AudioBuffer` API (mixing, gain
    /// application, analysis, etc.) without any data copies. The view is
    /// valid for the duration of the mutable borrow of `self`.
    ///
    /// # Panics
    ///
    /// Panics if `num_channels == 0` or `num_samples == 0`. Buffers
    /// with zero dimensions cannot be represented as `AudioBuffer`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::buffer::OwnedAudioBuffer;
    ///
    /// let mut owned = OwnedAudioBuffer::<f32>::new(2, 512);
    /// {
    ///     let mut view = owned.as_audio_buffer();
    ///     view.apply_gain(0.5f32);
    /// }
    /// // `owned` is accessible again here
    /// ```
    pub fn as_audio_buffer(&mut self) -> AudioBuffer<'_, T> {
        assert!(
            self.num_channels > 0,
            "as_audio_buffer: buffer has 0 channels"
        );
        assert!(
            self.num_samples > 0,
            "as_audio_buffer: buffer has 0 samples"
        );

        let num_samples = self.num_samples;

        // `chunks_mut` splits the flat data into non-overlapping, equal-length
        // mutable slices — one per channel. Because the Vec length is always
        // exactly `num_channels * num_samples`, every chunk is full.
        let channels: Vec<&mut [T]> = self.data.chunks_mut(num_samples).collect();

        // The invariant (all chunks equal length, at least one chunk) is upheld
        // by the asserts above, so `from_slices` will never fail here.
        AudioBuffer::from_slices(channels)
            .expect("OwnedAudioBuffer invariant violated in as_audio_buffer")
    }

    /// Resize the buffer to new dimensions, reallocating memory if necessary.
    ///
    /// After this call, all samples are **zeroed**. This is an allocating
    /// operation — call it only during setup, not during real-time processing.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dsp::buffer::OwnedAudioBuffer;
    ///
    /// let mut buf = OwnedAudioBuffer::<f32>::new(2, 256);
    /// buf.resize(2, 512); // Grow to a larger block size
    /// assert_eq!(buf.num_samples(), 512);
    /// ```
    pub fn resize(&mut self, num_channels: usize, num_samples: usize) {
        let total = num_channels.saturating_mul(num_samples);
        // Resize the Vec (may allocate or deallocate)
        self.data.resize(total, T::ZERO);
        // Zero any existing data that wasn't overwritten by resize
        self.data.fill(T::ZERO);
        self.num_channels = num_channels;
        self.num_samples = num_samples;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: build a stereo buffer with given value in every sample
    fn make_buffer(val: f32, num_samples: usize) -> (Vec<f32>, Vec<f32>) {
        (vec![val; num_samples], vec![val; num_samples])
    }

    // -----------------------------------------------------------------------
    // AudioBuffer construction
    // -----------------------------------------------------------------------

    #[test]
    fn test_from_slices_valid() {
        let mut ch0 = vec![0.0f32; 256];
        let mut ch1 = vec![0.0f32; 256];

        let buf = AudioBuffer::from_slices(vec![ch0.as_mut_slice(), ch1.as_mut_slice()]).unwrap();
        assert_eq!(buf.num_channels(), 2);
        assert_eq!(buf.num_samples(), 256);
    }

    #[test]
    fn test_from_slices_mono() {
        let mut ch0 = vec![0.5f32; 64];
        let buf = AudioBuffer::from_slices(vec![ch0.as_mut_slice()]).unwrap();
        assert!(buf.is_mono());
        assert!(!buf.is_stereo());
    }

    #[test]
    fn test_from_slices_no_channels() {
        let result = AudioBuffer::<f32>::from_slices(vec![]);
        assert_eq!(result.unwrap_err(), BufferError::NoChannels);
    }

    #[test]
    fn test_from_slices_no_samples() {
        let mut empty: Vec<f32> = vec![];
        let result = AudioBuffer::from_slices(vec![empty.as_mut_slice()]);
        assert_eq!(result.unwrap_err(), BufferError::NoSamples);
    }

    #[test]
    fn test_from_slices_length_mismatch() {
        let mut ch0 = vec![0.0f32; 256];
        let mut ch1 = vec![0.0f32; 128]; // Different length!

        let result = AudioBuffer::from_slices(vec![ch0.as_mut_slice(), ch1.as_mut_slice()]);
        match result.unwrap_err() {
            BufferError::ChannelLengthMismatch { expected, got, channel_index } => {
                assert_eq!(expected, 256);
                assert_eq!(got, 128);
                assert_eq!(channel_index, 1);
            }
            other => panic!("Expected ChannelLengthMismatch, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // AudioBuffer accessors
    // -----------------------------------------------------------------------

    #[test]
    fn test_channel_access() {
        let (mut l, mut r) = make_buffer(1.0, 4);
        r[2] = 0.5; // distinctive value in the right channel

        let buf = AudioBuffer::from_slices(vec![l.as_mut_slice(), r.as_mut_slice()]).unwrap();
        assert_eq!(buf.channel(0), &[1.0f32, 1.0, 1.0, 1.0]);
        assert_eq!(buf.channel(1), &[1.0f32, 1.0, 0.5, 1.0]);
    }

    #[test]
    fn test_channel_mut_access() {
        let mut ch0 = vec![0.0f32; 4];
        let mut buf = AudioBuffer::from_slices(vec![ch0.as_mut_slice()]).unwrap();

        buf.channel_mut(0)[2] = 42.0;
        assert_eq!(buf.channel(0)[2], 42.0);
    }

    #[test]
    fn test_iter_channels() {
        let (mut l, mut r) = make_buffer(1.0, 4);
        let buf = AudioBuffer::from_slices(vec![l.as_mut_slice(), r.as_mut_slice()]).unwrap();

        let sums: Vec<f32> = buf.iter_channels().map(|ch| ch.iter().sum()).collect();
        assert_eq!(sums, vec![4.0f32, 4.0]);
    }

    #[test]
    fn test_iter_channels_mut() {
        let (mut l, mut r) = make_buffer(1.0, 4);
        let mut buf = AudioBuffer::from_slices(vec![l.as_mut_slice(), r.as_mut_slice()]).unwrap();

        // Double every sample through the mutable iterator
        for ch in buf.iter_channels_mut() {
            for s in ch.iter_mut() {
                *s *= 2.0;
            }
        }

        assert_eq!(buf.channel(0), &[2.0f32, 2.0, 2.0, 2.0]);
        assert_eq!(buf.channel(1), &[2.0f32, 2.0, 2.0, 2.0]);
    }

    // -----------------------------------------------------------------------
    // AudioBuffer processing
    // -----------------------------------------------------------------------

    #[test]
    fn test_clear() {
        let (mut l, mut r) = make_buffer(1.0, 4);
        let mut buf = AudioBuffer::from_slices(vec![l.as_mut_slice(), r.as_mut_slice()]).unwrap();

        buf.clear();

        assert_eq!(buf.channel(0), &[0.0f32, 0.0, 0.0, 0.0]);
        assert_eq!(buf.channel(1), &[0.0f32, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_apply_gain() {
        let (mut l, mut r) = make_buffer(1.0, 4);
        let mut buf = AudioBuffer::from_slices(vec![l.as_mut_slice(), r.as_mut_slice()]).unwrap();

        buf.apply_gain(0.5f32);

        for i in 0..4 {
            assert!((buf.channel(0)[i] - 0.5).abs() < 1e-6);
            assert!((buf.channel(1)[i] - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn test_copy_from() {
        let (mut src_l, mut src_r) = make_buffer(0.7, 4);
        let (mut dst_l, mut dst_r) = make_buffer(0.0, 4);

        let src = AudioBuffer::from_slices(vec![src_l.as_mut_slice(), src_r.as_mut_slice()]).unwrap();
        let mut dst = AudioBuffer::from_slices(vec![dst_l.as_mut_slice(), dst_r.as_mut_slice()]).unwrap();

        dst.copy_from(&src);

        assert_eq!(dst.channel(0), src.channel(0));
        assert_eq!(dst.channel(1), src.channel(1));
    }

    #[test]
    fn test_mix_from_unity_gain() {
        // Mixing two identical buffers at unity gain should double the amplitude.
        let (mut src_l, mut src_r) = make_buffer(0.5, 4);
        let (mut dst_l, mut dst_r) = make_buffer(0.5, 4);

        let src = AudioBuffer::from_slices(vec![src_l.as_mut_slice(), src_r.as_mut_slice()]).unwrap();
        let mut dst = AudioBuffer::from_slices(vec![dst_l.as_mut_slice(), dst_r.as_mut_slice()]).unwrap();

        dst.mix_from(&src, 1.0f32);

        // 0.5 + (0.5 * 1.0) = 1.0
        for i in 0..4 {
            assert!((dst.channel(0)[i] - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_mix_from_half_gain() {
        let (mut src_l, mut src_r) = make_buffer(1.0, 4);
        let (mut dst_l, mut dst_r) = make_buffer(0.0, 4);

        let src = AudioBuffer::from_slices(vec![src_l.as_mut_slice(), src_r.as_mut_slice()]).unwrap();
        let mut dst = AudioBuffer::from_slices(vec![dst_l.as_mut_slice(), dst_r.as_mut_slice()]).unwrap();

        dst.mix_from(&src, 0.5f32);

        // 0.0 + (1.0 * 0.5) = 0.5
        for i in 0..4 {
            assert!((dst.channel(0)[i] - 0.5).abs() < 1e-6);
        }
    }

    // -----------------------------------------------------------------------
    // AudioBuffer analysis
    // -----------------------------------------------------------------------

    #[test]
    fn test_peak() {
        let mut ch0 = vec![0.0f32, 0.3, -0.8, 0.5];
        let buf = AudioBuffer::from_slices(vec![ch0.as_mut_slice()]).unwrap();
        // Peak should be the absolute max: |-0.8| = 0.8
        assert!((buf.peak() - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_is_silent() {
        let mut ch0 = vec![0.0f32; 64];
        let mut buf = AudioBuffer::from_slices(vec![ch0.as_mut_slice()]).unwrap();
        assert!(buf.is_silent(1e-6));

        // Set one sample above threshold
        buf.channel_mut(0)[0] = 1e-4;
        assert!(!buf.is_silent(1e-6));
    }

    // -----------------------------------------------------------------------
    // OwnedAudioBuffer
    // -----------------------------------------------------------------------

    #[test]
    fn test_owned_new_zeroed() {
        let buf = OwnedAudioBuffer::<f32>::new(2, 512);
        assert_eq!(buf.num_channels(), 2);
        assert_eq!(buf.num_samples(), 512);
        // All samples must be zero
        assert!(buf.channel(0).iter().all(|&s| s == 0.0));
        assert!(buf.channel(1).iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_owned_channel_mut() {
        let mut buf = OwnedAudioBuffer::<f32>::new(1, 8);
        buf.channel_mut(0)[3] = 99.0;
        assert_eq!(buf.channel(0)[3], 99.0);
    }

    #[test]
    fn test_owned_clear() {
        let mut buf = OwnedAudioBuffer::<f32>::new(2, 4);
        buf.channel_mut(0).fill(1.0);
        buf.channel_mut(1).fill(2.0);

        buf.clear();

        assert!(buf.channel(0).iter().all(|&s| s == 0.0));
        assert!(buf.channel(1).iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_owned_as_audio_buffer() {
        let mut owned = OwnedAudioBuffer::<f32>::new(2, 64);
        // Prefill channel 0 with 1.0
        owned.channel_mut(0).fill(1.0);

        {
            let view = owned.as_audio_buffer();
            assert_eq!(view.num_channels(), 2);
            assert_eq!(view.num_samples(), 64);
            assert_eq!(view.channel(0)[0], 1.0);
        }

        // `owned` is accessible again after the view is dropped
        owned.clear();
        assert_eq!(owned.channel(0)[0], 0.0);
    }

    #[test]
    fn test_owned_as_audio_buffer_processing() {
        let mut owned = OwnedAudioBuffer::<f32>::new(2, 4);
        owned.channel_mut(0).fill(1.0);
        owned.channel_mut(1).fill(1.0);

        // Apply gain through the AudioBuffer view
        let mut view = owned.as_audio_buffer();
        view.apply_gain(0.5f32);
        drop(view);

        // Changes are reflected in the underlying data
        assert!((owned.channel(0)[0] - 0.5).abs() < 1e-6);
        assert!((owned.channel(1)[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_owned_resize() {
        let mut buf = OwnedAudioBuffer::<f32>::new(2, 128);
        buf.channel_mut(0).fill(1.0); // Some data

        buf.resize(2, 512);

        assert_eq!(buf.num_channels(), 2);
        assert_eq!(buf.num_samples(), 512);
        // All samples zeroed after resize
        assert!(buf.channel(0).iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_owned_resize_shrink() {
        let mut buf = OwnedAudioBuffer::<f32>::new(4, 1024);
        buf.resize(1, 64);
        assert_eq!(buf.num_channels(), 1);
        assert_eq!(buf.num_samples(), 64);
    }

    #[test]
    fn test_f64_buffer() {
        // Ensure the generic implementation works with f64 as well.
        let mut ch0 = vec![1.0f64; 8];
        let mut ch1 = vec![2.0f64; 8];
        let mut buf =
            AudioBuffer::from_slices(vec![ch0.as_mut_slice(), ch1.as_mut_slice()]).unwrap();

        buf.apply_gain(0.5f64);
        assert!((buf.channel(0)[0] - 0.5).abs() < 1e-12);
        assert!((buf.channel(1)[0] - 1.0).abs() < 1e-12);
    }
}
