
use error::Error;
use portaudio::pa;
use portaudio::pa::Sample as PaSample;
use sample::{Sample, Wave};
use settings::{Channels, Settings, Frames, SampleHz};
use std::collections::VecDeque;

use super::{
    BufferFrequency,
    CallbackFlags,
    CallbackResult,
    DeltaTimeSeconds,
    MINIMUM_BUFFER_RESERVATION,
    PaParams,
    SoundStream,
    StreamFlags,
    StreamParams,
    wait_for_stream,
};


/// A builder context for an Input sound stream.
pub struct Builder<I> {
    pub stream_params: SoundStream,
    pub input_params: StreamParams<I>,
}

/// An iterator of blocking input stream events.
pub struct BlockingStream<I=Wave> where I: Sample + PaSample {
    /// Buffer the samples from the input until its length is equal to the buffer_length.
    buffer: VecDeque<I>,
    /// Number of input channels.
    channels: Channels,
    /// Stream sample rate.
    sample_hz: SampleHz,
    /// Frames per buffer.
    frames: Frames,
    /// The port audio stream.
    stream: pa::Stream<I, I>,
    is_closed: bool,
}

/// Stream callback function type.
pub type Callback<I> =
    Box<FnMut(&[I], Settings, DeltaTimeSeconds, CallbackFlags) -> CallbackResult>;

/// A handle to the non-blocking input stream.
pub struct NonBlockingStream<I=Wave> where I: Sample + PaSample {
    /// The port audio stream.
    stream: pa::Stream<I, I>,
    /// Is the stream currently closed.
    is_closed: bool,
}

/// An event returned by the Blocking Stream.
#[derive(Clone, Debug)]
pub struct Event<I>(pub Vec<I>, pub Settings);

impl<I> Builder<I> where I: Sample + PaSample {

    /// Retrieve the flags, input stream parameters, sample rate and frames per buffer.
    fn unwrap_params(self) -> Result<PaParams, Error> {
        let Builder { stream_params, input_params } = self;
        let SoundStream { maybe_buffer_frequency, maybe_sample_hz, maybe_flags } = stream_params;

        // Retrieve any stream flags.
        let flags = maybe_flags.unwrap_or_else(|| StreamFlags::empty());

        // Construct the PortAudio input params from the sound stream ones.
        let input_params = {
            let idx = input_params.idx.unwrap_or_else(|| pa::device::get_default_input());
            let info = match pa::device::get_info(idx) {
                Ok(info) => info,
                Err(err) => return Err(Error::PortAudio(err)),
            };
            let channels = input_params.channel_count
                .map(|n| ::std::cmp::min(n, info.max_input_channels))
                .unwrap_or_else(|| ::std::cmp::min(2, info.max_input_channels));
            let sample_format = input_params.sample_format();
            let suggested_latency = input_params.suggested_latency
                .unwrap_or_else(|| info.default_low_input_latency);
            pa::StreamParameters {
                device: idx,
                channel_count: channels,
                sample_format: sample_format,
                suggested_latency: suggested_latency,
            }
        };

        // Determine the sample rate.
        let sample_hz = match maybe_sample_hz {
            Some(sample_hz) => sample_hz,
            None => match pa::device::get_info(input_params.device) {
                Ok(info) => info.default_sample_rate,
                Err(err) => return Err(Error::PortAudio(err)),
            },
        };

        // Determine the closest number of frames per buffer to the requested rate.
        let frames = match maybe_buffer_frequency {
            Some(BufferFrequency::Frames(frames)) => frames as u32,
            Some(BufferFrequency::Hz(hz)) => (sample_hz as f32 / hz).round() as u32,
            None => 0,
        };

        Ok((flags, input_params, sample_hz, frames))
    }

    /// Launch a non-blocking input stream with the given callback!
    #[inline]
    pub fn run_callback(self, mut callback: Callback<I>) -> Result<NonBlockingStream<I>, Error>
        where I: 'static,
    {

        // Initialize PortAudio.
        try!(pa::initialize().map_err(|err| Error::PortAudio(err)));

        let (flags, input_params, sample_hz, frames) = try!(self.unwrap_params());
        let channels = input_params.channel_count;

        // Here we construct our PortAudio stream.
        let mut stream = pa::Stream::new();

        // Remember the last time the callback was called so we can create the delta time.
        let mut maybe_last_time = None; 

        // Construct a wrapper function around our callback.
        let f = Box::new(move |input: &[I],
                               _output: &mut[I],
                               frames: u32,
                               time_info: &pa::StreamCallbackTimeInfo,
                               flags: pa::StreamCallbackFlags| -> pa::StreamCallbackResult
        {
            let settings = Settings {
                sample_hz: sample_hz as u32,
                frames: frames as u16,
                channels: channels as u16,
            };
            let dt = time_info.current_time - maybe_last_time.unwrap_or(time_info.current_time);
            maybe_last_time = Some(time_info.current_time);
            match callback(input, settings, dt, flags) {
                CallbackResult::Continue => pa::StreamCallbackResult::Continue,
                CallbackResult::Complete => pa::StreamCallbackResult::Complete,
                CallbackResult::Abort    => pa::StreamCallbackResult::Abort,
            }
        });

        // Here we open the stream.
        try!(stream.open(Some(&input_params), None, sample_hz, frames, flags, Some(f))
            .map_err(|err| Error::PortAudio(err)));

        // And now let's kick it off!
        try!(stream.start().map_err(|err| Error::PortAudio(err)));

        Ok(NonBlockingStream { stream: stream, is_closed: false })
    }

