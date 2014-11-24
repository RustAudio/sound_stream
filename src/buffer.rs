//! The primary reason for the AudioBuffer trait was to be able to offer the option of
//! a Zeroed-by-default, stack-based array for the output buffer.
//!
//! The AudioBuffer trait is implemented for several different container types including:
//! - Vec<T>
//! - [f32, ..2]
//! - [f32, ..4]
//! - [f32, ..8]
//! - [f32, ..16]
//! - [f32, ..32]
//! - [f32, ..64]
//! - [f32, ..128]
//! - [f32, ..256]
//! - [f32, ..512]
//! - [f32, ..1024]
//! - [f32, ..2048]
//! - [f32, ..4096]
//! - [f32, ..8192]
//! - [f32, ..16384]

use portaudio::pa::Sample;
use std::slice::Items;
use std::slice::MutItems;
use settings::Settings;

/// A trait to be implemented by any Buffer used for audio processing in sound_stream.
/// This is primarily implemented for fixed-size arrays where len is a power of 2.
pub trait AudioBuffer<S> where S: Sample {
    /// Return the value at the given index.
    fn val(&self, idx: uint) -> S;
    /// Return an immutable reference to the value at the given index.
    fn get(&self, idx: uint) -> &S;
    /// Return a mutable reference to the value at the given index.
    fn get_mut(&mut self, idx: uint) -> &mut S;
    /// Return the AudioBuffer as a slice.
    fn as_slice(&self) -> &[S];
    /// Return the AudioBuffer as a mutable slice.
    fn as_mut_slice(&mut self) -> &mut [S];
    /// Return an immutable Iterator over the values.
    fn iter<'a>(&'a self) -> Items<'a, S>;
    /// Return a mutable Iterator over the values.
    fn iter_mut<'a>(&'a mut self) -> MutItems<'a, S>;
    /// Return a AudioBuffer full of the given value.
    fn from_elem(val: S) -> Self;
    /// Return a Zeroed AudioBuffer.
    fn zeroed() -> Self;
    /// Return the length of the AudioBuffer.
    fn len(&self) -> uint;
    /// Return the mono soundstream settings for this type.
    fn mono_settings(samples_per_sec: u32) -> Settings;
    /// Return the stereo soundstream settings for this type.
    fn stereo_settings(samples_per_sec: u32) -> Settings;
    /// Create a AudioBuffer from a Vec.
    #[inline]
    fn from_vec(vec: &Vec<S>) -> Self {
        let mut buffer: Self = AudioBuffer::zeroed();
        for i in range(0u, buffer.len()) {
            *buffer.get_mut(i) = vec[i];
        }
        buffer
    }
    /// Create a Vec from a AudioBuffer.
    fn clone_as_vec(&self) -> Vec<S> {
        Vec::from_fn(self.len(), |idx| self.val(idx))
    }
}

#[macro_escape]
macro_rules! impl_audio_buffer(
    ($buffer:ty, $len:expr) => (

        impl AudioBuffer<$buffer> for [$buffer, ..$len] {
            #[inline]
            fn val(&self, idx: uint) -> $buffer { self[idx] }
            #[inline]
            fn get(&self, idx: uint) -> &$buffer { &self[idx] }
            #[inline]
            fn get_mut(&mut self, idx: uint) -> &mut $buffer { &mut self[idx] }
            #[inline]
            fn as_slice(&self) -> &[$buffer] { self.as_slice() }
            #[inline]
            fn as_mut_slice(&mut self) -> &mut [$buffer] { self.as_mut_slice() }
            #[inline]
            fn iter<'a>(&'a self) -> Items<'a, $buffer> { self.as_slice().iter() }
            #[inline]
            fn iter_mut<'a>(&'a mut self) -> MutItems<'a, $buffer> { self.as_mut_slice().iter_mut() }
            #[inline]
            fn from_elem(val: $buffer) -> [$buffer, ..$len] { [val, ..$len] }
            #[inline]
            fn zeroed() -> [$buffer, ..$len] { [0.0, ..$len] }
            #[inline]
            fn len(&self) -> uint { $len }
            #[inline]
            fn mono_settings(samples_per_sec: u32) -> Settings {
                Settings::new(samples_per_sec, $len, 1)
            }
            #[inline]
            fn stereo_settings(samples_per_sec: u32) -> Settings {
                Settings::new(samples_per_sec, $len / 2, 2)
            }
        }

    )
)

impl_audio_buffer!(f32, 2)
impl_audio_buffer!(f32, 4)
impl_audio_buffer!(f32, 8)
impl_audio_buffer!(f32, 16)
impl_audio_buffer!(f32, 32)
impl_audio_buffer!(f32, 64)
impl_audio_buffer!(f32, 128)
impl_audio_buffer!(f32, 256)
impl_audio_buffer!(f32, 512)
impl_audio_buffer!(f32, 1024)
impl_audio_buffer!(f32, 2048)
impl_audio_buffer!(f32, 4096)
impl_audio_buffer!(f32, 8192)
impl_audio_buffer!(f32, 16384)

