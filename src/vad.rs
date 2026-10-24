//! Local speech gate using the bundled Silero model and the existing sherpa DLL.
//! It rejects recordings without speech. Once speech is found, retain the audio
//! context: a missed quiet phrase must not be trimmed out of a valid recording.
use libloading::Library;
use std::{
    ffi::{c_char, c_void, CString},
    path::Path,
};
type Handle = *const c_void;
type Action = unsafe extern "C" fn(Handle);
#[repr(C)]
#[derive(Default)]
struct ModelConfig {
    model: *const c_char,
    threshold: f32,
    min_silence_duration: f32,
    min_speech_duration: f32,
    window_size: i32,
    max_speech_duration: f32,
}
#[repr(C)]
struct Config {
    silero: ModelConfig,
    sample_rate: i32,
    threads: i32,
    provider: *const c_char,
    debug: i32,
    ten: ModelConfig,
}
#[repr(C)]
struct Segment {
    start: i32,
    samples: *mut f32,
    n: i32,
}
pub struct SpeechGate {
    handle: Handle,
    destroy: Action,
    reset: Action,
    accept: unsafe extern "C" fn(Handle, *const f32, i32),
    flush: Action,
    empty: unsafe extern "C" fn(Handle) -> i32,
    front: unsafe extern "C" fn(Handle) -> *const Segment,
    free: unsafe extern "C" fn(*const Segment),
    pop: Action,
}
impl SpeechGate {
    /// The library must outlive this gate; Recognizer owns both in that order.
    pub unsafe fn load(lib: &Library, path: &Path) -> Result<Self, String> {
        type Create = unsafe extern "C" fn(*const Config, f32) -> Handle;
        let create: Create = *lib
            .get(b"SherpaOnnxCreateVoiceActivityDetector\0")
            .map_err(|e| e.to_string())?;
        let path = CString::new(path.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
        let config = Config {
            silero: ModelConfig {
                model: path.as_ptr(),
                threshold: 0.6,
                min_silence_duration: 0.45,
                min_speech_duration: 0.16,
                window_size: 512,
                max_speech_duration: 90.,
            },
            sample_rate: 16000,
            threads: 1,
            provider: c"cpu".as_ptr(),
            debug: 0,
            ten: ModelConfig::default(),
        };
        macro_rules! get {
            ($name:literal) => {
                *lib.get(concat!($name, "\0").as_bytes())
                    .map_err(|e| e.to_string())?
            };
        }
        // Resolve every symbol before allocating the detector.
        let mut out = Self {
            handle: std::ptr::null(),
            destroy: get!("SherpaOnnxDestroyVoiceActivityDetector"),
            reset: get!("SherpaOnnxVoiceActivityDetectorReset"),
            accept: get!("SherpaOnnxVoiceActivityDetectorAcceptWaveform"),
            flush: get!("SherpaOnnxVoiceActivityDetectorFlush"),
            empty: get!("SherpaOnnxVoiceActivityDetectorEmpty"),
            front: get!("SherpaOnnxVoiceActivityDetectorFront"),
            free: get!("SherpaOnnxDestroySpeechSegment"),
            pop: get!("SherpaOnnxVoiceActivityDetectorPop"),
        };
        out.handle = create(&config, 100.);
        if out.handle.is_null() {
            return Err(
                "Lokaler Sprachfilter konnte nicht geladen werden. Setup erneut ausführen.".into(),
            );
        }
        Ok(out)
    }
    pub fn has_speech(&mut self, pcm: &[f32]) -> bool {
        if pcm.is_empty() {
            return false;
        }
        unsafe {
            (self.reset)(self.handle);
            for frame in pcm.chunks(512) {
                (self.accept)(self.handle, frame.as_ptr(), frame.len() as i32);
            }
            (self.flush)(self.handle);
            let mut detected = false;
            while (self.empty)(self.handle) == 0 {
                let seg = (self.front)(self.handle);
                if !seg.is_null() {
                    detected |= (*seg).n > 0;
                    (self.free)(seg);
                }
                (self.pop)(self.handle);
            }
            detected
        }
    }
}
impl Drop for SpeechGate {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { (self.destroy)(self.handle) }
        }
    }
}
