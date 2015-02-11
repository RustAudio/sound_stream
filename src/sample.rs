//!
//!  sample.rs
//!
//!  Created by Mitchell Nordine at 07:44PM on February 11, 2015.
//!
//!

/// Represents a sample from a Wave between -1.0 and 1.0.
pub type Wave = f32;
/// Represents a Wave amplitude between 0.0 and 1.0.
pub type Amplitude = f32;

/// A trait for working generically across different sample types.
pub trait Sample:
    Copy +
    Clone +
    ::std::default::Default +
    ::std::fmt::Debug +
    PartialOrd +
    PartialEq +
    ::std::ops::Add<Output=Self> +
    ::std::ops::Sub<Output=Self> +
    ::std::ops::Mul<Output=Self> +
    ::std::ops::Div<Output=Self> +
    ::std::ops::Rem<Output=Self> +
    ::std::num::ToPrimitive +
    ::std::num::FromPrimitive
{

    /// Construct a sample from a wave sample between -1. and 1.
    fn from_wave(Wave) -> Self;

    /// Convert to a wave sample between -1. and 1.
    fn to_wave(self) -> Wave;

    /// Multiply by a given amplitude.
    #[inline]
    fn mul_amp(self, amp: f32) -> Self {
        Sample::from_wave(self.to_wave() * amp)
    }

    /// Construct a sample from an arbitrary Sample type.
    #[inline]
    fn from_sample<S: Sample>(sample: S) -> Self {
        Sample::from_wave(sample.to_wave())
    }

    /// Construct an arbitrary sample type from a sample of this Self type.
    #[inline]
    fn to_sample<S: Sample>(self) -> S {
        Sample::from_wave(self.to_wave())
    }

    /// A silent sample.
    #[inline]
    fn zero() -> Self { ::std::default::Default::default() }

}

// FLOATING POINT NUMBERS.

impl Sample for f64 {
    #[inline]
    fn from_wave(wave: Wave) -> f64 { wave as f64 }
    #[inline]
    fn to_wave(self) -> Wave { self as f32 }
}

impl Sample for f32 {
    #[inline]
    fn from_wave(wave: Wave) -> f32 { wave }
    #[inline]
    fn to_wave(self) -> Wave { self }
}

// SIGNED INTEGERS.

impl Sample for i32 {
    #[inline]
    fn from_wave(wave: Wave) -> i32 {
        const MAX: Wave = ::std::i32::MAX as Wave;
        (MAX * wave) as i32
    }
    #[inline]
    fn to_wave(self) -> Wave {
        const MAX: Wave = ::std::i32::MAX as Wave;
        self as Wave / MAX
    }
}

impl Sample for i16 {
    #[inline]
    fn from_wave(wave: Wave) -> i16 {
        const MAX: Wave = ::std::i16::MAX as Wave;
        (MAX * wave) as i16
    }
    #[inline]
    fn to_wave(self) -> Wave {
        const MAX: Wave = ::std::i16::MAX as Wave;
        self as Wave / MAX
    }
}

impl Sample for i8 {
    #[inline]
    fn from_wave(wave: Wave) -> i8 {
        const MAX: Wave = ::std::i8::MAX as Wave;
        (MAX * wave) as i8
    }
    #[inline]
    fn to_wave(self) -> Wave {
        const MAX: Wave = ::std::i8::MAX as Wave;
        self as Wave / MAX
    }
}

// UNSIGNED INTEGERS.

impl Sample for u32 {
    #[inline]
    fn from_wave(wave: Wave) -> u32 {
        const HALF_MAX: Wave = (::std::u32::MAX / 2) as Wave;
        (HALF_MAX + HALF_MAX * wave) as u32
    }
    #[inline]
    fn to_wave(self) -> Wave {
        const MAX: Wave = ::std::u32::MAX as Wave;
        (self as Wave / MAX) * 2.0 - 1.0
    }
}

impl Sample for u16 {
    #[inline]
    fn from_wave(wave: Wave) -> u16 {
        const HALF_MAX: Wave = (::std::u16::MAX / 2) as Wave;
        (HALF_MAX + HALF_MAX * wave) as u16
    }
    #[inline]
    fn to_wave(self) -> Wave {
        const MAX: Wave = ::std::u16::MAX as Wave;
        (self as Wave / MAX) * 2.0 - 1.0
    }
}

impl Sample for u8 {
    #[inline]
    fn from_wave(wave: Wave) -> u8 {
        const HALF_MAX: Wave = (::std::u8::MAX / 2) as Wave;
        (HALF_MAX + HALF_MAX * wave) as u8
    }
    #[inline]
    fn to_wave(self) -> Wave {
        const MAX: Wave = ::std::u8::MAX as Wave;
        (self as Wave / MAX) * 2.0 - 1.0
    }
}

