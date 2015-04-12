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

    // The SoundStream iterator will automatically return each event in the correct order,
    // where the first three events will always be `In`, `Out` and then `Update`.
    //
    // The following order is determined by the given update rate. The update rate determines
    // the size of the `Out` buffer in frames, where the number of frames is as close as possible
    // in duration to the update rate given.
    //
    // The `Out` and `Update` events are called as fast as possible and the samples are stored
    // until the next `Out` would give `n` number of frames where `n >= FRAMES`. At this point the
    // stream will return an `In` event before continuing the cycle. The first `Out` event of the
    // next cycle would collect the final frames needed for sending to the output device. If more
    // frames are collected than is needed, the remaining frames will be stored for the next buffer.
    for event in stream.by_ref() {
        match event {
            Event::In(buffer) => {
                intermediate.extend(buffer.into_iter());
            },
            Event::Out(buffer, settings) => {

                let buffer_size = settings.buffer_size();
                let remaining = intermediate[buffer_size..].iter().map(|&sample| sample).collect();
                intermediate.truncate(buffer_size);
                let samples = ::std::mem::replace(&mut intermediate, remaining);
                for (output_sample, sample) in buffer.iter_mut().zip(samples.into_iter()) {
                    *output_sample = sample;
                }

                // NOTE: The above will be replaced by the following once `split_off` and
                // `clone_from_slice` are stablilised.
                // let remaining = intermediate.split_off(settings.buffer_size());
                // let samples = ::std::mem::replace(&mut intermediate, remaining);
                // buffer.clone_from_slice(&samples[..]);

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

