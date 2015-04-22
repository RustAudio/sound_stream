//! 
//! A more advanced example showing the customisation features of SoundStream.
//!
//! In this example we setup our SoundStream with a custom stream format and custom update
//! frequency. We will then copy the sound from the input stream straight to the output stream
//! while taking into account the unique update rate. Beware of speaker/mic feedback!
//!

extern crate sound_stream;

use sound_stream::{Event, Settings, SoundStream};

const SAMPLE_HZ: u32 = 44_100;
const FRAMES: u16 = 256;
const CHANNELS: u16 = 2;

const SETTINGS: Settings = Settings { sample_hz: SAMPLE_HZ, frames: FRAMES, channels: CHANNELS };

pub type Sample = f32;
pub type Output = Sample;
pub type Input = Sample;

fn main() {

    // Construct the stream with a custom format and update rate.
    let result = SoundStream::<Input, Output>::new()
        .settings(SETTINGS)
        .update_hz(1000.0)
        .run();

    // Handle any errors that may have occured.
    let mut stream = match result {
        Ok(stream) => stream,
        Err(err) => panic!("An error occurred while constructing SoundStream: {}", err),
    };

    // We'll use this to copy the input buffer straight to the output buffer.
    let mut intermediate = Vec::new();

    // We'll use this to count down from 3 seconds before breaking from the loop.
    let mut count: f64 = 3.0;

    // The SoundStream iterator will automatically return these events in this order,
    for event in stream.by_ref() {
        match event {
            Event::In(buffer, _) => { ::std::mem::replace(&mut intermediate, buffer); },
            Event::Out(buffer, _) => {
                for (output_sample, sample) in buffer.iter_mut().zip(intermediate.iter()) {
                    *output_sample = *sample;
                }
            },
            Event::Update(dt_secs) => {
                count -= dt_secs;
                if count < 0.0 { break } else { println!("{}", count) }
            },
        }
    }

    // You can close the stream and shut down PortAudio manually if you want to return the result.
    match stream.close() {
        Ok(()) => println!("SoundStream closed successfully!"),
        Err(err) => println!("An error occurred while closing SoundStream: {}", err),
    }

}

