use crate::audio::{self, Resampler};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
#[derive(Default)]
pub struct AudioData {
    pub pcm: Vec<f32>,
    pub total_frames: usize,
    pub level: f32,
    pub peak: f32,
    pub native_rate: u32,
    pub channels: usize,
    pub device: String,
    pub error: String,
    pub bars: [f32; 24],
}
// Register only the actual capture callback thread, not the ASR process.
// TLS cleanup runs on the same thread, as required by MMCSS.
struct AudioScheduling(isize);
impl AudioScheduling {
    fn new() -> Self {
        use windows_sys::Win32::System::Threading::*;
        unsafe {
            let mut task = 0;
            let handle =
                AvSetMmThreadCharacteristicsW(crate::platform::wide("Audio").as_ptr(), &mut task);
            if handle.is_null() {
                SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST);
            } else {
                AvSetMmThreadPriority(handle, AVRT_PRIORITY_HIGH);
            }
            Self(handle as isize)
        }
    }
}
impl Drop for AudioScheduling {
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe {
                windows_sys::Win32::System::Threading::AvRevertMmThreadCharacteristics(self.0 as _);
            }
        }
    }
}
thread_local! { static SCHEDULING: AudioScheduling = AudioScheduling::new(); }
struct Processor {
    collect_pcm: bool,
    convert: Resampler,
    channels: usize,
    data: Arc<Mutex<AudioData>>,
}
impl Processor {
    fn push(&mut self, input: &[f32]) {
        let mono = audio::mono(input, self.channels);
        let pcm = if self.collect_pcm {
            self.convert.push(&mono)
        } else {
            Vec::new()
        };
        let level = (audio::rms(&mono) * 9.).clamp(0., 1.);
        let peak = mono.iter().fold(0f32, |a, x| a.max(x.abs()));
        let mut d = self.data.lock().unwrap();
        d.level = level;
        d.peak = d.peak.max(peak);
        d.bars.rotate_left(1);
        d.bars[23] = level;
        if !self.collect_pcm {
            d.total_frames += mono.len() * 16000 / d.native_rate.max(1) as usize;
            return;
        }
        if d.pcm.len() + pcm.len() > 16000 * 50 * 60 {
            d.error = "Aufnahmelimit von 50 Minuten erreicht.".into()
        } else {
            d.total_frames += pcm.len();
            d.pcm.extend(pcm)
        }
    }
}
pub struct Capture {
    stream: Option<cpal::Stream>,
    processor: Arc<Mutex<Processor>>,
}
impl Capture {
    pub fn open(
        name: &str,
        data: Arc<Mutex<AudioData>>,
        collect_pcm: bool,
    ) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = if name.is_empty() {
            host.default_input_device()
                .ok_or("Kein Standardmikrofon gefunden.")?
        } else {
            host.input_devices().map_err(|e|e.to_string())?.find(|d|d.name().is_ok_and(|v|v==name)).ok_or("Das gewählte Mikrofon ist nicht mehr angeschlossen. In Settings neu auswählen.")?
        };
        let config = device
            .default_input_config()
            .map_err(|e| format!("Mikrofonformat: {e}. Windows-Mikrofonzugriff prüfen."))?;
        let rate = config.sample_rate().0;
        let channels = config.channels() as usize;
        *data.lock().unwrap() = AudioData {
            native_rate: rate,
            channels,
            device: device.name().unwrap_or_default(),
            ..Default::default()
        };
        let processor = Arc::new(Mutex::new(Processor {
            collect_pcm,
            convert: Resampler::new(rate),
            channels,
            data: data.clone(),
        }));
        // Give capture 100 ms of buffering headroom while ASR uses the CPU.
        // This is below the live decode interval and prevents lost phonemes.
        let default_config: cpal::StreamConfig = config.clone().into();
        let mut buffered = default_config.clone();
        let wanted = rate / 10;
        buffered.buffer_size = cpal::BufferSize::Fixed(match config.buffer_size() {
            cpal::SupportedBufferSize::Range { min, max } => wanted.clamp(*min, *max),
            cpal::SupportedBufferSize::Unknown => wanted,
        });
        macro_rules! build {
            ($ty:ty,$conv:expr) => {{
                let make = |cfg: &cpal::StreamConfig| {
                    let p = processor.clone();
                    let d = data.clone();
                    device.build_input_stream(
                        cfg,
                        move |samples: &[$ty], _| {
                            SCHEDULING.with(|_| {});
                            let pcm = samples.iter().map($conv).collect::<Vec<f32>>();
                            p.lock().unwrap().push(&pcm)
                        },
                        move |e| d.lock().unwrap().error = format!("Mikrofon: {e}"),
                        None,
                    )
                };
                // Some drivers only support their default buffer period.
                make(&buffered).or_else(|_| make(&default_config))
            }};
        }
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => build!(f32, |v: &f32| *v),
            cpal::SampleFormat::F64 => build!(f64, |v: &f64| *v as f32),
            cpal::SampleFormat::I16 => build!(i16, |v: &i16| *v as f32 / 32768.),
            cpal::SampleFormat::U16 => build!(u16, |v: &u16| (*v as f32 - 32768.) / 32768.),
            cpal::SampleFormat::I32 => build!(i32, |v: &i32| *v as f32 / 2147483648.),
            other => return Err(format!("Nicht unterstütztes Mikrofonformat: {other:?}")),
        }
        .map_err(|e| format!("Mikrofon öffnen: {e}. Zugriff für Desktop-Apps prüfen."))?;
        Ok(Self {
            stream: Some(stream),
            processor,
        })
    }
    pub fn start(&self) -> Result<(), String> {
        self.stream
            .as_ref()
            .unwrap()
            .play()
            .map_err(|e| format!("Aufnahmestart: {e}"))
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        drop(self.stream.take());
        let mut p = self.processor.lock().unwrap();
        let tail = if p.collect_pcm {
            p.convert.finish()
        } else {
            Vec::new()
        };
        let mut d = p.data.lock().unwrap();
        d.total_frames += tail.len();
        d.pcm.extend(tail);
        d.level = 0.;
    }
}
pub fn devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|v| v.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn meter_has_bounded_memory_and_does_not_store_pcm() {
        let data = Arc::new(Mutex::new(AudioData {
            native_rate: 48000,
            ..Default::default()
        }));
        let mut p = Processor {
            collect_pcm: false,
            convert: Resampler::new(48000),
            channels: 1,
            data: data.clone(),
        };
        let chunk = vec![0.05; 480];
        for _ in 0..10000 {
            p.push(&chunk);
        }
        let d = data.lock().unwrap();
        assert_eq!(d.pcm.len(), 0);
        assert_eq!(d.pcm.capacity(), 0);
        assert_eq!(d.total_frames, 1_600_000);
        assert!(d.level > 0. && d.peak > 0.);
    }
}
