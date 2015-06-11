
use error::Error;
use portaudio::pa;
use portaudio::pa::Sample as PaSample;
use sample::{Sample, Wave};
use settings::{Channels, Settings, Frames, SampleHz};
use std::collections::VecDeque;
use std::marker::PhantomData;

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


/// A builder context for an Output sound stream.
pub struct Builder<O> {
    pub stream_params: SoundStream,
    pub output_params: StreamParams<O>,
}

/// An iterator of blocking output stream events.
pub struct BlockingStream<'a, O=Wave> where O: Sample + PaSample {
    /// Buffer the samples from the output until its length is equal to the buffer_length.
    buffer: VecDeque<O>,
    /// Buffer passed to the user for writing.
    user_buffer: Vec<O>,
    /// Number of channels.
    channels: Channels,
    /// Stream sample rate.
    sample_hz: SampleHz,
    /// Frames per buffer.
    frames: Frames,
    /// The port audio stream.
    stream: pa::Stream<O, O>,
    is_closed: bool,
    marker: PhantomData<&'a ()>,
}

/// Stream callback function type.
pub type Callback<O> = Box<FnMut(&mut[O], Settings, DeltaTimeSeconds, CallbackFlags) -> CallbackResult>;

/// A handle to the non-blocking output stream.
pub struct NonBlockingStream<O=Wave> where O: Sample + PaSample {
    /// The port audio stream.
    stream: pa::Stream<O, O>,
    /// Is the stream currently closed.
    is_closed: bool,
}

