//! Diagnostic CLI: transcribe_file <model-dir> <runtime-dir> <audio> [...]
//! Uses the same local model, speech gate and resampling as Silbenlot.
use diktator::{asr::Recognizer, audio};
use std::{path::Path, sync::atomic::AtomicBool};
fn main() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 3 {
        return Err("model-dir runtime-dir audio [...]".into());
    }
    let mut r = Recognizer::load(Path::new(&args[1]), Path::new(&args[0]), 4)?;
    for file in &args[2..] {
        let pcm = audio::read_file(Path::new(file))?;
        let text = r.recognize_full(&pcm, &AtomicBool::new(false))?;
        println!("{file}: {text}");
    }
    Ok(())
}
