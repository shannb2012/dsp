//! Example demonstrating the core module functionality.
//!
//! This example shows how to use the fundamental types and traits
//! from the core module for basic DSP operations.

use dsp::core::*;

fn main() {
    println!("=== Rust DSP Core Module Example ===\n");

    // --- Sample Rate ---
    println!("1. Sample Rate");
    let sample_rate = SampleRate::new(44100.0).unwrap();
    println!("  Sample rate: {}", sample_rate);
    println!("  Nyquist frequency: {} Hz", sample_rate.nyquist());
    println!("  Period: {} seconds", sample_rate.period());

    // Time conversions
    let one_second_samples = sample_rate.seconds_to_samples(1.0);
    let hundred_ms = sample_rate.ms_to_samples(100.0);
    println!("  1 second = {:.0} samples", one_second_samples);
    println!("  100 ms = {:.0} samples", hundred_ms);

    println!();

    // --- Frequency ---
    println!("2. Frequency");
    let a4 = FrequencyHz::new(440.0);
    println!("  A4: {}", a4);
    println!("  MIDI note: {:.2}", a4.to_midi_note());

    // Convert back from MIDI
    let c4 = FrequencyHz::from_midi_note(60.0);
    println!("  Middle C (MIDI 60): {}", c4);

    // Phase increment for oscillator
    let phase_inc = sample_rate.freq_to_phase_increment(440.0);
    println!("  Phase increment for 440 Hz: {:.6}", phase_inc);

    println!();

    // --- Normalized Parameters ---
    println!("3. Normalized Parameters");
    let param = NormalizedParam::new(0.5).unwrap();
    println!("  Parameter value: {}", param);

    // Map to different ranges
    let linear = param.map_linear(0.0, 100.0);
    let log_freq = param.map_log(20.0, 20000.0);
    let exp_time = param.map_exp(0.001, 10.0, 2.0);

    println!("  Linear (0-100): {:.2}", linear);
    println!("  Log frequency (20-20000 Hz): {:.2} Hz", log_freq);
    println!("  Exponential time: {:.3} s", exp_time);

    println!();

    // --- Decibels ---
    println!("4. Decibels");
    let unity_gain = Decibels::ZERO;
    let minus_6db = Decibels::new(-6.0);
    let plus_6db = Decibels::new(6.0);

    println!("  0 dB: {} = {:.4} linear", unity_gain, unity_gain.linear());
    println!("  -6 dB: {} = {:.4} linear", minus_6db, minus_6db.linear());
    println!("  +6 dB: {} = {:.4} linear", plus_6db, plus_6db.linear());

    // Convert from linear
    let half_gain = Decibels::from_linear(0.5);
    println!("  0.5 linear = {}", half_gain);

    println!();

    // --- Time Values ---
    println!("5. Time Values");
    let time = TimeSeconds::new(0.5);
    println!("  Time: {} = {}", time, time.ms());

    let delay_samples = TimeSamples::new(sample_rate.ms_to_samples(250.0) as usize);
    println!("  250 ms delay: {}", delay_samples);

    println!();

    // --- Generic DSP Function ---
    println!("6. Generic DSP Functions");

    // Function that works with any sample type
    fn amplify<T: Sample>(sample: T, gain_db: f32) -> T {
        let gain = Decibels::new(gain_db).linear();
        sample * T::from_f32(gain)
    }

    // Works with f32
    let input_f32 = 0.5f32;
    let output_f32 = amplify(input_f32, -6.0);
    println!("  f32: {:.4} * -6dB = {:.4}", input_f32, output_f32);

    // Works with f64
    let input_f64 = 0.5f64;
    let output_f64 = amplify(input_f64, -6.0);
    println!("  f64: {:.4} * -6dB = {:.4}", input_f64, output_f64);

    println!();

    // --- Constants ---
    println!("7. DSP Constants");
    println!("  Min audible frequency: {:.1} Hz", MIN_AUDIBLE_FREQ);
    println!("  Max audible frequency: {:.1} Hz", MAX_AUDIBLE_FREQ);
    println!("  A4 frequency: {:.1} Hz", A4_FREQUENCY);
    println!("  Default filter Q: {:.3}", DEFAULT_FILTER_Q);
    println!(
        "  Silence threshold: {:.2e} ({:.1} dB)",
        SILENCE_THRESHOLD, SILENCE_THRESHOLD_DB
    );

    println!();

    // --- Practical Example: Simple Gain Processor ---
    println!("8. Practical Example: Simple Gain Processor");

    struct GainProcessor<T: Sample> {
        gain_db: Decibels,
        _phantom: std::marker::PhantomData<T>,
    }

    impl<T: Sample> GainProcessor<T> {
        fn new(gain_db: f32) -> Self {
            Self {
                gain_db: Decibels::new(gain_db),
                _phantom: std::marker::PhantomData,
            }
        }

        fn process(&self, input: T) -> T {
            input * self.gain_db.to_sample::<T>()
        }

        fn set_gain(&mut self, gain_db: f32) {
            self.gain_db = Decibels::new(gain_db);
        }
    }

    let mut processor = GainProcessor::<f32>::new(-3.0);
    let test_signal = 1.0f32;
    let processed = processor.process(test_signal);

    println!("  Input: {:.4}", test_signal);
    println!("  Gain: {}", processor.gain_db);
    println!("  Output: {:.4}", processed);

    processor.set_gain(-6.0);
    let processed2 = processor.process(test_signal);
    println!(
        "  After changing to {}: {:.4}",
        processor.gain_db, processed2
    );

    println!("\n=== Example Complete ===");
}
