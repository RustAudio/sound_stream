
/// Settings required for SoundStream.
#[deriving(Show, Clone, PartialEq)]
pub struct Settings {
    /// The number of samples per second.
    pub sample_hz: u32,
    /// How many samples per channel requested at a time in the buffer.
    /// The more frames, the less likely to make glitches,
    /// but this gives slower response.
    pub frames: u16,
    /// Number of channels, for example 2 for stereo sound (left + right speaker).
    pub channels: u16
}

impl Settings {

    /// Custom constructor for the Settings.
    pub fn new(sample_hz: u32, frames: u16, channels: u16) -> Settings {
        Settings {
            sample_hz: sample_hz,
            frames: frames,
            channels: channels
        }
    }

    /// Default, standard constructor for Settings.
    pub fn cd_quality() -> Settings {
        Settings {
            sample_hz: 44100,
            frames: 256,
            channels: 2
        }
    }

    /// Return the length of a SoundBuffer that would use Settings.
    pub fn buffer_size(&self) -> uint {
        self.frames as uint * self.channels as uint
    }

}

impl ::std::default::Default for Settings {
    fn default() -> Settings { Settings::cd_quality() }
}

