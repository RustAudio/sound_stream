//!
//! The SoundStream is an Iterator based interpretation of the PortAudio sound stream.
//!

use error::Error;
use portaudio::pa;
use portaudio::pa::Sample as PaSample;
use sample::{Sample, Wave};
use settings::{Settings, Frames};
use std::marker::PhantomData;
use time::SteadyTime;

/// Difference in time between Update events.
pub type DeltaTimeSeconds = f64;

/// An event to be returned by the SoundStream.
#[derive(Debug)]
pub enum Event<'a, I=Wave, O=Wave> where O: 'a {
    /// Audio awaits on the stream's input buffer.
    In(Vec<I>, Settings),
    /// The stream's output buffer is ready to be written to.
    Out(&'a mut [O], Settings),
    /// Called after handling In and Out.
    Update(DeltaTimeSeconds),
}

/// Represents the current state of the SoundStream.
#[derive(Clone, Copy)]
pub enum NextEvent {
    In,
    Out,
    Update,
}

/// Represents the update frequency.
enum UpdateFrequency {
    Hz(f32),
    Frames(u16),
    PerBuffer(u16),
}

/// A builder context for a SoundStream.
pub struct SoundStreamBuilder<'a, I, O> {
    maybe_settings: Option<Settings>,
    maybe_update_rate: Option<UpdateFrequency>,
    phantom_data_i: PhantomData<I>,
    phantom_data_o: PhantomData<O>,
    phantom_data_lifetime: PhantomData<&'a ()>,
}

impl<'a, I, O> SoundStreamBuilder<'a, I, O>
    where
        I: Sample + PaSample,
        O: Sample + PaSample,
{

    /// Custom SoundStreamSettings.
    #[inline]
    pub fn settings(self, settings: Settings) -> SoundStreamBuilder<'a, I, O> {
        SoundStreamBuilder { maybe_settings: Some(settings), ..self }
    }

    /// Custom `Event::Update` rate in hz.
    #[inline]
    pub fn update_hz(self, hz: f32) -> SoundStreamBuilder<'a, I, O> {
        assert!(hz > 0.0, "`update_hz` must be greater than 0.0, but you gave {:?}", hz);
        SoundStreamBuilder { maybe_update_rate: Some(UpdateFrequency::Hz(hz)), ..self }
    }

    /// Custom `Event::Update` rate in frames.
    #[inline]
    pub fn update_frames(self, frames: u16) -> SoundStreamBuilder<'a, I, O> {
        assert!(frames > 0, "`update_frames` must be greater than 0, but you gave {:?}", frames);
        SoundStreamBuilder { maybe_update_rate: Some(UpdateFrequency::Frames(frames)), ..self }
    }

    /// Custom `Event::Update` rate as a number of buffer divisions. The number of divisions
    /// given must be some multiple of two, so that the buffer can be divided evenly and
    /// consistently.
    #[inline]
    pub fn updates_per_buffer(self, num: u16) -> SoundStreamBuilder<'a, I, O> {
        assert!(num != 0 && num % 2 == 0, "`updates_per_buffer` may only take multiples of two, \
                but you gave it {:?}. If you wish to use a non-multiple of two, use the \
                `update_hz` method instead.", num);
        SoundStreamBuilder { maybe_update_rate: Some(UpdateFrequency::PerBuffer(num)), ..self }
    }

    /// Launch the SoundStream!
    #[inline]
    pub fn run(mut self) -> Result<SoundStream<'a, I, O>, Error> {

        // Take the settings from the SoundStreamBuilder or use defaults.
        let stream_settings = self.maybe_settings
            .take()
            .unwrap_or_else(|| Settings::cd_quality());
        let update_rate = self.maybe_update_rate
            .take()
            .unwrap_or_else(|| UpdateFrequency::PerBuffer(1));

        // Determine the closest number of frames to the requested rate.
        let frames_per_update = match update_rate {
            UpdateFrequency::Frames(frames) => frames,
            UpdateFrequency::PerBuffer(n) => stream_settings.frames / n,
            UpdateFrequency::Hz(hz) => {
                use num::Float;
                let buffer_hz = stream_settings.sample_hz as f32 / stream_settings.frames as f32;
                let updates_per_buffer = hz / buffer_hz;
                (stream_settings.frames as f32 / updates_per_buffer).round() as Frames
            },
        };

        assert!(frames_per_update == stream_settings.frames ||
                frames_per_update <= stream_settings.frames/2, "SoundStream currently only \
                supports custom update rates that are at least two times faster than the \
                stream callback.");

        let update_settings = Settings { frames: frames_per_update, ..stream_settings };

        // Initialize PortAudio.
        if let Err(err) = pa::initialize() {
            return Err(Error::PortAudio(err));
        }

        // We're just going to use the default I/O devices.
        let input = pa::device::get_default_input();
        let output = pa::device::get_default_output();

        // Determine the sample format for both the input and output.
        let default_input_sample: I = ::std::default::Default::default();
        let default_output_sample: O = ::std::default::Default::default();
        let input_sample_format  = default_input_sample.sample_format();
        let output_sample_format = default_output_sample.sample_format();

        // Request the suggested latency for the input and output devices from PortAudio.
        let input_latency = match pa::device::get_info(input) {
            Ok(info) => info.default_low_input_latency,
            Err(err) => return Err(Error::PortAudio(err)),
        };
        let output_latency = match pa::device::get_info(output) {
            Ok(info) => info.default_low_output_latency,
            Err(err) => return Err(Error::PortAudio(err)),
        };

        // Construct the input stream parameters.
        let input_stream_params = pa::StreamParameters {
            device: input,
            channel_count: stream_settings.channels as i32,
            sample_format: input_sample_format,
            suggested_latency: input_latency,
        };

        // Construct the output stream parameters.
        let output_stream_params = pa::StreamParameters {
            device: output,
            channel_count: stream_settings.channels as i32,
            sample_format: output_sample_format,
            suggested_latency: output_latency,
        };

        // Here we construct our PortAudio stream.
        let mut stream = pa::Stream::new();

        // Here we open the stream.
        if let Err(err) = stream.open(Some(&input_stream_params),
                                      Some(&output_stream_params),
                                      stream_settings.sample_hz as f64,
                                      stream_settings.frames as u32,
                                      pa::StreamFlags::ClipOff,
                                      None) {
            return Err(Error::PortAudio(err))
        }

        // And now let's kick it off!
        if let Err(err) = stream.start() {
            return Err(Error::PortAudio(err))
        }

        Ok(SoundStream {
            update_settings: update_settings,
            last_time: SteadyTime::now().0,
            output_buffer: Vec::with_capacity(stream_settings.buffer_size()),
            input_buffer: Vec::with_capacity(stream_settings.buffer_size()),
            next_event: NextEvent::In,
            stream: stream,
            stream_settings: stream_settings,
            marker: PhantomData,
            is_closed: false,
        })

    }

}

