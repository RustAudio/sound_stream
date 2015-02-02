
#![feature(core, unboxed_closures)]

extern crate portaudio;
extern crate time;

pub use buffer::AudioBuffer;
pub use error::Error;
pub use event::SoundStream;
pub use event::Event;
pub use portaudio::pa::Sample;
pub use settings::Settings;

pub mod buffer;
pub mod error;
pub mod event;
pub mod settings;
