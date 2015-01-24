//!
//! The primary reason for the AudioBuffer trait is to be able to offer the option of a
//! Zeroed-by-default, stack-based array for the output buffer.
//!
//! The AudioBuffer trait is implemented for several different container types including:
//! - Vec<S>
//! - [S, ..2]
//! - [S, ..4]
//! - [S, ..8]
//! - [S, ..16]
//! - [S, ..32]
//! - [S, ..64]
//! - [S, ..128]
//! - [S, ..256]
//! - [S, ..512]
//! - [S, ..1024]
//! - [S, ..2048]
//! - [S, ..4096]
//! - [S, ..8192]
//! - [S, ..16384]

use portaudio::pa::Sample;
use std::num::FromPrimitive;

/// A trait to be implemented by any Buffer used for audio processing in sound_stream.
/// This is primarily implemented for fixed-size arrays where len is a power of 2.
pub trait AudioBuffer {
    type Sample: Sample = f32;
    /// Return a Zeroed AudioBuffer.
    fn zeroed(len: usize) -> Self;
    /// Clone the AudioBuffer as a Vec.
    fn clone_as_vec(&self) -> Vec<<Self as AudioBuffer>::Sample>;
}

impl<S> AudioBuffer for Vec<S> where S: Sample {
    type Sample = S;
    fn zeroed(len: usize) -> Vec<S> { 
        (0..len).map(|_| FromPrimitive::from_u64(0).unwrap()).collect()
    }
    fn clone_as_vec(&self) -> Vec<S> { self.clone() }
}

#[macro_escape]
macro_rules! impl_audio_buffer(
    ($len:expr) => (
        impl<S> AudioBuffer for [S; $len] where S: Sample {
            type Sample = S;
            #[inline]
            fn zeroed(_len: usize) -> [S; $len] { [FromPrimitive::from_u64(0).unwrap(); $len] }
            #[inline]
            fn clone_as_vec(&self) -> Vec<S> { (0us..$len).map(|idx| self[idx]).collect() }
        }
    )
);

impl_audio_buffer!(2);
impl_audio_buffer!(4);
impl_audio_buffer!(8);
impl_audio_buffer!(16);
impl_audio_buffer!(32);
impl_audio_buffer!(64);
impl_audio_buffer!(128);
impl_audio_buffer!(256);
impl_audio_buffer!(512);
impl_audio_buffer!(1024);
impl_audio_buffer!(2048);
impl_audio_buffer!(4096);
impl_audio_buffer!(8192);
impl_audio_buffer!(16384);

