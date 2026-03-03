# Contributor Onboarding Guide

Welcome to the DSP library. This document gets you from zero to making meaningful contributions,
covering the audio programming concepts the code is built on and the Rust patterns you'll encounter
throughout the codebase.

---

## Table of Contents

1. [What we're building and why](#1-what-were-building-and-why)
2. [Audio programming fundamentals](#2-audio-programming-fundamentals)
   - [Digital audio basics](#21-digital-audio-basics)
   - [Sample rates and Nyquist](#22-sample-rates-and-nyquist)
   - [Amplitude, decibels, and gain](#23-amplitude-decibels-and-gain)
   - [Block-based processing](#24-block-based-processing)
   - [Real-time constraints](#25-real-time-constraints)
   - [Signal types: frequency, phase, time](#26-signal-types-frequency-phase-and-time)
3. [Rust fundamentals for this codebase](#3-rust-fundamentals-for-this-codebase)
   - [Traits and generics](#31-traits-and-generics)
   - [Lifetimes](#32-lifetimes)
   - [Zero-cost abstractions](#33-zero-cost-abstractions)
   - [Ownership and the borrow checker in DSP code](#34-ownership-and-the-borrow-checker-in-dsp-code)
   - [Inline and performance hints](#35-inline-and-performance-hints)
4. [Project architecture walkthrough](#4-project-architecture-walkthrough)
   - [Module structure](#41-module-structure)
   - [The Sample trait](#42-the-sample-trait)
   - [The Buffer module](#43-the-buffer-module)
5. [Getting your environment set up](#5-getting-your-environment-set-up)
6. [How to contribute a new DSP component](#6-how-to-contribute-a-new-dsp-component)
7. [Further reading](#7-further-reading)

---

## 1. What we're building and why

This is a **DSP (Digital Signal Processing) library** written in Rust. Its purpose is to be the
signal-processing engine inside audio plugins — the software that lives inside a DAW (Digital Audio
Workstation) like Ableton, Logic, or Reaper.

Think of it like this: a plugin framework (similar to JUCE or nih-plug) handles the plumbing —
talking to the operating system, managing GUI, dealing with VST/AU/CLAP formats. This library
handles the *math* — oscillators, filters, envelopes, effects.

**Why Rust?**
- The audio thread is one of the strictest real-time environments in software. Memory allocations,
  lock contention, or unpredictable garbage collection cause audible glitches (clicks, dropouts).
  Rust's ownership model and `#[forbid(unsafe)]` style enforce correctness *at compile time* rather
  than crashing at runtime.
- C++ is the traditional choice (JUCE is C++), but Rust gives us memory safety and a modern type
  system essentially for free.

---

## 2. Audio programming fundamentals

### 2.1 Digital audio basics

Sound is a pressure wave in air. A microphone converts that wave to a continuous electrical voltage.
An **Analog-to-Digital Converter (ADC)** samples that voltage at a fixed rate, turning the continuous
signal into a sequence of discrete numbers. Those numbers are **audio samples**.

```
Continuous waveform:   ~~~~~~~~~~~~~~~~~~~~

Sampled:               | | | | | | | | | |
                       0 1 2 3 4 5 6 7 8 9  (sample index)
```

Each sample is just a floating-point number representing the air pressure at that instant.
In this library, samples are either `f32` (32-bit float, standard for plugins) or `f64`
(64-bit, used when higher precision is needed).

**Amplitude convention:** Samples are normalized to the range `[-1.0, 1.0]`. A sample of `1.0`
represents the maximum positive pressure swing; `-1.0` is maximum negative. Values outside this
range are considered *clipping* — distortion caused by exceeding the hardware's range.

### 2.2 Sample rates and Nyquist

The **sample rate** (measured in Hz) is how many samples are taken per second.

| Common rate | Used in                                      |
|-------------|----------------------------------------------|
| 44,100 Hz   | CDs, most consumer audio                     |
| 48,000 Hz   | Professional audio, video/broadcast           |
| 96,000 Hz   | High-resolution studio recording             |
| 192,000 Hz  | Ultra-HD audio (rarely needed in practice)   |

The **Nyquist theorem** states that you can only accurately represent frequencies up to *half* the
sample rate. This limit is called the **Nyquist frequency**.

```
Sample rate 44,100 Hz → Nyquist frequency 22,050 Hz
```

Human hearing tops out around 20,000 Hz, so 44,100 Hz is enough to capture everything we can hear
with a little headroom.

**Why this matters in code:**

Filters and oscillators must respect the Nyquist limit. An oscillator asked to play at 30,000 Hz
when the sample rate is 44,100 Hz will produce **aliasing** — an incorrect, often buzzy artifact
frequency that appears at `44,100 - 30,000 = 14,100 Hz`. We clamp frequencies to the Nyquist limit
throughout the codebase.

See `src/core/sample_rate.rs` — `SampleRate::nyquist()` and `FrequencyHz::clamp_nyquist()`.

### 2.3 Amplitude, decibels, and gain

**Linear amplitude** is what the CPU operates on: the raw floating-point sample value.

**Decibels (dB)** are how humans perceive loudness. The relationship is logarithmic:

```
dB = 20 × log₁₀(linear_amplitude)

Linear → dB:
  1.0   →   0 dB   (unity gain, no change)
  0.5   →  -6 dB   (half amplitude, roughly half as loud)
  2.0   →  +6 dB   (double amplitude, roughly twice as loud)
  0.0   →  -∞ dB   (silence)
```

This is why the human perception of loudness doubles every ~10 dB, not every linear step.

**Practical rule:** Every -6 dB halves the linear amplitude. Every +6 dB doubles it.

In the codebase, `Decibels` in `src/core/parameter.rs` handles this conversion:

```rust
let gain = Decibels::new(-6.0).linear(); // → ~0.501
let db   = Decibels::from_linear(0.5);  // → ~-6.02 dB
```

Always store and process audio in **linear** amplitude. Convert to/from dB only at the UI layer
or when reading user-facing parameter values.

### 2.4 Block-based processing

Plugins don't process one sample at a time. They receive a **block** (also called a buffer or
chunk) of samples at a time, typically 64–2048 samples.

```
Block size 512 at 44,100 Hz → about 11.6ms of audio per block
Block size 64  at 44,100 Hz → about 1.45ms (lower latency, more CPU overhead)
```

The host (DAW) calls the plugin's `process()` callback repeatedly, handing it one block of input
samples and expecting one block of output samples in return.

```
Host calls process(buffer) → plugin fills buffer → host sends to speakers
                         ↑
                    11.6 ms deadline
```

If `process()` takes longer than the block duration, audio dropouts occur. This is why real-time
safety is paramount.

In our codebase, this is represented by `AudioBuffer` (see `src/buffer/`). Each call to `process()`
receives an `&mut AudioBuffer` containing the current block.

### 2.5 Real-time constraints

The audio thread is special. The OS gives it elevated priority, but it also imposes strict rules:

**Forbidden in `process()` (the hot path):**
- `Vec::push` / `String::new` / anything that calls `malloc` (heap allocation)
- `Mutex::lock` (may block waiting for another thread)
- File I/O, network calls (may block indefinitely)
- `println!` (may lock an internal mutex)
- `thread::sleep` or any waiting

**Allowed:**
- Stack allocation (local variables)
- Arithmetic on pre-allocated data
- Reading/writing to pre-allocated buffers
- Atomic operations (for lock-free parameter updates)

**The pattern used throughout this library:**

```rust
// prepare() — called before audio starts, NOT real-time
fn prepare(&mut self, sample_rate: SampleRate, max_block: usize) {
    self.buffer = OwnedAudioBuffer::new(2, max_block); // Allocation OK here
    self.filter.reset();
}

// process() — called on the audio thread, must be real-time safe
fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
    // No allocations. Only math on pre-allocated state.
    buffer.apply_gain(self.gain);
}
```

### 2.6 Signal types: frequency, phase, and time

#### Frequency and phase

An oscillator (sine wave generator) works by maintaining a **phase accumulator**: a running counter
that wraps from 0.0 to 1.0 (one full cycle).

```
phase_increment = frequency_hz / sample_rate_hz

Each sample:
  phase += phase_increment
  if phase >= 1.0 { phase -= 1.0 }  // Wrap
  output = sin(phase * 2π)
```

For a 440 Hz sine at 44,100 Hz: `phase_increment = 440 / 44100 ≈ 0.00997`

This is why `SampleRate::freq_to_phase_increment()` exists — it's the core calculation for every
oscillator we'll write.

#### Filters and time-domain state

Filters work by remembering previous samples. An **IIR (Infinite Impulse Response)** filter like
a biquad uses its own previous inputs and outputs to compute the current output. The "memory" of
previous samples is stored as **filter state**.

```
y[n] = b0*x[n] + b1*x[n-1] + b2*x[n-2] - a1*y[n-1] - a2*y[n-2]
                               ↑ history ↑              ↑ history ↑
```

This means filters must maintain state between `process()` calls. They're stateful objects, not
pure functions.

#### Time: seconds vs. samples

DSP code switches between two time units constantly:
- **Seconds** — human-readable, used for parameter values (attack time: 0.5s)
- **Samples** — what the CPU works with (at 44,100 Hz, 0.5s = 22,050 samples)

`SampleRate` provides the bridge: `sr.seconds_to_samples(0.5)`.

---

## 3. Rust fundamentals for this codebase

If you know Rust basics (ownership, borrowing, structs, enums), this section covers the specific
patterns used throughout this library. If you're new to Rust, work through
[The Rust Book](https://doc.rust-lang.org/book/) chapters 1–10 first.

### 3.1 Traits and generics

**The key insight:** We write DSP code once, and it works for both `f32` and `f64` sample types.
This is achieved through Rust's `trait` system.

The `Sample` trait (in `src/core/sample.rs`) is a contract: "any type that implements `Sample` can
be used as an audio sample." Both `f32` and `f64` implement it.

```rust
pub trait Sample: Copy + Clone + Debug + ... {
    const ZERO: Self;
    const PI: Self;
    fn sin(self) -> Self;
    fn abs(self) -> Self;
    // ... many more
}
```

A generic function using it:

```rust
// Works for f32 AND f64 — the compiler generates two versions
fn amplify<T: Sample>(sample: T, gain: T) -> T {
    sample * gain
}
```

**When you write a new DSP component**, use generics:

```rust
pub struct MyFilter<T: Sample> {
    state: T,   // Works for f32 or f64
}

impl<T: Sample> MyFilter<T> {
    pub fn process(&mut self, input: T) -> T {
        // Use T::ZERO, T::ONE, input.sin(), etc.
        self.state = self.state + (input - self.state) * T::from_f64(0.1);
        self.state
    }
}
```

**Why not just use `f32` everywhere?** Some DAWs process internally at `f64` precision, especially
for mixdown. By being generic, the same filter code works at either precision without duplication.

### 3.2 Lifetimes

Lifetimes are Rust's way of tracking how long a borrow is valid. You'll encounter them primarily in
the `buffer` module.

**The core lifetime in this codebase:**

```rust
pub struct AudioBuffer<'a, T: Sample> {
    channels: Vec<&'a mut [T]>,
}
```

The `'a` means: "the channel slices inside this buffer live at least as long as `'a`." In practice
this means `AudioBuffer` cannot outlive the data it points into.

```rust
let mut left = vec![0.0f32; 512];
let mut right = vec![0.0f32; 512];

// `buf` borrows `left` and `right`. It cannot outlive them.
let buf = AudioBuffer::from_slices(vec![left.as_mut_slice(), right.as_mut_slice()]).unwrap();
// `left` and `right` are dropped here, then `buf` is dropped — safe.
```

**Why non-owning?** The plugin host owns audio buffers. We need to work *in place* on the host's
memory without copying. A non-owning view makes zero-copy processing possible.

**The reborrow pattern:** You'll see this in `channel_mut()`:

```rust
pub fn channel_mut(&mut self, index: usize) -> &mut [T] {
    &mut *self.channels[index]  // ← reborrow
}
```

`self.channels[index]` has type `&'a mut [T]`. We can't return `&'a mut [T]` directly (that would
allow holding a mutable reference while also holding `&mut self`). Instead, `&mut *...` reborrowing
shortens the lifetime to `&mut self`, which is safe: the borrow checker prevents aliasing.

**Rule of thumb:** If you see `'a` in a struct, that struct is a *view* into someone else's data.
If there's no `'a`, the struct owns its data.

### 3.3 Zero-cost abstractions

Rust's generics use **monomorphization**: when you call `amplify::<f32>(...)`, the compiler
generates a concrete `amplify_f32` function with no generics, no runtime dispatch, no overhead.

This means:
```rust
fn amplify<T: Sample>(sample: T, gain: T) -> T { sample * gain }
// Compiles to the same machine code as:
fn amplify_f32(sample: f32, gain: f32) -> f32 { sample * gain }
```

There is no virtual dispatch (unlike C++ virtual functions or Rust `dyn Trait`) in the hot path.
This is why `T: Sample` in a struct definition is cheap — no vtable, no indirection.

**When to use `dyn Trait`:** Almost never in this library. `dyn Trait` causes heap allocation and
virtual dispatch — both forbidden on the audio thread.

### 3.4 Ownership and the borrow checker in DSP code

DSP code is stateful — filters, oscillators, and envelopes remember their previous output. Rust's
ownership model means state is always *owned* by exactly one place.

**Pattern: stateful DSP component**

```rust
pub struct OnePoleFilter<T: Sample> {
    state: T,        // Internal memory — only this struct can touch it
    coefficient: T,
}

impl<T: Sample> OnePoleFilter<T> {
    pub fn process(&mut self, input: T) -> T {
        // `&mut self` — we have exclusive access to `self.state`
        self.state = self.state + (input - self.state) * self.coefficient;
        self.state
    }
}
```

The `&mut self` in `process()` is the audio-thread contract: exactly one thread calls this at a
time, no synchronization needed. The plugin framework (not this library) is responsible for
ensuring that.

**Common borrow checker challenge:** Splitting a buffer into non-overlapping parts. If you try to
call `channel_mut(0)` and `channel_mut(1)` simultaneously, the borrow checker stops you:

```rust
// This DOESN'T compile — two mutable borrows of `buf`
let left  = buf.channel_mut(0);
let right = buf.channel_mut(1);
left[0] = right[0]; // borrow checker error
```

Use `iter_channels_mut()` instead, which uses Rust's `split_at_mut` semantics to prove the slices
are non-overlapping:

```rust
// This works
for ch in buf.iter_channels_mut() {
    ch[0] = 0.5;
}
```

### 3.5 Inline and performance hints

You'll see `#[inline]` on accessor methods throughout the codebase:

```rust
#[inline]
pub fn num_channels(&self) -> usize {
    self.channels.len()
}
```

`#[inline]` tells the compiler to *prefer* inlining this call at its call sites, eliminating
function call overhead. This matters when a method is called millions of times per second
(once per sample, across all channels, at 44,100+ Hz).

**Guidelines for when to use `#[inline]`:**
- Trivial accessors (getters) — always
- Methods called in per-sample loops — almost always
- Complex algorithms — rarely, let the compiler decide

`#[inline(always)]` forces inlining. Use only for truly hot-path micro-operations (e.g., a single
multiplication). Overuse bloats the binary and can hurt instruction cache.

---

## 4. Project architecture walkthrough

### 4.1 Module structure

```
src/
├── lib.rs                  # Crate root — declares public modules
├── core/                   # Foundation layer (no dependencies on other modules)
│   ├── mod.rs
│   ├── sample.rs           # Sample trait (the bedrock of everything)
│   ├── sample_rate.rs      # SampleRate type
│   ├── parameter.rs        # FrequencyHz, Decibels, NormalizedParam, etc.
│   └── constants.rs        # Physical/audio constants
└── buffer/                 # Depends on: core
    ├── mod.rs
    └── audio_buffer.rs     # AudioBuffer<'a,T> and OwnedAudioBuffer<T>
```

**Dependency rule:** Dependencies only flow *upward*. `buffer` can use `core`. `core` cannot use
`buffer`. This prevents circular dependencies and lets lower-level modules be used independently.

```
future: effects → oscillators + filters + envelopes + buffer + math
future: filters → core + math
future: math    → core
buffer          → core
core            (no internal deps)
```

### 4.2 The Sample trait

`src/core/sample.rs` — read this first.

The `Sample` trait is the foundation everything else builds on. Every DSP algorithm is written
in terms of `T: Sample` rather than `f32` directly.

Key constants you'll use constantly:
- `T::ZERO` — additive identity (0.0)
- `T::ONE` — multiplicative identity (1.0)
- `T::PI`, `T::TAU` — mathematical constants
- `T::from_f64(v)` — convert literal constants like `0.5` to whatever `T` is

Key methods:
- `sample.abs()`, `sample.sin()`, `sample.cos()`, etc. — standard math, all inline
- `sample.clamp(min, max)` — essential for preventing overflow and protecting speakers
- `sample.is_finite()` — defensive check; NaN/Inf in audio causes hardware damage

**How to write a DSP coefficient calculation:**

```rust
// DON'T do this — loses precision when T = f32, breaks when T = f64
let coeff = 0.5f32;

// DO this — works for both f32 and f64
let coeff = T::from_f64(0.5);
```

### 4.3 The Buffer module

`src/buffer/audio_buffer.rs` — the second file to read.

**`AudioBuffer<'a, T>`** — non-owning, non-interleaved.

Think of it as a safe wrapper around what a C plugin would receive as an array of channel pointers:

```c
// C plugin API
void process(float** channels, int num_channels, int num_samples);
```

```rust
// Rust equivalent
fn process(buffer: &mut AudioBuffer<f32>) { ... }
```

The key operations:

```rust
// Read a channel
let samples: &[T] = buf.channel(0);

// Write to a channel
let samples: &mut [T] = buf.channel_mut(0);

// Clear the buffer (fill with silence)
buf.clear();

// Apply gain
buf.apply_gain(T::from_f64(0.5));

// Mix another buffer in
buf.mix_from(&other, T::ONE);
```

**`OwnedAudioBuffer<T>`** — allocates its own memory.

Use this for any buffer a DSP component needs to own internally: delay lines, scratch space, etc.

```rust
// In prepare():
let delay_buf = OwnedAudioBuffer::<f32>::new(2, 44100); // 1 second of stereo

// In process():
let view = delay_buf.as_audio_buffer(); // Zero-copy borrow
view.apply_gain(feedback);
```

---

## 5. Getting your environment set up

**Prerequisites:**
- Rust stable (install via [rustup.rs](https://rustup.rs))
- No external C dependencies — everything is pure Rust

**Clone and verify:**

```bash
git clone <repo-url>
cd dsp

# Build
cargo build

# Run all tests (unit + doc tests)
cargo test

# Run examples
cargo run --example core_demo
cargo run --example buffer_demo

# Generate and open documentation
cargo doc --open
```

**Recommended tools:**

```bash
# Linter — catches common mistakes
cargo clippy

# Auto-formatter — run before every commit
cargo fmt

# Check without compiling fully (faster feedback loop)
cargo check
```

**Editor setup:** Any editor with `rust-analyzer` works (VS Code + rust-analyzer extension is the
most popular). `rust-analyzer` gives you inline type hints which are invaluable when working with
generic `T: Sample` code.

---

## 6. How to contribute a new DSP component

Let's walk through adding a hypothetical **one-pole lowpass filter** to illustrate the conventions.

### Step 1: Decide which module it belongs to

A one-pole filter goes in `src/filters/`. If that module doesn't exist yet, create it.

```bash
mkdir src/filters
touch src/filters/mod.rs
touch src/filters/one_pole.rs
```

### Step 2: Add the module to `src/lib.rs`

```rust
pub mod core;
pub mod buffer;
pub mod filters;  // ← add this
```

### Step 3: Write the implementation

Follow the conventions in existing files:

```rust
// src/filters/one_pole.rs

use crate::core::Sample;

/// A simple one-pole lowpass filter.
///
/// The transfer function is: y[n] = x[n] * (1 - a) + y[n-1] * a
///
/// where `a` is the filter coefficient (0.0 = no filtering, ~1.0 = heavy filtering).
///
/// This is the simplest useful IIR filter. Very efficient and commonly used for:
/// - Parameter smoothing (prevent clicks when a knob moves)
/// - DC blocking (remove constant offset from a signal)
/// - Gentle high-frequency attenuation
///
/// For sharper cutoffs, use the biquad filter instead.
pub struct OnePoleFilter<T: Sample> {
    // The "memory" of the filter — previous output sample.
    // IIR filters are defined by their state; this is what makes them stateful.
    state: T,
    // Pre-computed coefficient. Stored to avoid recomputation every sample.
    coefficient: T,
}

impl<T: Sample> OnePoleFilter<T> {
    /// Create a new one-pole filter.
    ///
    /// `cutoff_hz` — the -3 dB point in Hz. Frequencies above this are attenuated.
    /// `sample_rate` — must match the audio stream's sample rate.
    pub fn new(cutoff_hz: f64, sample_rate: f64) -> Self {
        let mut filter = Self {
            state: T::ZERO,
            coefficient: T::ZERO,
        };
        filter.set_cutoff(cutoff_hz, sample_rate);
        filter
    }

    /// Update the cutoff frequency.
    ///
    /// The coefficient is derived from the desired time constant.
    /// This involves a transcendental function (exp), so call from prepare(),
    /// or use parameter smoothing if you need to change cutoff in process().
    pub fn set_cutoff(&mut self, cutoff_hz: f64, sample_rate: f64) {
        // Time constant τ = 1 / (2π × fc)
        // Discrete coefficient: a = e^(-1 / (τ × fs)) = e^(-2π × fc / fs)
        let omega = -std::f64::consts::TAU * cutoff_hz / sample_rate;
        self.coefficient = T::from_f64(omega.exp());
    }

    /// Process a single sample.
    ///
    /// # Real-time safety
    ///
    /// No allocations. Only a multiply-accumulate operation. O(1).
    #[inline]
    pub fn process(&mut self, input: T) -> T {
        // y[n] = x[n] * (1 - a) + y[n-1] * a
        //      = x[n] + (y[n-1] - x[n]) * a
        let one_minus_a = T::ONE - self.coefficient;
        self.state = input * one_minus_a + self.state * self.coefficient;
        self.state
    }

    /// Reset filter state to silence.
    ///
    /// Call this when starting a new sound or when the plugin is reset.
    pub fn reset(&mut self) {
        self.state = T::ZERO;
    }
}
```

### Step 4: Write tests

Tests live in the same file, under `#[cfg(test)]`. Cover:
- Correct output values (compare against reference)
- Edge cases (zero input, max frequency)
- Stability (no NaN/Inf after many samples)
- Reset behavior

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dc_passthrough() {
        // At 0 Hz input (DC), a lowpass filter should pass it through eventually.
        let mut filter = OnePoleFilter::<f32>::new(1000.0, 44100.0);
        // Feed 1.0 for many samples — output should converge to 1.0
        for _ in 0..10000 {
            filter.process(1.0);
        }
        assert!((filter.process(1.0) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_silence_input_stays_silent() {
        let mut filter = OnePoleFilter::<f32>::new(1000.0, 44100.0);
        for _ in 0..100 {
            let out = filter.process(0.0);
            assert!(out.is_finite());
        }
    }

    #[test]
    fn test_reset() {
        let mut filter = OnePoleFilter::<f32>::new(100.0, 44100.0);
        filter.process(1.0); // Build up state
        filter.reset();
        // After reset, output should be near zero immediately
        assert!(filter.process(0.0).abs() < 1e-6);
    }

    #[test]
    fn test_works_with_f64() {
        let mut filter = OnePoleFilter::<f64>::new(1000.0, 44100.0);
        let out = filter.process(0.5f64);
        assert!(out.is_finite());
    }
}
```

### Step 5: Write a doc comment for every public item

Every `pub fn`, `pub struct`, and `pub const` needs a doc comment explaining:
1. What it does
2. Any mathematical background relevant to DSP readers
3. Real-time safety notes (allocations? O(n) time?)
4. An example if the usage isn't obvious

### Step 6: Run checks before submitting

```bash
cargo fmt          # Format code
cargo clippy       # Lint
cargo test         # All tests must pass
cargo doc          # Documentation must build without warnings
```

---

## 7. Further reading

### Audio programming

- **[The Scientist and Engineer's Guide to Digital Signal Processing](http://www.dspguide.com/)**
  (free online) — The best mathematical foundation for DSP. Start with chapters 1–6.

- **[Designing Audio Effect Plugins in C++ — Will Pirkle](https://www.willpirkle.com/)** —
  Excellent practical guide. The C++ translates directly to Rust patterns.

- **[MusicDSP.org source archive](https://github.com/Music-DSP/MusicDSP-Archives)** —
  Annotated DSP algorithms. Useful reference for filter and effect implementations.

- **[Jatin Chowdhury's blog](https://jatinchowdhury18.medium.com/)** — Modern, practically-focused
  DSP articles on filters, non-linear processing, and physical modeling.

### Rust for audio

- **[nih-plug source code](https://github.com/robbert-vdh/nih-plug)** — A mature Rust audio plugin
  framework. Excellent reference for how a full plugin framework is structured around a DSP core.

- **[dasp crate](https://github.com/RustAudio/dasp)** — Another Rust DSP library. Useful for
  seeing alternative API designs and implementations.

- **[Rust Audio Discord](https://discord.gg/8qW6q29S)** — Active community of Rust audio developers.

### Rust language

- **[The Rust Book](https://doc.rust-lang.org/book/)** — Start here if Rust is new to you.
  Chapters 10 (generics/traits) and 10.3 (lifetimes) are directly relevant.

- **[Rust by Example](https://doc.rust-lang.org/rust-by-example/)** — More hands-on complement to
  the book. Good for quickly looking up how a specific feature works.

- **[The Rustonomicon](https://doc.rust-lang.org/nomicon/)** — Deep dive into unsafe Rust. Only
  needed if you're writing SIMD or FFI code.

### Audio formats and plugin standards

- **[CLAP plugin specification](https://cleveraudio.org/)** — Modern, open audio plugin standard.
  This library will eventually target CLAP via a framework layer.

- **[VST3 specification](https://steinbergmedia.github.io/vst3_doc/)** — The most widely supported
  plugin format. Understanding it helps clarify why `AudioBuffer` is designed the way it is.
