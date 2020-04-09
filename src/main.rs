//! Feeds back the input stream directly into the output stream.
//!
//! Assumes that the input and output devices can use the same stream format and that they support
//! the f32 sample format.
//!
//! Uses a delay of `LATENCY_MS` milliseconds in case the default input and output streams are not
//! precisely synchronised.

extern crate anyhow;
extern crate cpal;
extern crate ringbuf;

use cpal::traits::{DeviceTrait, EventLoopTrait, HostTrait};
use ringbuf::RingBuffer;
use pitch_calc::Hz;


// So i need an input stream and no output stream
// input stream goes into the ringbuffer
// it is pulled out in chunks len == 2048 and then handed to pitch
// let (hz, amplitude) = pitch::detect(&samples)


fn main() -> Result<(), anyhow::Error> {
    let host = cpal::default_host();
    let event_loop = host.event_loop();

    // Default devices.
    let input_device = host.default_input_device().expect("failed to get default input device");
    println!("Using default input device: \"{}\"", input_device.name()?);

    // We'll try and use the same format between streams to keep it simple
    let mut format = input_device.default_input_format()?;
    format.data_type = cpal::SampleFormat::F32;

    // Build streams.
    println!("Attempting to build both streams with `{:?}`.", format);
    let input_stream_id = event_loop.build_input_stream(&input_device, &format)?;
    println!("Successfully built streams.");



    // The buffer to share samples
    let ring = RingBuffer::new(2048 * 16);
    let (mut producer, mut consumer) = ring.split();

    // Play the streams.
    event_loop.play_stream(input_stream_id.clone())?;
   // event_loop.play_stream(output_stream_id.clone())?;

    // Run the event loop on a separate thread.
    std::thread::spawn(move || {
        event_loop.run(move |id, result| {

            let data = match result {
                Ok(data) => data,
                Err(err) => {
                    eprintln!("an error occurred on stream {:?}: {}", id, err);
                    return;
                }
            };

            match data {
                cpal::StreamData::Input { buffer: cpal::UnknownTypeInputBuffer::F32(buffer) } => {
                    assert_eq!(id, input_stream_id);
                    for &sample in buffer.iter() {
                        if producer.push(sample).is_err() {
                            panic!("overfilled the buffer")
                        }
                    }
                },
                // We don't have an output stream
                cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::F32(mut buffer) } => {

                },
                _ => panic!("we're expecting f32 data"),
            }
        });
    });

    loop {
      //  println!("{}", consumer.len());
        if (consumer.len() > 2048 * 2) {
            let mut accum = vec![];
            //accum.push(1.0 as f64);
            for i in 0..2048 * 2 {
                match consumer.pop() {
                    Ok(s) => accum.push((s) as f64),
                    Err(err) => {
                        panic!("somehow we overran the ringbuffer");
                    },
                }
            }
            let (hz, amplitude) = pitch::detect(accum.as_slice());
            if (hz < 1100.0){
                println!("{:.2} ==== {:?}",hz, Hz(hz as f32).letter_octave());
            }

        }


    }
}