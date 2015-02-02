//! 
//! The SoundStream is an Iterator based interpretation of the PortAudio sound stream.
//!

use buffer::AudioBuffer;
use error::Error;
use portaudio::pa;
use portaudio::pa::Sample;
use settings::Settings;
use std::marker::{
    ContravariantLifetime,
    NoCopy,
};
use time::precise_time_ns;

pub type DeltaTimeSeconds = f64;

/// An event to be returned by the SoundStream.
#[derive(Debug)]
pub enum Event<'a, B, I=f32> where B: 'a {
    /// Audio awaits on the stream's input buffer.
    In(Vec<I>),
    /// The stream's output buffer is ready to be written to.
    Out(&'a mut B),
    /// Called after handling In and Out.
    Update(DeltaTimeSeconds),
}

/// Represents the current state of the SoundStream.
#[derive(Copy)]
pub enum State {
    In,
    Out,
    Update,
}

/// An Iterator type for producing Events.
pub struct SoundStream<'a, B=Vec<f32>, I=f32> where B: AudioBuffer + 'a, I: Sample {
    prev_state: State,
    last_time: u64,
    stream: pa::Stream<I, <B as AudioBuffer>::Sample>,
    settings: Settings,
    output_buffer: B,
    marker: ContravariantLifetime<'a>,
    marker2: NoCopy,
}

impl<'a, B, I> SoundStream<'a, B, I> where B: AudioBuffer + 'a, I: Sample {

    /// Constructor for a SoundStream.
    pub fn new(settings: Settings) -> Result<SoundStream<'a, B, I>, Error> {

        // Initialize PortAudio.
        if let Err(err) = pa::initialize() {
            return Err(Error::PortAudio(err));
        }

        // We're just going to use the default I/O devices.
        let input = pa::device::get_default_input();
        let output = pa::device::get_default_output();

        // Determine the sample format for both the input and output.
        let default_input_sample: I = ::std::default::Default::default();
        let default_output_sample: <B as AudioBuffer>::Sample = ::std::default::Default::default();
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
            channel_count: settings.channels as i32,
            sample_format: input_sample_format,
            suggested_latency: input_latency,
        };

        // Construct the output stream parameters.
        let output_stream_params = pa::StreamParameters {
            device: output,
            channel_count: settings.channels as i32,
            sample_format: output_sample_format,
            suggested_latency: output_latency,
        };

        // Here we construct our PortAudio stream.
        let mut stream = pa::Stream::new();


        // Here we open the stream.
        if let Err(err) = stream.open(Some(&input_stream_params),
                                      Some(&output_stream_params),
                                      settings.sample_hz as f64,
                                      settings.frames as u32,
                                      pa::StreamFlags::ClipOff) {
            return Err(Error::PortAudio(err))
        }

        // And now let's kick it off!
        if let Err(err) = stream.start() {
            return Err(Error::PortAudio(err))
        }

        Ok(SoundStream {
            prev_state: State::Update,
            last_time: precise_time_ns(),
            stream: stream,
            settings: settings,
            output_buffer: AudioBuffer::zeroed((settings.frames * settings.channels) as usize),
            marker: ContravariantLifetime,
            marker2: NoCopy,
        })
    }

    /// Close the stream and terminate PortAudio.
    pub fn close(&mut self) -> Result<(), Error> {
        if let Err(err) = self.stream.close() { return Err(Error::PortAudio(err)) }
        if let Err(err) = pa::terminate() { return Err(Error::PortAudio(err)) }
        Ok(())
    }

}

impl<'a, B, I> Iterator for SoundStream<'a, B, I>
where B: AudioBuffer + 'a, I: Sample {
    type Item = Event<'a, B, I>;
    fn next(&mut self) -> Option<Event<'a, B, I>> {

        // First, determine the new state by checking the previous state.
        let new_state = match self.prev_state {
            State::In => State::Out,
            State::Out => {
                use std::mem::replace;
                let len = (self.settings.frames * self.settings.channels) as usize;
                let output_buffer = replace(&mut self.output_buffer, AudioBuffer::zeroed(len))
                    .clone_as_vec();
                if let Err(err) = wait_for_stream(|| self.stream.get_stream_write_available()) {
                    println!("Breaking from loop as sound_stream failed to \
                             write to the PortAudio stream: {}.", err);
                    return None
                }
                match self.stream.write(output_buffer, self.settings.frames as u32) {
                    Ok(()) => State::Update,
                    Err(err) => {
                        println!("Breaking from loop as sound_stream failed to \
                                 write to the PortAudio stream: {}.", err);
                        return None
                    },
                }
            },
            State::Update => State::In,
        };

        // Prepare the next event in accordance with the new state. 
        self.prev_state = new_state;
        match new_state {

            State::In => {
                if let Err(err) = wait_for_stream(|| self.stream.get_stream_read_available()) {
                    println!("Breaking from loop as sound_stream failed to \
                             read from the PortAudio stream: {}.", err);
                    return None
                }
                match self.stream.read(self.settings.frames as u32) {
                    Ok(input_buffer) => Some(Event::In(input_buffer)),
                    Err(err) => {
                        println!("Breaking from loop as sound_stream failed to \
                                 read from the PortAudio stream: {}.", err);
                        None
                    },
                }
            },

            State::Out => {
                let SoundStream { output_buffer: ref mut buffer, .. } = *self;

                // Here we obtain a mutable reference to the buffer with the correct lifetime so
                // that we can return it via our `Event::Out`. Note: This means that a twisted,
                // evil person could do horrific things with this iterator by calling `.next()`
                // multiple times and storing aliasing mutable references to our output buffer,
                // HOWEVER - this is extremely unlikely to occur in practise as the api is designed
                // in a way that the reference is intended to die at the end of each loop before
                // `.next()` even gets called again.
                let output_buffer = unsafe { ::std::mem::transmute(buffer) };

                Some(Event::Out(output_buffer))
            },

            State::Update => {
                let this_time = precise_time_ns();
                let diff_time = this_time - self.last_time;
                self.last_time = this_time;
                const BILLION: f64 = 1_000_000_000.0;
                let diff_time_in_seconds = diff_time as f64 / BILLION;
                Some(Event::Update(diff_time_in_seconds))
            },

        }

    }
}

/// Wait for the given stream to become ready for reading/writing.
fn wait_for_stream<F>(f: F) -> Result<i64, Error>
where F: Fn() -> Result<Option<i64>, pa::Error> {
    loop {
        match f() {
            Ok(None) => (),
            Ok(Some(frames)) => return Ok(frames),
            Err(err) => return Err(Error::PortAudio(err)),
        }
    }
}

