//! Sample trait for generic audio sample types.

use std::fmt::Debug;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// Trait for audio sample types (typically f32 or f64).
///
/// This trait provides the mathematical operations and constants
/// needed for DSP processing. By being generic over `Sample`, we can
/// write code once that works for both f32 and f64 precision.
///
/// # Example
///
/// ```rust
/// use dsp::core::Sample;
///
/// fn amplify<T: Sample>(sample: T, gain: T) -> T {
///     sample * gain
/// }
///
/// let result_f32 = amplify(0.5f32, 2.0f32);
/// let result_f64 = amplify(0.5f64, 2.0f64);
/// ```
pub trait Sample:
    Copy
    + Clone
    + Debug
    + Default
    + PartialOrd
    + PartialEq
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Neg<Output = Self>
    + AddAssign
    + SubAssign
    + MulAssign
    + DivAssign
    + Send
    + Sync
    + 'static
{
    /// Zero value
    const ZERO: Self;

    /// One value
    const ONE: Self;

    /// Two value
    const TWO: Self;

    /// Pi constant (π)
    const PI: Self;

    /// Tau constant (2π)
    const TAU: Self;

    /// Euler's number (e)
    const E: Self;

    /// Square root of 2
    const SQRT_2: Self;

    /// 1 / Square root of 2 (useful for normalization)
    const FRAC_1_SQRT_2: Self;

    /// Convert from f64
    fn from_f64(v: f64) -> Self;

    /// Convert to f64
    fn to_f64(self) -> f64;

    /// Convert from f32
    fn from_f32(v: f32) -> Self;

    /// Convert to f32
    fn to_f32(self) -> f32;

    /// Convert from usize
    fn from_usize(v: usize) -> Self;

    /// Absolute value
    fn abs(self) -> Self;

    /// Clamp value between min and max
    fn clamp(self, min: Self, max: Self) -> Self;

    /// Minimum of two values
    fn min(self, other: Self) -> Self;

    /// Maximum of two values
    fn max(self, other: Self) -> Self;

    /// Square root
    fn sqrt(self) -> Self;

    /// Power function
    fn powf(self, n: Self) -> Self;

    /// Exponential function (e^x)
    fn exp(self) -> Self;

    /// Natural logarithm
    fn ln(self) -> Self;

    /// Base-2 logarithm
    fn log2(self) -> Self;

    /// Base-10 logarithm
    fn log10(self) -> Self;

    /// Sine function
    fn sin(self) -> Self;

    /// Cosine function
    fn cos(self) -> Self;

    /// Tangent function
    fn tan(self) -> Self;

    /// Hyperbolic tangent (useful for soft clipping)
    fn tanh(self) -> Self;

    /// Arc tangent
    fn atan(self) -> Self;

    /// Two-argument arc tangent
    fn atan2(self, other: Self) -> Self;

    /// Floor function
    fn floor(self) -> Self;

    /// Ceiling function
    fn ceil(self) -> Self;

    /// Round to nearest integer
    fn round(self) -> Self;

    /// Truncate to integer
    fn trunc(self) -> Self;

    /// Fractional part
    fn fract(self) -> Self;

    /// Check if the value is finite (not NaN or infinite)
    fn is_finite(self) -> bool;

    /// Check if the value is NaN
    fn is_nan(self) -> bool;

    /// Signum function (-1, 0, or 1)
    fn signum(self) -> Self;

    /// Copy the sign from another value
    fn copysign(self, sign: Self) -> Self;
}

impl Sample for f32 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
    const TWO: Self = 2.0;
    const PI: Self = std::f32::consts::PI;
    const TAU: Self = std::f32::consts::TAU;
    const E: Self = std::f32::consts::E;
    const SQRT_2: Self = std::f32::consts::SQRT_2;
    const FRAC_1_SQRT_2: Self = std::f32::consts::FRAC_1_SQRT_2;

    #[inline]
    fn from_f64(v: f64) -> Self {
        v as f32
    }

    #[inline]
    fn to_f64(self) -> f64 {
        self as f64
    }

    #[inline]
    fn from_f32(v: f32) -> Self {
        v
    }

    #[inline]
    fn to_f32(self) -> f32 {
        self
    }

    #[inline]
    fn from_usize(v: usize) -> Self {
        v as f32
    }

    #[inline]
    fn abs(self) -> Self {
        self.abs()
    }

    #[inline]
    fn clamp(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }

    #[inline]
    fn min(self, other: Self) -> Self {
        self.min(other)
    }

    #[inline]
    fn max(self, other: Self) -> Self {
        self.max(other)
    }

    #[inline]
    fn sqrt(self) -> Self {
        self.sqrt()
    }

    #[inline]
    fn powf(self, n: Self) -> Self {
        self.powf(n)
    }

    #[inline]
    fn exp(self) -> Self {
        self.exp()
    }

    #[inline]
    fn ln(self) -> Self {
        self.ln()
    }

    #[inline]
    fn log2(self) -> Self {
        self.log2()
    }

    #[inline]
    fn log10(self) -> Self {
        self.log10()
    }

    #[inline]
    fn sin(self) -> Self {
        self.sin()
    }

    #[inline]
    fn cos(self) -> Self {
        self.cos()
    }

    #[inline]
    fn tan(self) -> Self {
        self.tan()
    }

    #[inline]
    fn tanh(self) -> Self {
        self.tanh()
    }

    #[inline]
    fn atan(self) -> Self {
        self.atan()
    }

    #[inline]
    fn atan2(self, other: Self) -> Self {
        self.atan2(other)
    }

    #[inline]
    fn floor(self) -> Self {
        self.floor()
    }

    #[inline]
    fn ceil(self) -> Self {
        self.ceil()
    }

    #[inline]
    fn round(self) -> Self {
        self.round()
    }

    #[inline]
    fn trunc(self) -> Self {
        self.trunc()
    }

    #[inline]
    fn fract(self) -> Self {
        self.fract()
    }

    #[inline]
    fn is_finite(self) -> bool {
        self.is_finite()
    }

    #[inline]
    fn is_nan(self) -> bool {
        self.is_nan()
    }

    #[inline]
    fn signum(self) -> Self {
        self.signum()
    }

    #[inline]
    fn copysign(self, sign: Self) -> Self {
        self.copysign(sign)
    }
}

