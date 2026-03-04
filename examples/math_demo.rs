//! Example demonstrating the math module.
//!
//! Shows how each sub-module is used in a realistic audio context.

use dsp::math::{
    // Smoothing
    ParamSmoother,
    // Interpolation
    lerp, hermite_interp,
    // Window functions
    fill_hann, apply_window,
    // Conversions
    semitones_to_ratio, cents_to_ratio, bpm_to_seconds, midi_to_freq,
    // Fast math
    fast_tanh, fast_sin, wrap_phase_norm,
};
use dsp::core::SampleRate;

fn main() {
    println!("=== Rust DSP Math Module Example ===\n");

    let sample_rate = SampleRate::new(44100.0).unwrap();

    // -----------------------------------------------------------------------
    // 1. Parameter Smoothing
    // -----------------------------------------------------------------------
    println!("1. Parameter Smoothing");

    // Simulates a user moving a gain knob from 1.0 to 0.0.
    // Without smoothing, this would cause a click; the smoother ramps it down.
    let mut gain_smoother = ParamSmoother::<f32>::new(1.0, 0.02, sample_rate.hz());
    gain_smoother.set_target(0.0); // User moved knob to silent

    // Collect a few snapshots to show convergence
    let checkpoints = [0usize, 100, 441, 882, 2000, 4410];
    let mut last_pos = 0;
    for &pos in &checkpoints {
        let delta = pos - last_pos;
        let mut dummy = [0.0f32; 512];
        let to_run = delta.min(512);
        gain_smoother.process_block(&mut dummy[..to_run]);
        println!("  After {} samples ({:.1} ms): gain = {:.4}",
            pos,
            pos as f64 / sample_rate.hz() * 1000.0,
            gain_smoother.current()
        );
        last_pos = pos;
    }
    println!("  Settled (< 0.001): {}", gain_smoother.is_settled(0.001));
    println!();

    // -----------------------------------------------------------------------
    // 2. Interpolation
    // -----------------------------------------------------------------------
    println!("2. Interpolation");

    // A wavetable oscillator stores one cycle as discrete samples and reads
    // between them at a fractional phase. We simulate reading at phase 2.7
    // from a small lookup table.
    let table: [f32; 5] = [0.0, 0.5, 1.0, 0.5, 0.0]; // Rough triangle

    let frac_index = 2.7_f32;
    let i = frac_index as usize;     // 2
    let t = frac_index.fract();      // 0.7

    // Gather the 4 surrounding samples (with boundary wrapping)
    let y0 = table[(i + table.len() - 1) % table.len()];
    let y1 = table[i];
    let y2 = table[(i + 1) % table.len()];
    let y3 = table[(i + 2) % table.len()];

    let linear_val  = lerp(y1, y2, t);
    let hermite_val = hermite_interp(y0, y1, y2, y3, t);

    println!("  Reading wavetable at index {:.1} (between samples {} and {})", frac_index, i, i+1);
    println!("  Surrounding samples: [{:.1}, {:.1}, {:.1}, {:.1}]", y0, y1, y2, y3);
    println!("  Linear interpolation:  {:.4}", linear_val);
    println!("  Hermite interpolation: {:.4}", hermite_val);
    println!();

    // -----------------------------------------------------------------------
    // 3. Window Functions
    // -----------------------------------------------------------------------
    println!("3. Window Functions (Hann)");

    let block_size = 8;
    let mut window  = vec![0.0f32; block_size];
    let mut audio   = vec![1.0f32; block_size]; // Constant signal

    fill_hann(&mut window);
    println!("  Hann window coefficients:");
    for (i, &w) in window.iter().enumerate() {
        println!("    [{:>2}] {:.4} {}", i, w, "█".repeat((w * 20.0) as usize));
    }

    apply_window(&mut audio, &window);
    println!("  Signal after windowing:");
    for (i, &s) in audio.iter().enumerate() {
        println!("    [{:>2}] {:.4}", i, s);
    }
    println!();

    // -----------------------------------------------------------------------
    // 4. Unit Conversions
    // -----------------------------------------------------------------------
    println!("4. Unit Conversions");

    // Pitch shifting by semitones
    let base_freq = 440.0_f64; // A4
    println!("  Pitch shifting from A4 (440 Hz):");
    for semitones in [-12.0f64, -7.0, -5.0, 0.0, 5.0, 7.0, 12.0] {
        let shifted = base_freq * semitones_to_ratio(semitones);
        println!("    {:+.0} semitones → {:.2} Hz", semitones, shifted);
    }

    // Chorus detune in cents
    println!("\n  Chorus detune (±25 cents from 440 Hz):");
    for cents in [-25.0f64, -10.0, 0.0, 10.0, 25.0] {
        let detuned = base_freq * cents_to_ratio(cents);
        println!("    {:+.0} cents → {:.3} Hz", cents, detuned);
    }

    // Tempo-synced delay
    let bpm = 120.0;
    let beat_time = bpm_to_seconds(bpm);
    let samples_per_beat = sample_rate.seconds_to_samples(beat_time);
    println!("\n  Delay at 120 BPM:");
    println!("    Quarter note = {:.3} s = {:.0} samples", beat_time, samples_per_beat);
    println!("    Eighth note  = {:.3} s = {:.0} samples",
        beat_time / 2.0, samples_per_beat / 2.0);

    // MIDI note names
    println!("\n  MIDI note to frequency:");
    let notes = [("C4", 60.0f64), ("A4", 69.0), ("C5", 72.0), ("A5", 81.0)];
    for (name, note) in &notes {
        println!("    {} (MIDI {:.0}) = {:.2} Hz", name, note, midi_to_freq(*note));
    }
    println!();

    // -----------------------------------------------------------------------
    // 5. Fast Math
    // -----------------------------------------------------------------------
    println!("5. Fast Math Approximations");

    // fast_tanh: soft clipping a hot signal
    println!("  fast_tanh (soft clipper) vs true tanh:");
    for x in [0.0f32, 0.5, 1.0, 2.0, 3.0, 5.0] {
        let approx = fast_tanh(x);
        let exact  = x.tanh();
        let error  = (approx - exact).abs();
        println!("    tanh({:.1}): approx={:.4}  exact={:.4}  error={:.4}", x, approx, exact, error);
    }

    // fast_sin: LFO at 1 Hz, sample a quarter cycle
    println!("\n  fast_sin LFO (1 Hz at 44100 Hz SR):");
    let lfo_freq  = 1.0_f64;
    let phase_inc = lfo_freq / sample_rate.hz();
    let mut phase = 0.0_f32;
    for i in [0usize, 2756, 5513, 8269, 11025] { // every quarter cycle
        let time_ms = i as f64 / sample_rate.hz() * 1000.0;
        let actual_phase = (i as f64 * phase_inc) as f32;
        phase = wrap_phase_norm(actual_phase);
        // Convert normalised phase [0,1) to [-π, π] for fast_sin
        let sin_phase = (phase - 0.5) * std::f32::consts::TAU;
        let lfo_val = fast_sin(sin_phase);
        println!("    t={:.1} ms  phase={:.3}  lfo={:.4}", time_ms, phase, lfo_val);
    }

    println!("\n=== Example Complete ===");
}
