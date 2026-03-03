# Rust DSP Library

A modern, type-safe Digital Signal Processing library for real-time audio applications, built in Rust.

## Features

- **Type Safety**: Leverage Rust's type system to prevent common DSP errors at compile time
- **Performance**: Zero-cost abstractions with performance matching hand-optimized C++
- **Real-Time Safe**: No heap allocations in audio processing paths
- **Generic**: Works with both f32 and f64 sample types
- **Modular**: Use only what you need

## Project Status

🚧 **Early Development** - Currently implementing Phase 1 (Core Module)

### Completed
- Core module with Sample trait
- Type-safe SampleRate handling
- Parameter types (NormalizedParam, FrequencyHz, TimeSeconds, Decibels)
- DSP constants

### In Progress
- Buffer module
- Math module (interpolation, smoothing)
- Basic oscillators

### Planned
- Filters (biquad, SVF, one-pole)
- Envelopes (ADSR)
- Effects (delay, chorus, reverb)

## Quick Start

```rust
use dsp::core::{Sample, SampleRate, FrequencyHz, NormalizedParam};

// Create a sample rate
let sample_rate = SampleRate::new(44100.0).unwrap();

// Work with frequencies
let freq = FrequencyHz::new(440.0); // A4
let midi_note = freq.to_midi_note(); // 69

// Handle parameters type-safely
let gain_param = NormalizedParam::new(0.5).unwrap();
let actual_gain = gain_param.map_linear(-12.0, 12.0); // Map to dB range

// Generic over sample type
fn process<T: Sample>(input: T, gain: T) -> T {
    input * gain
}

let output_f32 = process(0.5f32, 2.0f32);
let output_f64 = process(0.5f64, 2.0f64);
```

## Core Module

The core module provides fundamental types and traits:

### Sample Trait

Generic trait for audio sample types (f32, f64) with all necessary mathematical operations.

```rust
pub trait Sample: Copy + Clone + ... {
    const ZERO: Self;
    const ONE: Self;
    const PI: Self;
    // ... more constants and methods
}
```

### SampleRate

Type-safe wrapper around sample rate with helpful conversions:

```rust
let sr = SampleRate::new(48000.0).unwrap();
let nyquist = sr.nyquist(); // 24000 Hz
let samples = sr.seconds_to_samples(0.1); // 4800 samples
let omega = sr.freq_to_angular(440.0); // Angular frequency
```

### Parameter Types

Distinct types prevent mixing different parameter domains:

- `NormalizedParam`: Values in [0, 1] for DAW automation
- `FrequencyHz`: Frequency in Hertz with MIDI note conversion
- `TimeSeconds`: Time values with ms conversion
- `TimeSamples`: Sample counts for delays
- `Decibels`: dB values with linear conversion

## Design Principles

1. **Real-Time First**: Every component is designed for real-time audio processing
   - No allocations in process() methods
   - Predictable, bounded execution time
   - Pre-allocation during initialization

2. **Type Safety**: Use the type system to prevent errors
   - Distinct types for different parameter domains
   - Generic over sample type (f32/f64)
   - Compile-time guarantees where possible

3. **Zero-Cost Abstractions**: Clean APIs without performance penalty
   - Traits compile to direct method calls
   - Inlining for hot paths
   - LLVM optimizations

4. **Modular Design**: Use what you need
   - Independent modules
   - Minimal dependencies
   - Clear dependency flow

## Building

```bash
cargo build --release
```

## Testing

```bash
cargo test
```

## Documentation

```bash
cargo doc --open
```

## Benchmarking

```bash
cargo bench
```

## Roadmap

See [Design Document](docs/rust_dsp_library_design.md) for detailed roadmap and architecture.

### Phase 1: Foundation (Weeks 1-4)
- Core types and traits ✅
- Buffer module
- Math utilities
- Basic oscillators

### Phase 2: Core DSP (Weeks 5-8)
- Complete oscillator set
- Biquad filter
- State variable filter
- ADSR envelope
- Simple delay

### Phase 3: Effects & Polish (Weeks 9-12)
- More effects (chorus, reverb, distortion)
- FIR filter
- Comprehensive test suite
- Performance benchmarks
- Documentation and examples

## Contributing

Contributions welcome! Please read our contributing guidelines.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Inspiration

This project draws inspiration from:
- [JUCE](https://juce.com/) - The C++ framework for audio applications
- [nih-plug](https://github.com/robbert-vdh/nih-plug) - Modern Rust plugin framework
- [dasp](https://github.com/RustAudio/dasp) - Digital audio signal processing primitives

## Resources

- [The Audio Programming Book](http://www.音audioprogramingbook.com/)
- [Designing Audio Effect Plugins in C++](https://www.willpirkle.com/)
- [DAFX: Digital Audio Effects](https://www.dafx.de/)
