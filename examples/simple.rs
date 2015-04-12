//! 
//! A simple-as-possible example showing the basics of SoundStream.
//!
//! In this example we just copy the input buffer straight to the output (beware of feedback).
//!

extern crate sound_stream;

use sound_stream::{Event, SoundStream};

pub type Sample = f32;
pub type Output = Sample;
pub type Input = Sample;

fn main() {

    // Construct the default stream. The default stream format is "cd_quality" aka 44.1khz stereo.
    let mut stream = SoundStream::<Input, Output>::new().run().unwrap();

    // We'll use this to copy the input buffer straight to the output buffer.
    let mut cloner = Vec::new();

    // We'll use this to count down from 3 seconds before breaking from the loop.
    let mut count: f64 = 3.0;

    // The SoundStream iterator will automatically return these events in this order.
    for event in stream.by_ref() {
        match event {
            Event::In(buffer) => { ::std::mem::replace(&mut cloner, buffer); },
            Event::Out(buffer, _) => {

                for (output_sample, sample) in buffer.iter_mut().zip(cloner.iter().map(|&s| s)) {
                    *output_sample = sample;
                }

                // NOTE: The above will be replaced by the following once `clone_from_slice` is
                // stabilised.
                // buffer.clone_from_slice(&cloner[..]);

            },
            Event::Update(dt_secs) => {
                count -= dt_secs;
                if count < 0.0 { break } else { println!("{}", count) }
            },
        }
    }

}
