//! Example demonstrating the buffer module.
//!
//! This example walks through the two buffer types and shows how they
//! fit together in a realistic plugin-style processing loop.

use dsp::buffer::{AudioBuffer, OwnedAudioBuffer};
use dsp::core::{Decibels, SILENCE_THRESHOLD};

fn main() {
    println!("=== Rust DSP Buffer Module Example ===\n");

    // -----------------------------------------------------------------------
    // 1. AudioBuffer — wrapping externally-owned data
    // -----------------------------------------------------------------------
    println!("1. AudioBuffer (non-owning view)");

    // In a real plugin, these slices come from the host. Here we simulate that
    // by allocating Vecs and borrowing their slices.
    let block_size = 8;
    let mut left  = vec![0.0f32; block_size];
    let mut right = vec![0.0f32; block_size];

    // Fill with a simple test signal: 1.0 on the left, 0.5 on the right.
    left.fill(1.0);
    right.fill(0.5);

    let mut buf = AudioBuffer::from_slices(vec![left.as_mut_slice(), right.as_mut_slice()])
        .expect("channels have the same length");

    println!("  Channels: {}", buf.num_channels());
    println!("  Block size: {} samples", buf.num_samples());
    println!("  Is stereo: {}", buf.is_stereo());
    println!("  Peak before gain: {:.2}", buf.peak());

    // Apply -6 dB gain (linear ≈ 0.501)
    let gain_linear = Decibels::new(-6.0).linear();
    buf.apply_gain(gain_linear);

    println!("  Peak after -6 dB gain: {:.4}", buf.peak());
    println!("  Left [0]: {:.4}  Right [0]: {:.4}", buf.channel(0)[0], buf.channel(1)[0]);
    println!();

    // -----------------------------------------------------------------------
    // 2. OwnedAudioBuffer — heap-allocated internal buffer
    // -----------------------------------------------------------------------
    println!("2. OwnedAudioBuffer (owned storage)");

    // Allocate a mono scratch buffer for internal processing.
    // In a plugin you'd do this in `prepare()`, not `process()`.
    let mut scratch = OwnedAudioBuffer::<f32>::new(1, block_size);
    println!("  Allocated: {} ch × {} samples", scratch.num_channels(), scratch.num_samples());

    // Write a ramp signal into the scratch buffer.
    for (i, s) in scratch.channel_mut(0).iter_mut().enumerate() {
        *s = i as f32 / block_size as f32;
    }
    println!("  Ramp:  {:?}", scratch.channel(0));

    // Borrow as AudioBuffer to use the full processing API.
    {
        let view = scratch.as_audio_buffer();
        println!("  Peak of ramp: {:.4}", view.peak());
    }

    scratch.clear();
    println!("  After clear: {:?}", scratch.channel(0));
    println!();

    // -----------------------------------------------------------------------
    // 3. Mixing two buffers together
    // -----------------------------------------------------------------------
    println!("3. Mixing buffers");

    let mut dry_l = vec![1.0f32; 4];
    let mut dry_r = vec![1.0f32; 4];
    let mut wet_l = vec![0.5f32; 4];
    let mut wet_r = vec![0.5f32; 4];

    let wet = AudioBuffer::from_slices(vec![wet_l.as_mut_slice(), wet_r.as_mut_slice()]).unwrap();
    let mut dry = AudioBuffer::from_slices(vec![dry_l.as_mut_slice(), dry_r.as_mut_slice()]).unwrap();

    // 50/50 wet-dry mix: mix wet into dry at gain 0.5.
    // After: dry[n] = 1.0 + 0.5 * 0.5 = 1.25
    dry.mix_from(&wet, 0.5f32);

    println!("  After 50%% wet mix — left channel: {:?}", dry.channel(0));
    println!("  Expected value: {:.4}", 1.0f32 + 0.5 * 0.5);
    println!();

    // -----------------------------------------------------------------------
    // 4. Silence detection
    // -----------------------------------------------------------------------
    println!("4. Silence detection");

    let mut silent_data = vec![0.0f32; 64];
    let silent_buf = AudioBuffer::from_slices(vec![silent_data.as_mut_slice()]).unwrap();
    println!("  Empty buffer is silent: {}", silent_buf.is_silent(SILENCE_THRESHOLD));

    // -----------------------------------------------------------------------
    // 5. Plugin-style processing loop simulation
    // -----------------------------------------------------------------------
    println!();
    println!("5. Simulated plugin processing loop");

    struct SimpleGain {
        gain_linear: f32,
        scratch: OwnedAudioBuffer<f32>,
    }

    impl SimpleGain {
        fn new(gain_db: f32, block_size: usize) -> Self {
            Self {
                gain_linear: Decibels::new(gain_db).linear(),
                // Pre-allocate internal buffer. In a real plugin this happens
                // in the `prepare()` callback before audio starts.
                scratch: OwnedAudioBuffer::new(2, block_size),
            }
        }

        fn process(&mut self, output: &mut AudioBuffer<f32>) {
            // Copy output into scratch, process, copy back.
            // (Here we're just demonstrating the API; a real effect would
            //  do something more interesting with the scratch buffer.)
            {
                let mut view = self.scratch.as_audio_buffer();
                view.copy_from(output);
                view.apply_gain(self.gain_linear);
                // After this block, `view` is dropped and `self.scratch` is accessible again.
            }

            let result = self.scratch.as_audio_buffer();
            output.copy_from(&result);
        }
    }

    let block = 4;
    let mut out_l = vec![1.0f32; block];
    let mut out_r = vec![1.0f32; block];
    let mut output = AudioBuffer::from_slices(vec![out_l.as_mut_slice(), out_r.as_mut_slice()]).unwrap();

    let mut plugin = SimpleGain::new(-3.0, block);

    println!("  Input peak: {:.4}", output.peak());
    plugin.process(&mut output);
    println!("  Output peak after -3 dB: {:.4}", output.peak());
    println!("  Expected: {:.4}", Decibels::new(-3.0).linear());

    println!("\n=== Example Complete ===");
}