/// An Iterator type for producing Events.
pub struct SoundStream<'a, I=Wave, O=Wave>
    where
        I: Sample + PaSample,
        O: Sample + PaSample,
{
    last_time: u64,
    /// Requested stream format.
    stream_settings: Settings,
    /// Contains a description of the stream format with the target update rate.
    update_settings: Settings,
    /// Store samples in this until there is enough to write to the output stream.
    output_buffer: Vec<O>,
    /// Buffer the samples from the input until its length is equal to the buffer_length.
    input_buffer: Vec<I>,
    /// The next event that will occur.
    next_event: NextEvent,
    /// The port audio stream.
    stream: pa::Stream<I, O>,
    is_closed: bool,
    marker: PhantomData<&'a ()>,
}

impl<'a, I, O> SoundStream<'a, I, O>
    where
        I: Sample + PaSample,
        O: Sample + PaSample,
{

    /// Constructs the builder for a new SoundStream.
    #[inline]
    pub fn new() -> SoundStreamBuilder<'a, I, O> {
        SoundStreamBuilder {
            maybe_settings: None,
            maybe_update_rate: None,
            phantom_data_i: PhantomData,
            phantom_data_o: PhantomData,
            phantom_data_lifetime: PhantomData,
        }
    }

    /// Close the stream and terminate PortAudio.
    pub fn close(&mut self) -> Result<(), Error> {
        self.is_closed = true;
        if let Err(err) = self.stream.close() { return Err(Error::PortAudio(err)) }
        if let Err(err) = pa::terminate() { return Err(Error::PortAudio(err)) }
        Ok(())
    }

}

impl<'a, I, O> Drop for SoundStream<'a, I, O>
    where
        I: Sample + PaSample,
        O: Sample + PaSample,
{
    fn drop(&mut self) {
        if !self.is_closed {
            if let Err(err) = self.close() {
                println!("An error occurred while closing SoundStream: {}", err);
            }
        }
    }
}

