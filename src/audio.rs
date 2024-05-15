use std::fs::File;
use std::io::BufReader;
use std::sync::atomic;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::Duration;
use rodio::Decoder;
use rodio::Sink;

pub fn play_audio(file: &str, stop: &AtomicBool) {
    let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
    let file = File::open(file).unwrap();

    let s = stream_handle.play_once(BufReader::new(file)).unwrap();

    loop {
        if stop.load(atomic::Ordering::SeqCst) {
            break;
        }
    }

    drop(s);
}