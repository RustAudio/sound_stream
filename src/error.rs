//! 
//! The sound_stream Error type.
//!

use portaudio::pa::error::Error as PortAudioError;

/// A type for representing errors in sound_stream.
#[derive(Debug, Copy, Clone)]
pub enum Error {
    /// Errors returned by rust-portaudio.
    PortAudio(PortAudioError),
}

impl ::std::fmt::Display for Error {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        use self::Error::*;
        match *self {
            PortAudio(ref err) => err.description(),
        }
    }
}

// /// A type for indicating what to do on the occurence of an error.
// #[derive(Debug, Copy, Clone)]
// pub enum Action {
//     /// Break from the portaudio stream loop.
//     Break,
//     /// Ignore the error and continue the stream loop.
//     Ignore,
// }

