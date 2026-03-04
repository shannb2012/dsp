//! Example demonstrating the oscillators module.
//!
//! Shows each oscillator in a realistic context: pitch, waveform character,
//! band-limiting, pulse-width modulation, and noise.

use dsp::oscillators::{
    Oscillator, SineOscillator, SawOscillator,
    SquareOscillator, TriangleOscillator,
    NoiseGenerator, NoiseColor,
};
use dsp::math::ParamSmoother;

fn main() {
    println!("=== Rust DSP Oscillators Module Example ===\n");

    let sample_rate = 44100.0_f64;
    let block_size  = 512_usize;

    // -----------------------------------------------------------------------
    // 1. Sine oscillator — phase accuracy check
    // -----------------------------------------------------------------------
    println!("1. Sine Oscillator (440 Hz A4)");
    {
        let mut osc = SineOscillator::<f32>::new(440.0, sample_rate);

        // Sample a quarter cycle; values should rise from 0 → ~1
        let quarter = (sample_rate / 440.0 / 4.0).round() as usize;
        let first  = osc.process();
        for _ in 1..quarter { osc.process(); }
        let peak = osc.process();

        println!("  Phase=0 (start):          {first:.4}  (expected ≈ 0.0)");
        println!("  Phase≈0.25 (quarter cycle): {peak:.4}  (expected ≈ 1.0)");

        // Demonstrate reset
        osc.reset();
        println!("  After reset():              {:.4}  (expected ≈ 0.0)", osc.process());
    }
    println!();

    // -----------------------------------------------------------------------
    // 2. Sawtooth — naive vs band-limited
    // -----------------------------------------------------------------------
    println!("2. Sawtooth Oscillator (440 Hz) — naive vs PolyBLEP");
    {
        let mut naive = SawOscillator::<f32>::new(440.0, sample_rate);
        let mut bl    = SawOscillator::<f32>::new(440.0, sample_rate);
        naive.set_band_limited(false);
        bl.set_band_limited(true);

        // Compute peak absolute value over one block
        let peak = |osc: &mut SawOscillator<f32>| -> f32 {
            (0..block_size).map(|_| osc.process().abs()).fold(0.0f32, f32::max)
        };

        let peak_naive = peak(&mut naive);
        let peak_bl    = peak(&mut bl);

        println!("  Naive peak (should be ≈ 1.0):    {peak_naive:.4}");
        println!("  BL peak    (slight overshoot OK): {peak_bl:.4}");

        // Show first three values of naive saw (rising from -1)
        let mut demo = SawOscillator::<f32>::new(100.0, sample_rate);
        demo.set_band_limited(false);
        let s0 = demo.process();
        let s1 = demo.process();
        let s2 = demo.process();
        println!("  Naive @ 100 Hz first 3 samples: [{s0:.4}, {s1:.4}, {s2:.4}]");
        println!("  (each should be slightly > previous — linear rise from -1)");
    }
    println!();

    // -----------------------------------------------------------------------
    // 3. Square oscillator — pulse width modulation
    // -----------------------------------------------------------------------
    println!("3. Square Oscillator (100 Hz) — pulse width modulation");
    {
        let freq = 100.0;
        let n    = (sample_rate / freq) as usize; // one period

        // At 50% pw: mean over one period should be ≈ 0 (equal time +1 and -1)
        let mut sq50 = SquareOscillator::<f32>::new(freq, sample_rate);
        sq50.set_band_limited(false);
        sq50.set_pulse_width(0.5);
        let mean50: f32 = (0..n).map(|_| sq50.process()).sum::<f32>() / n as f32;

        // At 25% pw: mean should be ≈ -0.5
        let mut sq25 = SquareOscillator::<f32>::new(freq, sample_rate);
        sq25.set_band_limited(false);
        sq25.set_pulse_width(0.25);
        let mean25: f32 = (0..n).map(|_| sq25.process()).sum::<f32>() / n as f32;

        println!("  50% duty cycle DC: {mean50:.4}  (expected ≈ 0.0)");
        println!("  25% duty cycle DC: {mean25:.4}  (expected ≈ -0.5)");

        // PWM demo: slowly sweep pulse width with a smoother
        println!("\n  PWM sweep (pulse width modulated by triangle LFO):");
        let mut osc = SquareOscillator::<f32>::new(440.0, sample_rate);
        let mut pw_smoother = ParamSmoother::<f32>::new(0.1, 0.05, sample_rate);
        let lfo_targets = [0.1f32, 0.3, 0.5, 0.7, 0.9];
        for &target_pw in &lfo_targets {
            pw_smoother.set_target(target_pw);
            // Settle the smoother for ~2205 samples (50 ms)
            for _ in 0..2205 {
                let pw = pw_smoother.process();
                osc.set_pulse_width(pw as f64);
                osc.process();
            }
            println!("  pw → {target_pw:.1}: settled pw = {:.3}", osc.pulse_width());
        }
    }
    println!();

    // -----------------------------------------------------------------------
    // 4. Triangle oscillator — amplitude and zero crossings
    // -----------------------------------------------------------------------
    println!("4. Triangle Oscillator (440 Hz)");
    {
        let freq = 440.0;
        let mut osc = TriangleOscillator::<f32>::new(freq, sample_rate);
        let period  = (sample_rate / freq).round() as usize;

        let mut min = f32::MAX;
        let mut max = f32::MIN;
        let mut zero_crossings = 0usize;
        let mut prev = osc.process();
        min = min.min(prev);
        max = max.max(prev);

        for _ in 1..period * 10 {
            let cur = osc.process();
            if (prev < 0.0) != (cur < 0.0) {
                zero_crossings += 1;
            }
            min = min.min(cur);
            max = max.max(cur);
            prev = cur;
        }

        println!("  Min amplitude: {min:.4}  (expected ≈ -1.0)");
        println!("  Max amplitude: {max:.4}  (expected ≈ +1.0)");
        // Triangle crosses zero twice per period
        println!("  Zero crossings in 10 periods: {zero_crossings}  (expected ≈ 20)");
    }
    println!();

    // -----------------------------------------------------------------------
    // 5. Noise generator — statistics by color
    // -----------------------------------------------------------------------
    println!("5. Noise Generator — statistics by color");
    {
        let n = 100_000usize;

        for color in [NoiseColor::White, NoiseColor::Pink, NoiseColor::Brown] {
            let mut ng = NoiseGenerator::new(color, 0xABCD_1234);
            // Warm up pink/brown filter state
            for _ in 0..1000 { ng.process(); }

            let samples: Vec<f32> = (0..n).map(|_| ng.process()).collect();
            let mean = samples.iter().sum::<f32>() / n as f32;
            let rms  = (samples.iter().map(|x| x * x).sum::<f32>() / n as f32).sqrt();
            let max  = samples.iter().cloned().fold(f32::MIN, f32::max);
            let min  = samples.iter().cloned().fold(f32::MAX, f32::min);

            let name = format!("{color:?}");
            println!("  {name:<8} mean={mean:+.4}  rms={rms:.4}  range=[{min:.3}, {max:.3}]");
        }
    }
    println!();

    // -----------------------------------------------------------------------
    // 6. Plugin-style voice: smoothed pitch + oscillator
    // -----------------------------------------------------------------------
    println!("6. Plugin-style Voice: sine osc with smoothed pitch change");
    {
        let mut osc = SineOscillator::<f32>::new(440.0, sample_rate);
        // Smooth frequency changes over 10 ms to avoid zipper noise
        let mut freq_smoother = ParamSmoother::<f32>::new(440.0, 0.01, sample_rate);

        // Simulate a MIDI pitch bend: 440 → 880 Hz (one octave up)
        freq_smoother.set_target(880.0);

        let mut output = vec![0.0f32; block_size];
        for slot in output.iter_mut() {
            let freq = freq_smoother.process();
            osc.set_frequency(freq as f64);
            *slot = osc.process();
        }

        let final_freq = freq_smoother.current();
        let settled = freq_smoother.is_settled(1.0);
        println!("  After {block_size} samples: freq = {final_freq:.1} Hz");
        println!("  Smoother settled (±1 Hz): {settled}");
        println!("  Output sample[0] = {:.4}", output[0]);
        println!("  Output sample[255] = {:.4}", output[255]);
        println!("  Output sample[511] = {:.4}", output[511]);
    }

    println!("\n=== Example Complete ===");
}
