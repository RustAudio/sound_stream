
use error::Error;
use portaudio::pa;
use portaudio::pa::Sample as PaSample;
use sample::{Sample, Wave};
use settings::{Channels, Settings, Frames, SampleHz};
use std::collections::VecDeque;
use std::marker::PhantomData;
use utils::take_front;

use super::{
    BufferFrequency,
    CallbackFlags,
    CallbackResult,
    DeltaTimeSeconds,
    MINIMUM_BUFFER_RESERVATION,
    SoundStream,
    StreamFlags,
    StreamParams,
    wait_for_stream,
};


/// A builder context for a duplex sound stream.
pub struct Builder<I, O> {
    pub stream_params: SoundStream,
    pub input_params: StreamParams<I>,
    pub output_params: StreamParams<O>,
}


/// An iterator of blocking stream events.
pub struct BlockingStream<'a, I=Wave, O=Wave>
    where
        I: Sample + PaSample,
        O: Sample + PaSample,
{
    /// Buffer the samples from the input until its length is equal to the buffer_length.
    input_buffer: VecDeque<I>,
    /// Store samples in this until there is enough to write to the output stream.
    output_buffer: VecDeque<O>,
    /// A buffer for retrieving samples from the user for writing.
    user_buffer: Vec<O>,
    /// Number of input channels.
    in_channels: Channels,
    /// Number of output channels.
    out_channels: Channels,
    /// Stream sample rate.
    sample_hz: SampleHz,
    /// Frames per buffer.
    frames: Frames,
    /// The last event that has occured.
    last_event: Option<LastEvent>,
    /// The port audio stream.
    stream: pa::Stream<I, O>,
    is_closed: bool,
    marker: PhantomData<&'a ()>,
}


/// Stream callback function type.
pub type Callback<I, O> =
    Box<FnMut(&[I], Settings, &mut[O], Settings, DeltaTimeSeconds, CallbackFlags) -> CallbackResult>;

/// A handle to the non-blocking duplex stream.
pub struct NonBlockingStream<I=Wave, O=Wave>
    where
        I: Sample + PaSample,
        O: Sample + PaSample,
{
    /// The port audio stream.
    stream: pa::Stream<I, O>,
    /// Whether or not the stream is currently closed.
    is_closed: bool,
}

