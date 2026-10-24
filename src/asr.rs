#[path = "asr_ffi.rs"]
mod ffi;
use ffi::*;
use libloading::Library;
use std::{
    ffi::{c_void, CStr, CString},
    path::Path,
    time::Instant,
};
type Handle = *const c_void;
type Create = unsafe extern "C" fn(*const SherpaOnnxOfflineRecognizerConfig) -> Handle;
type Destroy = unsafe extern "C" fn(Handle);
type Stream = unsafe extern "C" fn(Handle) -> Handle;
type Accept = unsafe extern "C" fn(Handle, i32, *const f32, i32);
type Decode = unsafe extern "C" fn(Handle, Handle);
type ResultFn = unsafe extern "C" fn(Handle) -> *const SherpaOnnxOfflineRecognizerResult;
type DestroyResult = unsafe extern "C" fn(*const SherpaOnnxOfflineRecognizerResult);
pub struct Recognizer {
    handle: Handle,
    destroy: Destroy,
    stream: Stream,
    stream_destroy: Destroy,
    accept: Accept,
    decode: Decode,
    result: ResultFn,
    result_destroy: DestroyResult,
    speech_gate: crate::vad::SpeechGate,
    _library: Library,
}
impl Recognizer {
    pub fn load(runtime: &Path, model: &Path, threads: usize) -> Result<Self, String> {
        unsafe {
            #[cfg(windows)]
            let lib: Library = libloading::os::windows::Library::load_with_flags(
                runtime.join("sherpa-onnx-c-api.dll"),
                0x00000100 | 0x00001000,
            )
            .map_err(|e| format!("KI-Bibliothek: {e}"))?
            .into();
            #[cfg(not(windows))]
            let lib = Library::new(runtime.join("libsherpa-onnx-c-api.dylib"))
                .map_err(|e| e.to_string())?;
            let speech_gate = crate::vad::SpeechGate::load(&lib, &model.join("silero_vad.onnx"))?;
            let create: Create = *lib
                .get(b"SherpaOnnxCreateOfflineRecognizer\0")
                .map_err(|e| e.to_string())?;
            let paths = [
                "encoder.int8.onnx",
                "decoder.onnx",
                "joiner.onnx",
                "tokens.txt",
            ]
            .map(|n| {
                CString::new(model.join(n).to_string_lossy().as_bytes()).map_err(|e| e.to_string())
            })
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
            let cpu = CString::new("cpu").unwrap();
            let kind = CString::new("nemo_transducer").unwrap();
            let search = CString::new("greedy_search").unwrap();
            let mut config = SherpaOnnxOfflineRecognizerConfig::default();
            config.feat_config.sample_rate = 16000;
            config.feat_config.feature_dim = 80;
            config.model_config.transducer.encoder = paths[0].as_ptr();
            config.model_config.transducer.decoder = paths[1].as_ptr();
            config.model_config.transducer.joiner = paths[2].as_ptr();
            config.model_config.tokens = paths[3].as_ptr();
            config.model_config.num_threads = threads.clamp(1, 32) as i32;
            config.model_config.provider = cpu.as_ptr();
            config.model_config.model_type = kind.as_ptr();
            config.decoding_method = search.as_ptr();
            config.max_active_paths = 4;
            let start = Instant::now();
            let handle = create(&config);
            if handle.is_null() {
                return Err("Parakeet v3 konnte nicht geladen werden. Modellintegrität und freien Arbeitsspeicher prüfen.".into());
            }
            macro_rules! get {
                ($name:literal) => {
                    *lib.get(concat!($name, "\0").as_bytes())
                        .map_err(|e| e.to_string())?
                };
            }
            let out = Self {
                handle,
                destroy: get!("SherpaOnnxDestroyOfflineRecognizer"),
                stream: get!("SherpaOnnxCreateOfflineStream"),
                stream_destroy: get!("SherpaOnnxDestroyOfflineStream"),
                accept: get!("SherpaOnnxAcceptWaveformOffline"),
                decode: get!("SherpaOnnxDecodeOfflineStream"),
                result: get!("SherpaOnnxGetOfflineStreamResult"),
                result_destroy: get!("SherpaOnnxDestroyOfflineRecognizerResult"),
                speech_gate,
                _library: lib,
            };
            eprintln!(
                "Parakeet warm loaded in {:.2}s",
                start.elapsed().as_secs_f64()
            );
            Ok(out)
        }
    }
    pub fn recognize(&mut self, pcm: &[f32]) -> Result<String, String> {
        if pcm.is_empty() {
            return Ok(String::new());
        }
        if pcm.len() > 16000 * 95 {
            return Err("ASR-Segment ist zu lang.".into());
        }
        if !self.speech_gate.has_speech(pcm) {
            return Ok(String::new());
        }
        // Preserve quiet sentence starts and language context. The VAD is a
        // speech/no-speech gate, not a destructive trim. Synthetic silence gives
        // the offline encoder clean edges without waiting for more live audio.
        let mut padded = vec![0.; 2560];
        padded.extend_from_slice(pcm);
        padded.resize(padded.len() + 5120, 0.);
        let pcm = &padded;
        unsafe {
            let stream = (self.stream)(self.handle);
            if stream.is_null() {
                return Err("Erkennungsstream konnte nicht erstellt werden.".into());
            }
            (self.accept)(stream, 16000, pcm.as_ptr(), pcm.len() as i32);
            (self.decode)(self.handle, stream);
            let result = (self.result)(stream);
            let text = if result.is_null() {
                Err("Keine Modellausgabe.".into())
            } else if (*result).text.is_null() {
                Ok(String::new())
            } else {
                Ok(CStr::from_ptr((*result).text)
                    .to_string_lossy()
                    .trim()
                    .to_string())
            };
            if !result.is_null() {
                (self.result_destroy)(result)
            }
            (self.stream_destroy)(stream);
            text
        }
    }
    pub fn recognize_full(
        &mut self,
        pcm: &[f32],
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<String, String> {
        let mut texts = Vec::new();
        let mut start = 0;
        while start < pcm.len() {
            if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                return Err("Abgebrochen".into());
            }
            let preferred = (start + 16000 * 24).min(pcm.len());
            let pause = (preferred < pcm.len())
                .then(|| crate::audio::speech_pause(&pcm[start..preferred], 16000 * 12))
                .flatten();
            let mut end = pause.map_or((start + 16000 * 85).min(pcm.len()), |at| start + at);
            if pause.is_none() && end < pcm.len() {
                let low = (end - 16000 * 5).max(start);
                let step = 320;
                let mut best = f32::MAX;
                for pos in (low..end).step_by(step) {
                    let e = crate::audio::rms(&pcm[pos..(pos + step).min(pcm.len())]);
                    if e < best {
                        best = e;
                        end = pos + step
                    }
                }
            }
            texts.push(self.recognize(&pcm[start..end])?);
            start = end;
        }
        Ok(crate::edit::join(&texts))
    }
}
impl Drop for Recognizer {
    fn drop(&mut self) {
        unsafe { (self.destroy)(self.handle) }
    }
}
