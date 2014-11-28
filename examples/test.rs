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

pub type Input = f32;
pub type Output = f32;
pub type OutputBuffer = Vec<Output>;

fn main() {

    // Construct the stream and handle any errors that may have occurred.
    let mut stream = match SoundStream::<OutputBuffer, Input, Output>::new(SETTINGS) {
        Ok(stream) => stream,
        Err(err) => panic!("An error occurred while constructing SoundStream: {}", err),
    };

    // We'll use this to copy the input buffer straight to the output buffer.
    let mut cloner = Vec::new();

    // The SoundStream iterator will automatically return these events in this order.
    for event in stream {
        match event {
            Event::In(buffer) => { ::std::mem::replace(&mut cloner, buffer); },
            Event::Out(buffer) => *buffer = cloner.clone(),
            Event::Update(dt) => println!("update: dt {}", dt),
        }
    }

    // Close the stream and shut down PortAudio.
    match stream.close() {
        Ok(()) => println!("SoundStream closed successfully!"),
        Err(err) => println!("An error occurred while closing SoundStream: {}", err),
    }

}

