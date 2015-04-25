//! 
//! A simple-as-possible example showing how to construct and use a non-blocking duplex stream.
//!
//! In this example we just copy the input buffer straight to the output (beware of feedback).
//!

extern crate sound_stream;

use sound_stream::{CallbackFlags, CallbackResult, SoundStream, Settings, StreamParams};

fn main() {

    // We'll use this to count down from 3 seconds before breaking from the loop.
    let mut count = 3.0;

    // The callback we'll use to pass to the Stream. It will write the input directly to the output.
    let f = Box::new(move |i: &[f32], _: Settings, o: &mut[f32], _: Settings, dt: f64, _: CallbackFlags| {
        for (output_sample, input_sample) in o.iter_mut().zip(i.iter()) {
            *output_sample = *input_sample;
        }
        count -= dt;
        if count >= 0.0 { CallbackResult::Continue } else { CallbackResult::Complete }
    });

    // Construct the default duplex stream that produces 512 frames per buffer.
    let stream = SoundStream::new()
        .frames_per_buffer(512)
        .duplex(StreamParams::new(), StreamParams::new())
        .run_callback(f)
        .unwrap();

    while let Ok(true) = stream.is_active() {}

}

