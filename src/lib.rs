
#![deny(missing_docs)]

extern crate portaudio;

pub type DeltaTime = f64;

/// An event to be returned by the Event iterator.
pub enum Event {
    Update(DeltaTime),
    In,
    Out,
}


