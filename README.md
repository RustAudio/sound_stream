
# SoundStream [![Build Status](https://travis-ci.org/RustAudio/sound_stream.svg?branch=master)](https://travis-ci.org/RustAudio/sound_stream)

A simple-as-possible, *fast* audio I/O stream wrapping PortAudio for Rust! It looks like this:

```Rust
for event in stream.by_ref() {
    match event {
        Event::In(input_buffer) => println!("Incoming audio!"),
        Event::Out(output_buffer) => println!("Time to write to output!"),
        Event::Update(delta_time) => println!("Updatey stuff here."),
    }
}
```


Usage
-----

Add sound_stream to your Cargo.toml dependencies like so:

```
[dependencies]
sound_stream = "*"
```

For more details, see [the example](https://github.com/RustAudio/sound_stream/blob/master/examples/test.rs).

PortAudio
---------

SoundStream uses [PortAudio](http://www.portaudio.com) as a cross-platform audio backend. The [rust-portaudio](https://github.com/jeremyletang/rust-portaudio) dependency will first try to find an already installed version on your system before trying to download it and build PortAudio itself.

License
-------

MIT - Same license as [PortAudio](http://www.portaudio.com/license.html).

