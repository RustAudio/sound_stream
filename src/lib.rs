
#![feature(collections, core, unboxed_closures, unsafe_destructor)]

extern crate portaudio;
extern crate sample;
extern crate time;

pub use error::Error;
pub use event::SoundStream;
pub use event::Event;
pub use portaudio::pa::Sample as PaSample;
pub use sample::{Amplitude, Sample, Wave};
pub use settings::Settings;

pub mod error;
pub mod event;
pub mod settings;
