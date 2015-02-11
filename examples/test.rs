//! 
//! A simple example showing the basics of SoundStream.
//!
//! In this example we just copy the input buffer straight to the output (beware of feedback).
//!

extern crate sound_stream;

use sound_stream::{
    Event,
    Settings,
    SoundStream,
};

const SAMPLE_HZ: u32 = 44_100;
const FRAMES: u16 = 256;
const CHANNELS: u16 = 2;

const SETTINGS: Settings = Settings { sample_hz: SAMPLE_HZ, frames: FRAMES, channels: CHANNELS };

pub type Sample = f32;
pub type Input = Sample;
pub type OutputBuffer = Vec<Sample>;

fn main() {

    // Construct the stream and handle any errors that may have occurred.
    let mut stream = match SoundStream::<OutputBuffer, Input>::new(SETTINGS) {
        Ok(stream) => stream,
        Err(err) => panic!("An error occurred while constructing SoundStream: {}", err),
    };

    // We'll use this to copy the input buffer straight to the output buffer.
    let mut cloner = Vec::new();

    // We'll use this to count down from 3 seconds before breaking from the loop.
    let mut count: f64 = 3.0;

    // The SoundStream iterator will automatically return these events in this order.
    for event in stream.by_ref() {
        match event {
            Event::In(buffer) => { ::std::mem::replace(&mut cloner, buffer); },
            Event::Out(buffer) => *buffer = cloner.clone(),
            Event::Update(dt_secs) => {
                count -= dt_secs;
                if count < 0.0 { break } else { println!("{}", count) }
            },
        }
    }

    // Close the stream and shut down PortAudio.
    match stream.close() {
        Ok(()) => println!("SoundStream closed successfully!"),
        Err(err) => println!("An error occurred while closing SoundStream: {}", err),
    }

}

