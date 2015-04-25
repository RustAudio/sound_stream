
extern crate num;
extern crate portaudio;
extern crate sample;
extern crate time;

pub use error::Error;
pub use portaudio::pa::Sample as PaSample;
pub use sample::{Amplitude, Sample, Wave};
pub use settings::{Settings, SampleHz, Frames, Channels};
pub use stream::Idx as DeviceIdx;
pub use stream::{
    input,
    output,
    duplex,
    CallbackFlags,
    CallbackResult,
    DeltaTimeSeconds,
    Latency,
    SoundStream,
    StreamFlags,
    StreamParams,
};

mod error;
mod settings;
mod stream;
mod utils;

