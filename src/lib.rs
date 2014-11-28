
#![feature(default_type_params, if_let, globs, macro_rules, unboxed_closures)]

extern crate portaudio;
extern crate time;

pub use buffer::AudioBuffer;
pub use event::SoundStream;
pub use event::Event;
pub use settings::Settings;

pub mod buffer;
pub mod error;
pub mod event;
pub mod settings;