/// An event to be returned by the BlockingStream.
#[derive(Debug)]
pub enum Event<'a, I=Wave, O=Wave> where O: 'a {
    /// Audio awaits on the stream's input buffer.
    In(Vec<I>, Settings),
    /// The stream's output buffer is ready to be written to.
    Out(&'a mut [O], Settings),
}

/// Represents the current state of the BlockingStream.
#[derive(Clone, Copy)]
pub enum LastEvent {
    In,
    Out,
    Update,
}

/// The params to be unwrapped after the building is complete.
type PaParams = (StreamFlags, pa::StreamParameters, pa::StreamParameters, f64, u32);

impl<I, O> Builder<I, O>
    where
        I: Sample + PaSample,
        O: Sample + PaSample,
{

    /// Retrieve the flags, stream parameters, sample rate and frames per buffer.
    fn unwrap_params(self) -> Result<PaParams, Error> {
        let Builder { stream_params, input_params, output_params } = self;
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

        Ok((flags, input_params, output_params, sample_hz, frames))
    }

    /// Launch a non-blocking duplex stream with the given callback!
    #[inline]
    pub fn run_callback(self, mut callback: Callback<I, O>) -> Result<NonBlockingStream<I, O>, Error> {

        // Initialize PortAudio.
        try!(pa::initialize().map_err(|err| Error::PortAudio(err)));

        let (flags, input_params, output_params, sample_hz, frames) = try!(self.unwrap_params());
        let in_channels = input_params.channel_count;
        let out_channels = output_params.channel_count;

        // Here we construct our PortAudio stream.
        let mut stream = pa::Stream::new();

        // Remember the last time the callback was called so we can create the delta time.
        let mut maybe_last_time = None; 

        // Construct a wrapper function around our callback.
        let f = Box::new(move |input: &[I],
                               output: &mut[O],
                               frames: u32,
                               time_info: &pa::StreamCallbackTimeInfo,
                               flags: pa::StreamCallbackFlags| -> pa::StreamCallbackResult {
            let in_settings = Settings {
                sample_hz: sample_hz as u32,
                frames: frames as u16,
                channels: in_channels as u16,
            };
            let out_settings = Settings { channels: out_channels as u16, ..in_settings };
            let dt = time_info.current_time - maybe_last_time.unwrap_or(time_info.current_time);
            maybe_last_time = Some(time_info.current_time);
            match callback(input, in_settings, output, out_settings, dt, flags) {
                CallbackResult::Continue => pa::StreamCallbackResult::Continue,
                CallbackResult::Complete => pa::StreamCallbackResult::Complete,
                CallbackResult::Abort    => pa::StreamCallbackResult::Abort,
            }
        });

        // Here we open the stream.
        try!(stream.open(Some(&input_params), Some(&output_params), sample_hz, frames, flags, Some(f))
                .map_err(|err| Error::PortAudio(err)));

        // And now let's kick it off!
        try!(stream.start().map_err(|err| Error::PortAudio(err)));

        Ok(NonBlockingStream { stream: stream, is_closed: false })
    }

    /// Launch a blocking duplex stream!
    #[inline]
    pub fn run<'a>(self) -> Result<BlockingStream<'a, I, O>, Error> {

        // Initialize PortAudio.
        try!(pa::initialize().map_err(|err| Error::PortAudio(err)));

        let (flags, input_params, output_params, sample_hz, frames) = try!(self.unwrap_params());

        // Here we construct our PortAudio stream.
        let mut stream = pa::Stream::new();

        // Here we open the stream.
        try!(stream.open(Some(&input_params), Some(&output_params), sample_hz, frames, flags, None)
                .map_err(|err| Error::PortAudio(err)));

        // And now let's kick it off!
        try!(stream.start().map_err(|err| Error::PortAudio(err)));

        let in_channels = input_params.channel_count;
        let double_input_buffer_len = (frames as usize * in_channels as usize) * 2;
        let input_buffer_len = ::std::cmp::max(double_input_buffer_len, MINIMUM_BUFFER_RESERVATION);

        let out_channels = output_params.channel_count;
        let double_output_buffer_len = (frames as usize * out_channels as usize) * 2;
        let output_buffer_len = ::std::cmp::max(double_output_buffer_len, MINIMUM_BUFFER_RESERVATION);

        Ok(BlockingStream {
            stream: stream,
            input_buffer: VecDeque::with_capacity(input_buffer_len),
            output_buffer: VecDeque::with_capacity(output_buffer_len),
            user_buffer: Vec::with_capacity(frames as usize * out_channels as usize),
            frames: frames as u16,
            in_channels: in_channels as u16,
            out_channels: out_channels as u16,
            sample_hz: sample_hz as u32,
            last_event: None,
            is_closed: false,
            marker: PhantomData,
        })
    }

}

impl<I, O> NonBlockingStream<I, O>
    where
        I: Sample + PaSample,
        O: Sample + PaSample,
{

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

impl<I, O> Drop for NonBlockingStream<I, O>
    where
        I: Sample + PaSample,
        O: Sample + PaSample,
{
    fn drop(&mut self) {
        if !self.is_closed {
            if let Err(err) = self.close() {
                println!("An error occurred while closing NonBlockingStream: {}", err);
            }
        }
    }
}

impl<'a, I, O> BlockingStream<'a, I, O>
    where
        I: Sample + PaSample,
        O: Sample + PaSample,
{
    /// Close the stream and terminate PortAudio.
    pub fn close(&mut self) -> Result<(), Error> {
        self.is_closed = true;
        try!(self.stream.close().map_err(|err| Error::PortAudio(err)));
        try!(pa::terminate().map_err(|err| Error::PortAudio(err)));
        Ok(())
    }
}

impl<'a, I, O> Drop for BlockingStream<'a, I, O>
    where
        I: Sample + PaSample,
        O: Sample + PaSample,
{
    fn drop(&mut self) {
        if !self.is_closed {
            if let Err(err) = self.close() {
                println!("An error occurred while closing BlockingStream: {}", err);
            }
        }
    }
}

impl<'a, I, O> Iterator for BlockingStream<'a, I, O>
    where
        I: Sample + PaSample,
        O: Sample + PaSample,
{
    type Item = Event<'a, I, O>;

    fn next(&mut self) -> Option<Event<'a, I, O>> {

        let BlockingStream {
            ref mut stream,
            ref mut input_buffer,
            ref mut output_buffer,
            ref mut user_buffer,
            ref mut last_event,
            ref frames,
            ref in_channels,
            ref out_channels,
            ref sample_hz,
            ..
        } = *self;

        let input_settings = Settings { channels: *in_channels, frames: *frames, sample_hz: *sample_hz };
        let target_input_buffer_size = input_settings.buffer_size();

        let output_settings = Settings { channels: *out_channels, frames: *frames, sample_hz: *sample_hz };
        let target_output_buffer_size = output_settings.buffer_size();

        // If the user_buffer was written to last event, take it's contents and append them to the
        // output_buffer for writing.
        if let Some(LastEvent::Out) = *last_event {
            // If some frames were written last event, add them to our output_buffer.
            if user_buffer.len() > 0 {
                output_buffer.extend(user_buffer.iter().map(|&sample| sample));
                user_buffer.clear();
            }
            // Considering the last event was an output event, let us check first for an input event.
            if input_buffer.len() >= target_input_buffer_size {
                let event_buffer = take_front(input_buffer, input_settings.buffer_size());
                *last_event = Some(LastEvent::In);
                return Some(Event::In(event_buffer, input_settings));
            }
        }

        // Loop until we can satisfy an event condition.
        loop {
            use std::error::Error as StdError;

            // How many frames are available on the input stream?
            let available_in_frames = match wait_for_stream(|| stream.get_stream_read_available()) {
                Ok(frames) => frames,
                Err(err) => {
                    println!("An error occurred while requesting the number of available \
                             frames for reading from the input stream: {}. BlockingStream will \
                             now exit the event loop.", StdError::description(&err));
                    return None;
                },
            };

            // If there are frames available, let's take them and add them to our input_buffer.
            if available_in_frames > 0 {
                match stream.read(available_in_frames) {
                    Ok(input_samples) => input_buffer.extend(input_samples.into_iter()),
                    Err(err) => {
                        println!("An error occurred while reading from the input stream: {}. \
                                 BlockingStream will now exit the event loop.",
                                 StdError::description(&err));
                        return None;
                    },
                }
            }

            // How many frames are available for writing on the output stream?
            let available_out_frames = match wait_for_stream(|| stream.get_stream_write_available()) {
                Ok(frames) => frames,
                Err(err) => {
                    println!("An error occurred while requesting the number of available \
                             frames for writing from the output stream: {}. BlockingStream will \
                             now exit the event loop.", StdError::description(&err));
                    return None;
                },
            };

            // How many frames do we have in our output_buffer so far?
            let output_buffer_frames = (output_buffer.len() / *out_channels as usize) as u32;

            // If there are frames available for writing and we have some to write, then write!
            if available_out_frames > 0 && output_buffer_frames > 0 {
                // If we have more than enough frames for writing, take them from the start of the buffer.
                let (write_buffer, write_frames) = if output_buffer_frames >= available_out_frames {
                    let out_samples = (available_out_frames * *out_channels as u32) as usize;
                    let write_buffer = take_front(output_buffer, out_samples);
                    (write_buffer, available_out_frames)
                }
                // Otherwise if we have less, just take what we can for now.
                else {
                    let len = output_buffer.len();
                    let write_buffer = take_front(output_buffer, len);
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
            if output_buffer.len() <= output_buffer.capacity() - target_output_buffer_size {
                use std::iter::repeat;
                // Start the slice just after the already filled samples.
                let start = user_buffer.len();
                // Extend the update buffer by the necessary number of frames.
                user_buffer.extend(repeat(O::zero()).take(output_settings.buffer_size()));
                // Here we obtain a mutable reference to the slice with the correct lifetime so
                // that we can return it via our `Event::Out`. Note: This means that a twisted,
                // evil person could do horrific things with this iterator by calling `.next()`
                // multiple times and storing aliasing mutable references to our output buffer,
                // HOWEVER - this is extremely unlikely to occur in practise as the api is designed
                // in a way that the reference is intended to die at the end of each loop before
                // `.next()` even gets called again.
                let slice = unsafe { ::std::mem::transmute(&mut user_buffer[start..]) };
                *last_event = Some(LastEvent::Out);
                return Some(Event::Out(slice, output_settings));
            }
            // Otherwise, if we've read enough frames for an In event, return one.
            else if input_buffer.len() >= target_input_buffer_size {
                let event_buffer = take_front(input_buffer, input_settings.buffer_size());
                *last_event = Some(LastEvent::In);
                return Some(Event::In(event_buffer, input_settings));
            }

            // If no events occured on this loop, set the last_event to None.
            *last_event = None;

        }

    }

}