impl<'a, I, O> Iterator for SoundStream<'a, I, O>
    where
        I: Sample + PaSample,
        O: Sample + PaSample,
{
    type Item = Event<'a, I, O>;

    fn next(&mut self) -> Option<Event<'a, I, O>> {

        let SoundStream {
            ref mut stream,
            ref mut input_buffer,
            ref mut output_buffer,
            ref mut next_event,
            ref stream_settings,
            ref update_settings,
            ref mut last_time,
            ..
        } = *self;

        loop {
            use std::error::Error as StdError;
            use std::mem::replace;

            // How many frames are available on the input stream?
            let available_in_frames = match wait_for_stream(|| stream.get_stream_read_available()) {
                Ok(frames) => frames,
                Err(err) => {
                    println!("An error occurred while requesting the number of available \
                             frames for reading from the input stream: {}. SoundStream will \
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
                                 SoundStream will now exit the event loop.",
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
                             frames for writing from the output stream: {}. SoundStream will \
                             now exit the event loop.", StdError::description(&err));
                    return None;
                },
            };

            // How many frames do we have in our output_buffer so far?
            let output_buffer_frames = (output_buffer.len() / stream_settings.channels as usize) as u32;

            // If there are frames available for writing and we have some to write, then write!
            if available_out_frames > 0 && output_buffer_frames > 0 {
                // If we have more than enough frames for writing, take them from the start of the buffer.
                let (write_buffer, write_frames) = if output_buffer_frames >= available_out_frames {
                    let out_samples = (available_out_frames * stream_settings.channels as u32) as usize;
                    let remaining = output_buffer[out_samples..].iter().map(|&sample| sample).collect();
                    output_buffer.truncate(out_samples);
                    let write_buffer = replace(output_buffer, remaining);
                    (write_buffer, available_out_frames)
                }
                // Otherwise if we have less, just take what we can for now.
                else {
                    let write_buffer = replace(output_buffer, Vec::with_capacity(stream_settings.buffer_size()));
                    (write_buffer, output_buffer_frames)
                };
                if let Err(err) = stream.write(write_buffer, write_frames) {
                    println!("An error occurred while writing to the output stream: {}",
                             StdError::description(&err));
                    return None
                }
            }

            match *next_event {
                NextEvent::In => {
                    let target_samples = update_settings.buffer_size();
                    // Once we have enough samples to create an input event, do so.
                    if input_buffer.len() >= target_samples {
                        let remaining = input_buffer[target_samples..].iter()
                            .map(|&sample| sample).collect();
                        input_buffer.truncate(target_samples);
                        let buffer = replace(input_buffer, remaining);
                        *next_event = NextEvent::Out;
                        return Some(Event::In(buffer, *update_settings))
                    }
                },
                NextEvent::Out => {
                    // Don't let the output_buffer fill any higher than the given stream_settings.
                    if output_buffer.len() <= stream_settings.buffer_size() {
                        use std::iter::repeat;
                        // Start the slice just after the already filled samples.
                        let start = output_buffer.len();
                        // Extend the update buffer by the necessary number of frames.
                        output_buffer.extend(repeat(Sample::zero()).take(update_settings.buffer_size()));
                        // Here we obtain a mutable reference to the slice with the correct lifetime so
                        // that we can return it via our `Event::Out`. Note: This means that a twisted,
                        // evil person could do horrific things with this iterator by calling `.next()`
                        // multiple times and storing aliasing mutable references to our output buffer,
                        // HOWEVER - this is extremely unlikely to occur in practise as the api is designed
                        // in a way that the reference is intended to die at the end of each loop before
                        // `.next()` even gets called again.
                        let slice = unsafe { ::std::mem::transmute(&mut output_buffer[start..]) };
                        *next_event = NextEvent::Update;
                        return Some(Event::Out(slice, *update_settings))
                    }
                },
                NextEvent::Update => {
                    let this_time = SteadTime::now().0;
                    let diff_time = this_time - *last_time;
                    *last_time = this_time;
                    const BILLION: f64 = 1_000_000_000.0;
                    let diff_time_in_seconds = diff_time as f64 / BILLION;
                    *next_event = NextEvent::In;
                    return Some(Event::Update(diff_time_in_seconds))
                },
            }

        }

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
