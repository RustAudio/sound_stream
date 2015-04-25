
use error::Error;
use portaudio::pa;
use portaudio::pa::Sample as PaSample;
use sample::Format as SampleFormat;
use sample::Sample;
use settings::Frames;
use std::marker::PhantomData;

pub mod duplex;
pub mod input;
pub mod output;

/// The size of the VecDeque reservation with headroom for overflowing samples.
pub const MINIMUM_BUFFER_RESERVATION: usize = 2048;

/// A builder context for a SoundStream.
pub struct SoundStream {
    maybe_buffer_frequency: Option<BufferFrequency>,
    maybe_sample_hz: Option<f64>,
    maybe_flags: Option<StreamFlags>,
}

/// Bit flags to be passed to the stream.
pub type StreamFlags = pa::StreamFlags;

/// Bit flags fed to the callback to indicate non-blocking stream behaviour.
pub type CallbackFlags = pa::StreamCallbackFlags;

/// Represents the update frequency.
enum BufferFrequency {
    Hz(f32),
    Frames(Frames),
}

/// Difference in time between Update events.
pub type DeltaTimeSeconds = f64;

/// To be returned by the callback that is run by the non-blocking streams.
#[derive(Copy, Clone, Debug)]
pub enum CallbackResult {
    /// Successfully finish and close the stream.
    Complete,
    /// Continue the stream.
    Continue,
    /// Abort the stream.
    Abort,
}

/// The params unwrapped by the input and output stream builders.
pub type PaParams = (StreamFlags, pa::StreamParameters, f64, u32);

/// The index of the device.
pub type Idx = pa::DeviceIndex;

/// A suggested amount of latency.
pub type Latency = pa::Time;

/// A type for building stream parameters.
pub struct StreamParams<S> {
    pub idx: Option<Idx>,
    pub channel_count: Option<i32>,
    pub sample_format: Option<SampleFormat>,
    pub suggested_latency: Option<Latency>,
    pub phantom_sample: PhantomData<S>,
}

impl SoundStream {

    /// Constructs the builder for a new SoundStream.
    #[inline]
    pub fn new() -> SoundStream {
        SoundStream {
            maybe_buffer_frequency: None,
            maybe_sample_hz: None,
            maybe_flags: None,
        }
    }

    /// Desired stream sample rate (samples per second). For a duplex stream, it is the sample rate
    /// for both the input and output streams.
    #[inline]
    pub fn sample_hz(self, sample_hz: f64) -> SoundStream {
        SoundStream { maybe_sample_hz: Some(sample_hz), ..self }
    }

    /// Flags indicating stream behaviour.
    #[inline]
    pub fn flags(self, flags: StreamFlags) -> SoundStream {
        SoundStream { maybe_flags: Some(flags), ..self }
    }

    /// Used to calculate the number of frames per buffer.
    #[inline]
    pub fn buffer_hz(self, hz: f32) -> SoundStream {
        assert!(hz > 0.0, "`update_hz` must be greater than 0.0, but you gave {:?}", hz);
        SoundStream { maybe_buffer_frequency: Some(BufferFrequency::Hz(hz)), ..self }
    }

    /// The number of frames per buffer of audio.
    #[inline]
    pub fn frames_per_buffer(self, frames: Frames) -> SoundStream {
        SoundStream { maybe_buffer_frequency: Some(BufferFrequency::Frames(frames)), ..self }
    }

    /// Custom input device.
    #[inline]
    pub fn input<I>(self, params: StreamParams<I>) -> input::Builder<I>
        where
            I: Sample + PaSample
    {
        input::Builder { stream_params: self, input_params: params }
    }

    /// Custom output device.
    #[inline]
    pub fn output<O>(self, params: StreamParams<O>) -> output::Builder<O>
        where
            O: Sample + PaSample
    {
        output::Builder { stream_params: self, output_params: params }
    }

    /// Duplex stream with given custom input and output devices.
    #[inline]
    pub fn duplex<I, O>(self,
                        input_params: StreamParams<I>,
                        output_params: StreamParams<O>) -> duplex::Builder<I, O>
        where
            I: Sample + PaSample,
            O: Sample + PaSample,
    {
        duplex::Builder {
            stream_params: self,
            input_params: input_params,
            output_params: output_params,
        }
    }

}

impl<S> StreamParams<S> {

    /// Construct a default StreamParams.
    pub fn new() -> StreamParams<S> {
        StreamParams {
            idx: None,
            channel_count: None,
            sample_format: None,
            suggested_latency: None,
            phantom_sample: PhantomData,
        }
    }

    /// Specify the index of the device to be used for the Stream.
    #[inline]
    pub fn device_idx(self, idx: Idx) -> StreamParams<S> {
        StreamParams { idx: Some(idx), ..self }
    }

    /// Request a number of channels for the Stream.
    #[inline]
    pub fn channels(self, channels: i32) -> StreamParams<S> {
        StreamParams { channel_count: Some(channels), ..self }
    }

    /// Return the sample format for the Stream.
    #[inline]
    pub fn sample_format(&self) -> pa::SampleFormat where S: pa::Sample {
        let s: S = ::std::default::Default::default();
        pa::Sample::sample_format(&s)
    }

    /// Suggest a latency to use for the stream.
    #[inline]
    pub fn suggest_latency(self, latency: Latency) -> StreamParams<S> {
        StreamParams { suggested_latency: Some(latency), ..self }
    }

}

/// Wait for the given stream to become ready for reading/writing.
fn wait_for_stream<F>(f: F) -> Result<u32, Error>
    where
        F: Fn() -> Result<pa::StreamAvailable, pa::Error>,
{
    loop {
        match f() {
            Ok(available) => match available {
                pa::StreamAvailable::Frames(frames) => return Ok(frames as u32),
                pa::StreamAvailable::InputOverflowed => println!("Input stream has overflowed"),
                pa::StreamAvailable::OutputUnderflowed => println!("Output stream has underflowed"),
            },
            Err(err) => return Err(Error::PortAudio(err)),
        }
    }
}