    /// Launch a blocking input stream!
    #[inline]
    pub fn run(self) -> Result<BlockingStream<I>, Error>
        where I: 'static,
    {

        // Initialize PortAudio.
        try!(pa::initialize().map_err(|err| Error::PortAudio(err)));

        let (flags, input_params, sample_hz, frames) = try!(self.unwrap_params());

        // Here we construct our PortAudio stream.
        let mut stream = pa::Stream::new();

        // Here we open the stream.
        try!(stream.open(Some(&input_params), None, sample_hz, frames, flags, None)
            .map_err(|err| Error::PortAudio(err)));

        // And now let's kick it off!
        try!(stream.start().map_err(|err| Error::PortAudio(err)));

        let channels = input_params.channel_count;
        let double_buffer_len = (frames as usize * channels as usize) * 2;
        let buffer_len = ::std::cmp::max(double_buffer_len, MINIMUM_BUFFER_RESERVATION);

        Ok(BlockingStream {
            buffer: VecDeque::with_capacity(buffer_len),
            stream: stream,
            channels: channels as u16,
            frames: frames as u16,
            sample_hz: sample_hz as u32,
            is_closed: false,
        })
    }

}

impl<I> NonBlockingStream<I> where I: Sample + PaSample {

    /// Close the stream and terminate PortAudio.
    pub fn close(&mut self) -> Result<(), Error> {
        self.is_closed = true;
        try!(self.stream.close().map_err(|err| Error::PortAudio(err)));
        try!(pa::terminate().map_err(|err| Error::PortAudio(err)));
        Ok(())
    }

    /// Check whether or not the stream is currently active.
    pub fn is_active(&self) -> Result<bool, Error> {
        self.stream.is_active().map_err(|err| Error::PortAudio(err))
    }

}

impl<I> Drop for NonBlockingStream<I> where I: Sample + PaSample {
    fn drop(&mut self) {
        if !self.is_closed {
            if let Err(err) = self.close() {
                println!("An error occurred while closing NonBlockingStream: {}", err);
            }
        }
    }
}

impl<I> BlockingStream<I> where I: Sample + PaSample {
    /// Close the stream and terminate PortAudio.
    pub fn close(&mut self) -> Result<(), Error> {
        self.is_closed = true;
        try!(self.stream.close().map_err(|err| Error::PortAudio(err)));
        try!(pa::terminate().map_err(|err| Error::PortAudio(err)));
        Ok(())
    }
}

impl<I> Drop for BlockingStream<I> where I: Sample + PaSample {
    fn drop(&mut self) {
        if !self.is_closed {
            if let Err(err) = self.close() {
                println!("An error occurred while closing BlockingStream: {}", err);
            }
        }
    }
}

impl<I> Iterator for BlockingStream<I> where I: Sample + PaSample {
    type Item = Event<I>;

    fn next(&mut self) -> Option<Event<I>> {

        let BlockingStream {
            ref mut buffer,
            ref mut stream,
            ref channels,
            ref frames,
            ref sample_hz,
            ..
        } = *self;

        let settings = Settings { channels: *channels, frames: *frames, sample_hz: *sample_hz };
        let buffer_size = settings.buffer_size();

        loop {
            use std::error::Error as StdError;
            use utils::take_front;

            // If we have the requested number of frames, return them in an Event.
            if buffer.len() >= buffer_size {
                let event_buffer = take_front(buffer, buffer_size);
                return Some(Event(event_buffer, settings));
            }

            // How many frames are available on the input stream?
            let available_frames = match wait_for_stream(|| stream.get_stream_read_available()) {
                Ok(frames) => frames,
                Err(err) => {
                    println!("An error occurred while requesting the number of available \
                             frames for reading from the input stream: {}. BlockingStream will \
                             now exit the event loop.", StdError::description(&err));
                    return None;
                },
            };

            // If there are frames available and we have room in the buffer, take them.
            if available_frames > 0 && buffer.capacity() >= buffer.len() + available_frames as usize {
                match stream.read(available_frames) {
                    Ok(input_samples) => buffer.extend(input_samples.into_iter()),
                    Err(err) => {
                        println!("An error occurred while reading from the input stream: {}. \
                                 BlockingStream will now exit the event loop.",
                                 StdError::description(&err));
                        return None;
                    },
                }
            }

        }

    }

}