impl Sample for f64 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
    const TWO: Self = 2.0;
    const PI: Self = std::f64::consts::PI;
    const TAU: Self = std::f64::consts::TAU;
    const E: Self = std::f64::consts::E;
    const SQRT_2: Self = std::f64::consts::SQRT_2;
    const FRAC_1_SQRT_2: Self = std::f64::consts::FRAC_1_SQRT_2;

    #[inline]
    fn from_f64(v: f64) -> Self {
        v
    }

    #[inline]
    fn to_f64(self) -> f64 {
        self
    }

    #[inline]
    fn from_f32(v: f32) -> Self {
        v as f64
    }

    #[inline]
    fn to_f32(self) -> f32 {
        self as f32
    }

    #[inline]
    fn from_usize(v: usize) -> Self {
        v as f64
    }

    #[inline]
    fn abs(self) -> Self {
        self.abs()
    }

    #[inline]
    fn clamp(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }

    #[inline]
    fn min(self, other: Self) -> Self {
        self.min(other)
    }

    #[inline]
    fn max(self, other: Self) -> Self {
        self.max(other)
    }

    #[inline]
    fn sqrt(self) -> Self {
        self.sqrt()
    }

    #[inline]
    fn powf(self, n: Self) -> Self {
        self.powf(n)
    }

    #[inline]
    fn exp(self) -> Self {
        self.exp()
    }

    #[inline]
    fn ln(self) -> Self {
        self.ln()
    }

    #[inline]
    fn log2(self) -> Self {
        self.log2()
    }

    #[inline]
    fn log10(self) -> Self {
        self.log10()
    }

    #[inline]
    fn sin(self) -> Self {
        self.sin()
    }

    #[inline]
    fn cos(self) -> Self {
        self.cos()
    }

    #[inline]
    fn tan(self) -> Self {
        self.tan()
    }

    #[inline]
    fn tanh(self) -> Self {
        self.tanh()
    }

    #[inline]
    fn atan(self) -> Self {
        self.atan()
    }

    #[inline]
    fn atan2(self, other: Self) -> Self {
        self.atan2(other)
    }

    #[inline]
    fn floor(self) -> Self {
        self.floor()
    }

    #[inline]
    fn ceil(self) -> Self {
        self.ceil()
    }

    #[inline]
    fn round(self) -> Self {
        self.round()
    }

    #[inline]
    fn trunc(self) -> Self {
        self.trunc()
    }

    #[inline]
    fn fract(self) -> Self {
        self.fract()
    }

    #[inline]
    fn is_finite(self) -> bool {
        self.is_finite()
    }

    #[inline]
    fn is_nan(self) -> bool {
        self.is_nan()
    }

    #[inline]
    fn signum(self) -> Self {
        self.signum()
    }

    #[inline]
    fn copysign(self, sign: Self) -> Self {
        self.copysign(sign)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(f32::ZERO, 0.0);
        assert_eq!(f32::ONE, 1.0);
        assert_eq!(f64::ZERO, 0.0);
        assert_eq!(f64::ONE, 1.0);
    }

    #[test]
    fn test_conversions() {
        let val_f64 = 3.14159265359;
        let val_f32 = f32::from_f64(val_f64);

        assert!((val_f32 - 3.14159265).abs() < 0.00001);
        assert!((val_f32.to_f64() - val_f64).abs() < 0.0001);
    }

    #[test]
    fn test_math_operations() {
        let x = 2.0f32;
        assert_eq!(x.sqrt(), 1.4142135623730951f32);
        assert_eq!(x.powf(3.0), 8.0);

        let y = f32::PI / 2.0;
        assert!((y.sin() - 1.0).abs() < 0.00001);
    }

    #[test]
    fn test_clamp() {
        assert_eq!(5.0f32.clamp(0.0, 10.0), 5.0);
        assert_eq!((-5.0f32).clamp(0.0, 10.0), 0.0);
        assert_eq!(15.0f32.clamp(0.0, 10.0), 10.0);
    }

    #[test]
    fn test_generic_function() {
        fn amplify<T: Sample>(sample: T, gain: T) -> T {
            sample * gain
        }

        assert_eq!(amplify(0.5f32, 2.0f32), 1.0f32);
        assert_eq!(amplify(0.5f64, 2.0f64), 1.0f64);
    }
}
