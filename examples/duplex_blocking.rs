//! 
//! A simple-as-possible example showing how to construct and use a blocking duplex stream.
//!
//! In this example we just copy the input buffer straight to the output (beware of feedback).
//!
//! NOTE: It is recommended to use the non-blocking stream instead when possible as blocking
//! streams are currently unstable and trickier to synchronise.
//!

extern crate sound_stream;

use sound_stream::{SoundStream, StreamParams};
use sound_stream::duplex::Event;

fn main() {

    // Construct the default duplex stream that produces 512 frames per buffer.
    let mut stream = SoundStream::new()
        .frames_per_buffer(128)
        .duplex::<f32, f32>(StreamParams::new(), StreamParams::new())
        .run().unwrap();

    // We'll use this to count down from 3 seconds before breaking from the loop.
    let mut count = 3.0;

    // We'll use this to copy the input buffer straight to the output buffer.
    let mut intermediate = Vec::new();

    for event in stream.by_ref() {
        match event {
            Event::In(input, _) => { ::std::mem::replace(&mut intermediate, input); }
            Event::Out(output, settings) => {
                for (output_sample, sample) in output.iter_mut().zip(intermediate.iter()) {
                    *output_sample = *sample;
                }
                count -= settings.frames as f32 / settings.sample_hz as f32;
                if count <= 0.0 { break }
            }
        }
    }

}

