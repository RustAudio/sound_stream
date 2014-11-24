
extern crate sound_stream;

use sound_stream::{
    Event,
    Settings,
    SoundStream,
};

fn main() {

    let mut stream = match SoundStream::<[f32, ..256], f32, f32>::new(Settings::cd_quality()) {
        Ok(stream) => stream,
        Err(err) => panic!("A SoundStream error occurred: {}", err),
    };

    for event in stream {
        match event {
            Event::In(..) => println!("in"),
            Event::Out(..) => println!("out"),
            Event::Update(..) => println!("update"),
        }
    }

}

