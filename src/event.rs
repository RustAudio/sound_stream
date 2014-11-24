//! 
//! The SoundStream is an Iterator based interpretation of the PortAudio sound stream.
//!

use buffer::AudioBuffer;
use error::Error;
use portaudio::pa;
use portaudio::pa::Sample;
use settings::Settings;
use time::precise_time_ns;

pub type DeltaTimeNs = u64;

/// An event to be returned by the SoundStream.
pub enum Event<'a, B: 'a, I=f32, O=f32> {
    /// Audio awaits on the stream's input buffer.
    In(Vec<I>, Settings),
    /// The stream's output buffer is ready to be written to.
    Out(&'a mut B, Settings),
    /// Called after handling In and Out.
    Update(DeltaTimeNs, Settings),
}

/// Represents the curreent state of the SoundStream.
pub enum State {
    In,
    Out,
    Update,
}

/// An Iterator type for producing Events.
pub struct SoundStream<B, I=f32, O=f32> where B: AudioBuffer<O>, I: Sample, O: Sample {
    prev_state: State,
    last_time: u64,
    stream: pa::Stream<I, O>,
    settings: Settings,
    output_buffer: B,
}

impl<B, I, O> SoundStream<B, I, O> where B: AudioBuffer<O>, I: Sample, O: Sample {

    /// Constructor for an SoundStream.
    pub fn new(settings: Settings) -> Result<SoundStream<B, I, O>, Error> {

        // Initialize PortAudio.
        if let Err(err) = pa::initialize() {
            return Err(Error::PortAudio(err));
        }

        // We're just going to use the default I/O devices.
        let input = pa::device::get_default_input();
        let output = pa::device::get_default_output();

        // Retrieve the number of channels from the given Settings.
        let num_channels = settings.channels as i32;

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
            channel_count: num_channels,
            sample_format: input_sample_format,
            suggested_latency: input_latency,
        };

        // Construct the output stream parameters.
        let output_stream_params = pa::StreamParameters {
            device: output,
            channel_count: num_channels,
            sample_format: output_sample_format,
            suggested_latency: output_latency,
        };

        // Here we construct our PortAudio stream.
        let mut stream = pa::Stream::new();

        // And now let's kick it off!
        if let Err(err) = stream.open(Some(&input_stream_params),
                                      Some(&output_stream_params),
                                      settings.sample_hz as f64,
                                      settings.frames as u32,
                                      pa::StreamFlags::ClipOff) {
            return Err(Error::PortAudio(err))
        }

        Ok(SoundStream {
            prev_state: State::Update,
            last_time: 0,
            stream: stream,
            settings: settings,
            output_buffer: AudioBuffer::zeroed(),
        })
    }

}

impl<'a, B, I, O> Iterator<Event<'a, B, I, O>> for SoundStream<B, I, O>
where B: AudioBuffer<O>, I: Sample, O: Sample {
    fn next(&mut self) -> Option<Event<'a, B, I, O>> {

        // First, determine the new state by checking the previous state.
        let new_state = match self.prev_state {
            State::In => State::Out,
            State::Out => {
                let output_buffer = self.output_buffer.clone_as_vec();
                self.output_buffer = AudioBuffer::zeroed();
                while self.stream.get_stream_write_available() == 0 {}
                match self.stream.write(output_buffer, self.settings.frames as u32) {
                    Ok(()) => {
                        State::Update
                    },
                    Err(err) => {
                        println!("Breaking from loop as sound_stream failed to \
                                 write to the PortAudioStream: {}.", err);
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
                while self.stream.get_stream_read_available() == 0 {}
                match self.stream.read(self.settings.frames as u32) {
                    Ok(input_buffer) => {
                        Some(Event::In(input_buffer, self.settings))
                    },
                    Err(err) => {
                        println!("Breaking from loop as sound_stream failed to \
                                 read from the PortAudioStream: {}.", err);
                        None
                    },
                }
            },
            State::Out => {
                let SoundStream { output_buffer: ref mut buffer, settings: settings, .. } = *self;
                Some(Event::Out(buffer, settings))
            },
            State::Update => {
                let this_time = precise_time_ns();
                let diff_time = this_time - self.last_time;
                self.last_time = this_time;
                Some(Event::Update(diff_time, self.settings))
            },
        }
    }
}