/// An event returned by the Blocking Stream.
#[derive(Debug)]
pub struct Event<'a, O: 'a>(pub &'a mut [O], pub Settings);

impl<O> Builder<O> where O: Sample + PaSample {

    /// Retrieve the flags, output stream parameters, sample rate and frames per buffer.
    fn unwrap_params(self) -> Result<PaParams, Error> {
        let Builder { stream_params, output_params } = self;
        let SoundStream { maybe_buffer_frequency, maybe_sample_hz, maybe_flags } = stream_params;

        // Retrieve any stream flags.
        let flags = maybe_flags.unwrap_or_else(|| StreamFlags::empty());

        // Construct the PortAudio output params from the sound stream ones.
        let output_params = {
            let idx = output_params.idx.unwrap_or_else(|| pa::device::get_default_output());
            let info = match pa::device::get_info(idx) {
                Ok(info) => info,
                Err(err) => return Err(Error::PortAudio(err)),
            };
            let channels = output_params.channel_count
                .map(|n| ::std::cmp::min(n, info.max_output_channels))
                .unwrap_or_else(|| ::std::cmp::min(2, info.max_output_channels));
            let sample_format = output_params.sample_format();
            let suggested_latency = output_params.suggested_latency
                .unwrap_or_else(|| info.default_low_output_latency);
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
            None => match pa::device::get_info(output_params.device) {
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

        Ok((flags, output_params, sample_hz, frames))
    }

    /// Launch a non-blocking output stream with the given callback!
    #[inline]
    pub fn run_callback(self, mut callback: Callback<O>) -> Result<NonBlockingStream<O>, Error> {

        // Initialize PortAudio.
        try!(pa::initialize().map_err(|err| Error::PortAudio(err)));

        let (flags, output_params, sample_hz, frames) = try!(self.unwrap_params());
        let channels = output_params.channel_count;

        // Here we construct our PortAudio stream.
        let mut stream = pa::Stream::new();

        // Remember the last time the callback was called so we can create the delta time.
        let mut maybe_last_time = None; 

        // Construct a wrapper function around our callback.
        let f = Box::new(move |_input: &[O],
                               output: &mut[O],
                               frames: u32,
                               time_info: &pa::StreamCallbackTimeInfo,
                               flags: pa::StreamCallbackFlags| -> pa::StreamCallbackResult {
            let settings = Settings {
                sample_hz: sample_hz as u32,
                frames: frames as u16,
                channels: channels as u16,
            };
            let dt = time_info.current_time - maybe_last_time.unwrap_or(time_info.current_time);
            maybe_last_time = Some(time_info.current_time);
            match callback(output, settings, dt, flags) {
                CallbackResult::Continue => pa::StreamCallbackResult::Continue,
                CallbackResult::Complete => pa::StreamCallbackResult::Complete,
                CallbackResult::Abort    => pa::StreamCallbackResult::Abort,
            }
        });

        // Here we open the stream.
        try!(stream.open(None, Some(&output_params), sample_hz, frames, flags, Some(f))
            .map_err(|err| Error::PortAudio(err)));

        // And now let's kick it off!
        try!(stream.start().map_err(|err| Error::PortAudio(err)));

        Ok(NonBlockingStream { stream: stream, is_closed: false })
    }

    /// Launch a blocking output stream!
    #[inline]
    pub fn run<'a>(self) -> Result<BlockingStream<'a, O>, Error> {

        // Initialize PortAudio.
        try!(pa::initialize().map_err(|err| Error::PortAudio(err)));

        let (flags, output_params, sample_hz, frames) = try!(self.unwrap_params());

        // Here we construct our PortAudio stream.
        let mut stream = pa::Stream::new();

        // Here we open the stream.
        try!(stream.open(None, Some(&output_params), sample_hz, frames, flags, None)
            .map_err(|err| Error::PortAudio(err)));

        // And now let's kick it off!
        try!(stream.start().map_err(|err| Error::PortAudio(err)));

        let channels = output_params.channel_count;
        let double_buffer_len = (frames as usize * channels as usize) * 2;
        let buffer_len = ::std::cmp::max(double_buffer_len, MINIMUM_BUFFER_RESERVATION);

        Ok(BlockingStream {
            buffer: VecDeque::with_capacity(buffer_len),
            user_buffer: Vec::with_capacity(frames as usize * channels as usize),
            stream: stream,
            channels: channels as u16,
            frames: frames as u16,
            sample_hz: sample_hz as u32,
            is_closed: false,
            marker: PhantomData,
        })
    }

}

impl<O> NonBlockingStream<O> where O: Sample + PaSample {

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

impl<O> Drop for NonBlockingStream<O> where O: Sample + PaSample {
    fn drop(&mut self) {
        if !self.is_closed {
            if let Err(err) = self.close() {
                println!("An error occurred while closing NonBlockingStream: {}", err);
            }
        }
    }
}

impl<'a, O> BlockingStream<'a, O> where O: Sample + PaSample {
    /// Close the stream and terminate PortAudio.
    pub fn close(&mut self) -> Result<(), Error> {
        self.is_closed = true;
        try!(self.stream.close().map_err(|err| Error::PortAudio(err)));
        try!(pa::terminate().map_err(|err| Error::PortAudio(err)));
        Ok(())
    }
}

impl<'a, O> Drop for BlockingStream<'a, O> where O: Sample + PaSample {
    fn drop(&mut self) {
        if !self.is_closed {
            if let Err(err) = self.close() {
                println!("An error occurred while closing BlockingStream: {}", err);
            }
        }
    }
}

impl<'a, O> Iterator for BlockingStream<'a, O> where O: Sample + PaSample {
    type Item = Event<'a, O>;

    fn next(&mut self) -> Option<Event<'a, O>> {
        use std::error::Error as StdError;
        use utils::take_front;

        let BlockingStream {
            ref mut buffer,
            ref mut user_buffer,
            ref mut stream,
            ref channels,
            ref frames,
            ref sample_hz,
            ..
        } = *self;

        let settings = Settings { channels: *channels, frames: *frames, sample_hz: *sample_hz };
        let buffer_size = settings.buffer_size();

        if user_buffer.len() > 0 {
            buffer.extend(user_buffer.iter().map(|&sample| sample));
            user_buffer.clear();
        }

        loop {

            // How many frames are available for writing on the output stream?
            let available_frames = match wait_for_stream(|| stream.get_stream_write_available()) {
                Ok(frames) => frames,
                Err(err) => {
                    println!("An error occurred while requesting the number of available \
                             frames for writing from the output stream: {}. BlockingStream will \
                             now exit the event loop.", StdError::description(&err));
                    return None;
                },
            };

            // How many frames do we have in our output_buffer so far?
            let output_buffer_frames = (buffer.len() / *channels as usize) as u32;

            // If there are frames available for writing and we have some to write, then write!
            if available_frames > 0 && output_buffer_frames > 0 {
                // If we have more than enough frames for writing, take them from the start of the buffer.
                let (write_buffer, write_frames) = if output_buffer_frames >= available_frames {
                    let out_samples = (available_frames * *channels as u32) as usize;
                    let write_buffer = take_front(buffer, out_samples);
                    (write_buffer, available_frames)
                }
                // Otherwise if we have less, just take what we can for now.
                else {
                    let len = buffer.len();
                    let write_buffer = take_front(buffer, len);
                    (write_buffer, output_buffer_frames)
                };
                if let Err(err) = stream.write(write_buffer, write_frames) {
                    println!("An error occurred while writing to the output stream: {}. \
                             BlockingStream will now exit the event loop.",
                             StdError::description(&err));
                    return None
                }
            }

            // If we need more frames, return a buffer for writing.
            if buffer.len() <= buffer.capacity() - buffer_size {
                use std::iter::repeat;
                // Start the slice just after the already filled samples.
                let start = user_buffer.len();
                // Extend the update buffer by the necessary number of frames.
                user_buffer.extend(repeat(O::zero()).take(buffer_size));
                // Here we obtain a mutable reference to the slice with the correct lifetime so
                // that we can return it via our `Event::Out`. Note: This means that a twisted,
                // evil person could do horrific things with this iterator by calling `.next()`
                // multiple times and storing aliasing mutable references to our output buffer,
                // HOWEVER - this is extremely unlikely to occur in practise as the api is designed
                // in a way that the reference is intended to die at the end of each loop before
                // `.next()` even gets called again.
                let slice = unsafe { ::std::mem::transmute(&mut user_buffer[start..]) };
                return Some(Event(slice, settings));
            }

        }

    }

}


