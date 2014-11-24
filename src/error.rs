//! 
//! The sound_stream Error type.
//!

use portaudio::pa::error::Error as PortAudioError;

/// A type for representing errors in sound_stream.
#[deriving(Show, Clone)]
pub enum Error {
    /// Errors returned by rust-portaudio.
    PortAudio(PortAudioError),
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        use self::Error::*;
        match *self {
            PortAudio(ref err) => err.description(),
        }
    }
}

