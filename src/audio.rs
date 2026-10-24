use std::path::Path;
pub const RATE: u32 = 16000;
pub fn rms(p: &[f32]) -> f32 {
    if p.is_empty() {
        0.
    } else {
        (p.iter().map(|v| v * v).sum::<f32>() / p.len() as f32).sqrt()
    }
}
/// Locate a genuine quiet span, not a single low-energy phoneme frame.
/// Keep the cut in the middle of 200 ms of quiet; return None for continuous speech.
pub fn speech_pause(pcm: &[f32], earliest: usize) -> Option<usize> {
    let window = RATE as usize / 5;
    if pcm.len() < earliest + window {
        return None;
    }
    let limit = (rms(pcm) * 0.12).max(0.0002);
    (earliest..=pcm.len() - window)
        .step_by(RATE as usize / 50)
        .filter_map(|at| {
            let level = rms(&pcm[at..at + window]);
            (level < limit).then_some((at + window / 2, level))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(at, _)| at)
}
pub fn mono(input: &[f32], channels: usize) -> Vec<f32> {
    if channels == 0 {
        return vec![];
    }
    if channels == 1 {
        return input.to_vec();
    }
    let mut energies = vec![0f64; channels];
    let mut average_energy = 0.;
    let mut out = Vec::with_capacity(input.len() / channels);
    for frame in input.chunks_exact(channels) {
        let m = frame.iter().sum::<f32>() / channels as f32;
        out.push(m);
        average_energy += f64::from(m * m);
        for (c, x) in frame.iter().enumerate() {
            energies[c] += f64::from(x * x)
        }
    }
    let best = (0..channels).fold(0, |best, c| {
        if energies[c] > energies[best] {
            c
        } else {
            best
        }
    });
    if average_energy < energies[best] * 0.1 {
        for (i, f) in input.chunks_exact(channels).enumerate() {
            out[i] = f[best]
        }
    }
    out
}
/// Streaming 64-tap windowed-sinc converter with antialias filtering. Carries
/// its sample position and filter history across WASAPI callbacks.
pub struct Resampler {
    rate: u32,
    data: Vec<f32>,
    base: i64,
    next: f64,
    total: usize,
    emitted: usize,
}
impl Resampler {
    pub fn new(rate: u32) -> Self {
        Self {
            rate,
            data: vec![],
            base: 0,
            next: 0.,
            total: 0,
            emitted: 0,
        }
    }
    pub fn push(&mut self, pcm: &[f32]) -> Vec<f32> {
        self.total += pcm.len();
        if self.rate == RATE {
            self.emitted += pcm.len();
            return pcm.to_vec();
        }
        self.data.extend_from_slice(pcm);
        let mut out = Vec::new();
        let half = 32i64;
        let cutoff = (RATE as f64 / self.rate as f64 * 0.46).min(0.46);
        let end = self.base + self.data.len() as i64;
        while self.next.floor() as i64 + half < end {
            let center = self.next.floor() as i64;
            let mut v = 0.;
            let mut norm = 0.;
            for i in center - half + 1..=center + half {
                let d = i as f64 - self.next;
                let phase = 2. * std::f64::consts::PI * cutoff * d;
                let sinc = if phase.abs() < 1e-10 {
                    1.
                } else {
                    phase.sin() / phase
                };
                let window = 0.42
                    + 0.5 * (std::f64::consts::PI * d / half as f64).cos()
                    + 0.08 * (2. * std::f64::consts::PI * d / half as f64).cos();
                let w = sinc * window;
                norm += w;
                if i >= self.base && i < end {
                    v += f64::from(self.data[(i - self.base) as usize]) * w
                }
            }
            out.push((v / norm) as f32);
            self.next += self.rate as f64 / RATE as f64;
        }
        let drop = ((self.next.floor() as i64 - half).max(self.base) - self.base) as usize;
        if drop > 0 {
            self.data.drain(..drop.min(self.data.len()));
            self.base += drop as i64
        }
        self.emitted += out.len();
        out
    }
    pub fn finish(&mut self) -> Vec<f32> {
        if self.rate == RATE {
            return vec![];
        }
        let needed = (self.total as f64 * RATE as f64 / self.rate as f64).round() as usize;
        let remain = needed.saturating_sub(self.emitted);
        let mut out = self.push(&[0.; 96]);
        out.truncate(remain);
        out
    }
}
pub fn write_wav(path: &Path, pcm: &[f32]) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for x in pcm {
        w.write_sample((x.clamp(-1., 1.) * 32767.) as i16)
            .map_err(|e| e.to_string())?
    }
    w.finalize().map_err(|e| e.to_string())
}
pub fn read_file(path: &Path) -> Result<Vec<f32>, String> {
    use symphonia::core::{
        audio::SampleBuffer, codecs::DecoderOptions, formats::FormatOptions, io::MediaSourceStream,
        meta::MetadataOptions, probe::Hint,
    };
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > 200 * 1024 * 1024 {
        return Err("Audiodatei ist größer als 200 MB.".into());
    }
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|v| v.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            MediaSourceStream::new(Box::new(file), Default::default()),
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| e.to_string())?;
    let mut format = probed.format;
    let track = format.default_track().ok_or("Keine Audiospur gefunden.")?;
    let id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| e.to_string())?;
    let mut convert = None;
    let mut sample_rate = 0;
    let mut output = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break
            }
            Err(e) => return Err(e.to_string()),
        };
        if packet.track_id() != id {
            continue;
        }
        let audio = decoder.decode(&packet).map_err(|e| e.to_string())?;
        let spec = *audio.spec();
        if sample_rate != 0 && sample_rate != spec.rate {
            return Err("Wechselnde Samplingrate wird nicht unterstützt.".into());
        }
        sample_rate = spec.rate;
        let mut buf = SampleBuffer::<f32>::new(audio.capacity() as u64, spec);
        buf.copy_interleaved_ref(audio);
        let m = mono(buf.samples(), spec.channels.count());
        let c = convert.get_or_insert_with(|| Resampler::new(spec.rate));
        output.extend(c.push(&m));
        if output.len() > RATE as usize * 50 * 60 {
            return Err("Maximal 50 Minuten pro Audiodatei.".into());
        }
    }
    if let Some(mut c) = convert {
        output.extend(c.finish())
    }
    if output.is_empty() {
        return Err("Datei enthält keine Audiodaten.".into());
    }
    Ok(output)
}
pub fn validate(p: &[f32]) -> Result<(), String> {
    if p.len() < RATE as usize / 2 {
        return Err("Aufnahme zu kurz. Mindestens eine halbe Sekunde sprechen.".into());
    }
    if p.iter().all(|x| x.abs() < 2. / 32768.) {
        return Err(
            "Nur Stille empfangen. Mikrofon, Stummschaltung und Windows-Zugriff prüfen.".into(),
        );
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn segment_cuts_require_a_real_pause_and_keep_word_context() {
        let mut voice = vec![0.05; RATE as usize * 25];
        // A short voiceless consonant must not become a segment boundary.
        voice[16000 * 18..16000 * 18 + 320].fill(0.);
        assert_eq!(speech_pause(&voice[..16000 * 24], 16000 * 12), None);
        // A complete 400 ms pause is a safe cut, with quiet audio on both sides.
        voice[16000 * 20..16000 * 20 + 6400].fill(0.);
        let at = speech_pause(&voice[..16000 * 24], 16000 * 12).unwrap();
        assert!((16000 * 20 + 1600..=16000 * 20 + 4800).contains(&at));
        assert!(voice[at - 1600..at + 1600].iter().all(|x| *x == 0.));
        assert_eq!(speech_pause(&voice[..16000], 16000 * 12), None);
    }
    #[test]
    fn resampling_preserves_duration_and_stream_boundaries() {
        for rate in [16000, 22050, 44100, 48000, 96000] {
            let p = (0..rate)
                .map(|i| (i as f32 * 1000. * std::f32::consts::TAU / rate as f32).sin() * 0.5)
                .collect::<Vec<_>>();
            let mut r = Resampler::new(rate);
            let mut out = vec![];
            for b in p.chunks(237) {
                out.extend(r.push(b))
            }
            out.extend(r.finish());
            assert_eq!(out.len(), 16000, "{rate}");
            assert!((rms(&out) - 0.3535).abs() < 0.01);
        }
    }
    #[test]
    fn phase_inversion_does_not_cancel_speech() {
        let p = [0.5, -0.5, 0.3, -0.3];
        assert_eq!(mono(&p, 2), vec![0.5, 0.3]);
    }
    #[test]
    fn quiet_speech_is_not_discarded() {
        assert!(validate(&vec![0.; 16000]).is_err());
        assert!(validate(&vec![0.001; 16000]).is_ok());
    }
}

/// Scale our fixed PCM16 cue assets. Header/format remain identical; no OS volume mutation.
pub fn cue_wav(start: bool, volume: u8) -> Vec<u8> {
    let source: &[u8] = if start {
        include_bytes!("../assets/start.wav")
    } else {
        include_bytes!("../assets/stop.wav")
    };
    let mut bytes = source.to_vec();
    let gain = f32::from(volume.min(100)) / 100.;
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let len = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        let begin = at + 8;
        let end = begin.saturating_add(len).min(bytes.len());
        if &bytes[at..at + 4] == b"data" {
            for sample in bytes[begin..end].as_chunks_mut::<2>().0 {
                let scaled =
                    (f32::from(i16::from_le_bytes([sample[0], sample[1]])) * gain).round() as i16;
                sample.copy_from_slice(&scaled.to_le_bytes());
            }
            break;
        }
        at = end.saturating_add(len % 2);
    }
    bytes
}
#[cfg(test)]
mod cue_tests {
    use super::*;
    #[test]
    fn chimes_are_distinct_quiet_and_scale_without_format_changes() {
        use std::io::Cursor;
        let a = cue_wav(true, 100);
        let b = cue_wav(false, 100);
        assert_ne!(a, b);
        let half = cue_wav(true, 50);
        let muted = cue_wav(true, 0);
        let read = |v: Vec<u8>| {
            let r = hound::WavReader::new(Cursor::new(v)).unwrap();
            assert_eq!(r.spec().sample_rate, 44100);
            r.into_samples::<i16>()
                .map(Result::unwrap)
                .collect::<Vec<_>>()
        };
        let full = read(a);
        let h = read(half);
        let m = read(muted);
        assert!(m.iter().all(|x| *x == 0));
        assert_eq!(full.len(), 10584);
        assert!(full.iter().map(|x| i32::from(*x).abs()).max().unwrap() < 10000);
        for (x, y) in full.into_iter().zip(h) {
            assert!((f32::from(x) * 0.5 - f32::from(y)).abs() <= 0.51);
        }
    }
}
