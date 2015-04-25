//! 
//! Generate a 440hz sine wave with sound_stream's non-blocking output stream.
//!

extern crate sound_stream;

use sound_stream::{CallbackFlags, CallbackResult, SoundStream, Settings, StreamParams};

/// Produce a sine wave given some phase.
fn sine_wave(phase: f64) -> f32 {
    ((phase * ::std::f64::consts::PI * 2.0).sin() * 0.5) as f32
}

fn main() {

    // We'll use this to count down from 3 seconds before breaking from the loop.
    let mut count = 3.0;

    // We'll use this as the phase for our oscillator.
    let mut phase = 0.0;

    // The callback we'll use to pass to the Stream. It will write a 440hz sine wave to the output.
    let callback = Box::new(move |output: &mut[f32], settings: Settings, dt: f64, _: CallbackFlags| {
        for frame in output.chunks_mut(settings.channels as usize) {
            let amp = sine_wave(phase);
            for channel in frame {
                *channel = amp;
            }
            phase += 440.0 / settings.sample_hz as f64;
        }
        count -= dt;
        if count >= 0.0 { CallbackResult::Continue } else { CallbackResult::Complete }
    });

    // Construct the default, non-blocking output stream and run our callback.
    let stream = SoundStream::new().output(StreamParams::new()).run_callback(callback).unwrap();

    while let Ok(true) = stream.is_active() {}

}

