use crate::{
    asr::Recognizer,
    audio::{self, RATE},
    edit,
    platform::{
        self,
        microphone::{AudioData, Capture},
        Target,
    },
    store::{self, Entry, OutputMode, Settings, Shortcut, Store},
};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicIsize, AtomicU32, AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc, Mutex, OnceLock,
    },
    thread,
    time::{Duration, Instant},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Recording,
    Testing,
    Finalizing,
    Importing,
}
#[derive(Clone)]
pub struct State {
    pub phase: Phase,
    pub ready: bool,
    pub model_loaded: bool,
    pub status: String,
    pub error: String,
    pub warning: String,
    pub text: String,
    pub inserted: String,
    pub session: u64,
    pub anchor: Target,
    pub output_mode: OutputMode,
    pub started: Option<Instant>,
    pub inference_ms: f64,
    pub progress: u32,
    pub update: String,
}
impl Default for State {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            ready: true,
            model_loaded: false,
            status: "Bereit · Parakeet wird beim Diktieren geladen".into(),
            error: String::new(),
            warning: String::new(),
            text: String::new(),
            inserted: String::new(),
            session: 0,
            anchor: Target::default(),
            output_mode: OutputMode::Live,
            started: None,
            inference_ms: 0.,
            progress: 0,
            update: String::new(),
        }
    }
}
pub enum Control {
    Toggle,
    Test,
    Cancel,
    SetShortcut { cancel: bool, shortcut: Shortcut },
    Settings(Settings),
    Quit,
}
pub struct App {
    pub data: Mutex<Store>,
    pub state: Mutex<State>,
    pub audio: Arc<Mutex<AudioData>>,
    pub control: Sender<Control>,
    pub imports: Mutex<VecDeque<PathBuf>>,
    pub dir: PathBuf,
    pub root: PathBuf,
    pub quitting: AtomicBool,
    pub recording: AtomicBool,
    pub insertion_active: AtomicBool,
    pub input_paused: AtomicBool,
    pub record_key: AtomicU64,
    pub cancel_key: AtomicU64,
    pub capture_shortcut: AtomicU32,
    pub main_window: AtomicIsize,
    pub native_window: AtomicIsize,
    pub overlay_window: AtomicIsize,
    pub hooks_ready: AtomicBool,
    pub update_busy: AtomicBool,
    pub reload: AtomicBool,
    pub cancel_job: AtomicBool,
    pub release_model: AtomicBool,
    pub decoder: OnceLock<thread::Thread>,
    pub hook_thread: AtomicU32,
}
impl App {
    pub fn new(root: PathBuf, dir: PathBuf, data: Store) -> Arc<Self> {
        let (tx, rx) = mpsc::channel();
        let app = Arc::new(Self {
            record_key: AtomicU64::new(Self::encode_shortcut(data.settings.shortcut)),
            cancel_key: AtomicU64::new(Self::encode_shortcut(data.settings.cancel_shortcut)),
            data: Mutex::new(data),
            state: Mutex::new(State::default()),
            audio: Arc::new(Mutex::new(AudioData::default())),
            control: tx,
            imports: Mutex::new(VecDeque::new()),
            dir,
            root,
            quitting: AtomicBool::new(false),
            recording: AtomicBool::new(false),
            insertion_active: AtomicBool::new(false),
            input_paused: AtomicBool::new(false),
            capture_shortcut: AtomicU32::new(0),
            main_window: AtomicIsize::new(0),
            native_window: AtomicIsize::new(0),
            overlay_window: AtomicIsize::new(0),
            hooks_ready: AtomicBool::new(false),
            update_busy: AtomicBool::new(false),
            reload: AtomicBool::new(false),
            cancel_job: AtomicBool::new(false),
            release_model: AtomicBool::new(false),
            decoder: OnceLock::new(),
            hook_thread: AtomicU32::new(0),
        });
        let a = app.clone();
        thread::spawn(move || a.capture_loop(rx));
        let a = app.clone();
        thread::spawn(move || a.decode_loop());
        app
    }
    pub fn wake(&self) {
        if let Some(t) = self.decoder.get() {
            t.unpark();
        }
    }
    pub fn release_idle_model(&self) {
        self.release_model.store(true, Ordering::SeqCst);
        self.wake();
    }
    pub fn encode_shortcut(s: Shortcut) -> u64 {
        s.key as u64 | ((s.modifiers as u64) << 32)
    }
    pub fn decode_shortcut(s: u64) -> Shortcut {
        Shortcut {
            key: s as u32,
            modifiers: (s >> 32) as u32,
        }
    }
    pub fn settings(&self) -> Settings {
        self.data.lock().unwrap().settings.clone()
    }
    pub fn snapshot(&self) -> State {
        self.state.lock().unwrap().clone()
    }
    pub fn fail(&self, error: String) {
        let mut s = self.state.lock().unwrap();
        s.error = error.clone();
        s.status = error;
    }
    pub fn save_settings(&self, settings: Settings) -> Result<(), String> {
        settings.validate()?;
        let old = self.settings();
        if settings.autostart != old.autostart {
            platform::set_autostart(settings.autostart)?
        }
        let mut data = self.data.lock().unwrap();
        data.settings = settings.clone();
        if let Err(e) = data.save(&self.dir) {
            data.settings = old.clone();
            if settings.autostart != old.autostart {
                let _ = platform::set_autostart(old.autostart);
            }
            return Err(e);
        }
        self.record_key
            .store(Self::encode_shortcut(settings.shortcut), Ordering::SeqCst);
        self.cancel_key.store(
            Self::encode_shortcut(settings.cancel_shortcut),
            Ordering::SeqCst,
        );
        if settings.threads != old.threads {
            self.reload.store(true, Ordering::SeqCst)
        }
        self.wake();
        platform::overlay::refresh(self);
        Ok(())
    }
    pub fn shutdown(&self) {
        self.quitting.store(true, Ordering::SeqCst);
        self.insertion_active.store(false, Ordering::SeqCst);
        self.input_paused.store(true, Ordering::SeqCst);
        self.cancel_job.store(true, Ordering::SeqCst);
        let _ = self.control.send(Control::Quit);
        self.wake();
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                self.hook_thread.load(Ordering::Relaxed),
                windows_sys::Win32::UI::WindowsAndMessaging::WM_QUIT,
                0,
                0,
            );
        }
        platform::overlay::refresh(self);
    }
    pub fn queue_file(&self, path: PathBuf) -> Result<(), String> {
        if self.snapshot().phase != Phase::Idle {
            return Err("Bitte das aktuelle Diktat zuerst beenden.".into());
        }
        self.imports.lock().unwrap().push_back(path);
        self.wake();
        Ok(())
    }
    fn capture_loop(self: Arc<Self>, rx: Receiver<Control>) {
        let mut capture: Option<Capture> = None;
        while !self.quitting.load(Ordering::Relaxed) {
            let received = if capture.is_some() {
                rx.recv_timeout(Duration::from_millis(40))
            } else {
                rx.recv().map_err(|_| mpsc::RecvTimeoutError::Disconnected)
            };
            let command = match received {
                Ok(v) => Some(v),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(_) => break,
            };
            let has_command = command.is_some();
            let phase = self.snapshot().phase;
            match command {
                Some(Control::Quit) => break,
                Some(Control::Cancel) => {
                    if matches!(phase, Phase::Recording | Phase::Testing) {
                        drop(capture.take());
                        self.recording.store(false, Ordering::SeqCst);
                        self.insertion_active.store(false, Ordering::SeqCst);
                        platform::play_sound(
                            false,
                            self.settings().sounds,
                            self.settings().stop_volume,
                        );
                        let mut s = self.state.lock().unwrap();
                        s.phase = Phase::Idle;
                        s.session += 1;
                        s.text.clear();
                        s.inserted.clear();
                        s.status =
                            "Aufnahme verworfen. Bereits eingefügter Text bleibt im Zielfeld."
                                .into();
                        self.audio.lock().unwrap().pcm = Vec::new();
                    } else {
                        self.cancel_job.store(true, Ordering::SeqCst);
                    }
                }
                Some(Control::Settings(settings)) => {
                    if let Err(e) = self.save_settings(settings) {
                        self.fail(e)
                    }
                }
                Some(Control::SetShortcut { cancel, shortcut }) => {
                    let mut settings = self.settings();
                    if cancel {
                        settings.cancel_shortcut = shortcut
                    } else {
                        settings.shortcut = shortcut
                    }
                    match self.save_settings(settings) {
                        Ok(_) => {
                            self.state.lock().unwrap().status =
                                "Tastenkombination gespeichert.".into()
                        }
                        Err(e) => self.fail(e),
                    }
                }
                Some(Control::Toggle) | Some(Control::Test) => {
                    let test = matches!(command, Some(Control::Test));
                    if phase == Phase::Recording || phase == Phase::Testing {
                        drop(capture.take());
                        self.recording.store(false, Ordering::SeqCst);
                        platform::play_sound(
                            false,
                            self.settings().sounds,
                            self.settings().stop_volume,
                        );
                        let mut s = self.state.lock().unwrap();
                        if phase == Phase::Testing {
                            s.phase = Phase::Idle;
                            s.status =
                                "Mikrofontest beendet. Es wurde nichts im Verlauf gespeichert."
                                    .into();
                            self.audio.lock().unwrap().pcm = Vec::new();
                        } else {
                            s.phase = Phase::Finalizing;
                            s.status = "Diktat wird abgeschlossen …".into();
                        }
                    } else if phase == Phase::Idle {
                        let settings = self.settings();
                        let anchor = if test {
                            Target::default()
                        } else {
                            platform::target()
                        };
                        if !test && !self.snapshot().ready {
                            self.fail("Parakeet wird noch geladen. Bitte kurz warten.".into());
                            continue;
                        }
                        if !test
                            && settings.output_mode.inserts()
                            && !self.hooks_ready.load(Ordering::SeqCst)
                        {
                            self.fail(
                                "Globale Tastatur-/Fokusüberwachung ist nicht bereit.".into(),
                            );
                            continue;
                        }
                        if !test
                            && settings.output_mode.inserts()
                            && (anchor.window == 0 || anchor.focus == 0)
                        {
                            self.fail("Ein Textfeld in deinem Zielprogramm anklicken und das Aufnahme-Kürzel drücken.".into());
                            continue;
                        }
                        match Capture::open(&settings.microphone, self.audio.clone(), !test) {
                            Ok(c) => {
                                platform::play_sound(true, settings.sounds, settings.start_volume);
                                match c.start() {
                                    Ok(_) => {
                                        capture = Some(c);
                                        self.input_paused.store(false, Ordering::SeqCst);
                                        self.cancel_job.store(false, Ordering::SeqCst);
                                        self.recording.store(true, Ordering::SeqCst);
                                        self.insertion_active.store(
                                            !test && settings.output_mode.inserts(),
                                            Ordering::SeqCst,
                                        );
                                        let mut s = self.state.lock().unwrap();
                                        s.session += 1;
                                        s.phase = if test {
                                            Phase::Testing
                                        } else {
                                            Phase::Recording
                                        };
                                        s.started = Some(Instant::now());
                                        s.anchor = anchor;
                                        s.output_mode = settings.output_mode;
                                        s.text.clear();
                                        s.inserted.clear();
                                        s.error.clear();
                                        s.warning.clear();
                                        s.status = if test {
                                            "Mikrofontest · kein Verlauf und keine Texteingabe"
                                        } else {
                                            match settings.output_mode {
                                                OutputMode::Live => {
                                                    "Diktat läuft · Text wird live eingefügt"
                                                }
                                                OutputMode::Paste => {
                                                    "Diktat läuft · Einfügen nach dem Stoppen"
                                                }
                                                OutputMode::Clipboard => {
                                                    "Diktat läuft · Nur Zwischenablage"
                                                }
                                            }
                                        }
                                        .into();
                                    }
                                    Err(e) => self.fail(e),
                                }
                            }
                            Err(e) => self.fail(e),
                        }
                    } else {
                        self.fail("Das vorherige Audio wird noch verarbeitet.".into())
                    }
                }
                None => {}
            }
            if has_command {
                self.wake();
                platform::overlay::refresh(&self);
            }
            if capture.is_some() {
                let err = self.audio.lock().unwrap().error.clone();
                if !err.is_empty() {
                    drop(capture.take());
                    self.recording.store(false, Ordering::SeqCst);
                    platform::play_sound(
                        false,
                        self.settings().sounds,
                        self.settings().stop_volume,
                    );
                    let mut s = self.state.lock().unwrap();
                    s.warning = err;
                    if s.phase == Phase::Testing {
                        s.phase = Phase::Idle;
                        self.audio.lock().unwrap().pcm = Vec::new();
                    } else {
                        s.phase = Phase::Finalizing;
                        s.status =
                            "Mikrofon getrennt · vorhandenes Audio wird abgeschlossen".into();
                    }
                }
            }
        }
        drop(capture);
        self.recording.store(false, Ordering::SeqCst);
    }
    fn load_model(&self) -> Result<Recognizer, String> {
        self.state.lock().unwrap().ready = false;
        let r = Recognizer::load(
            &self.root.join("runtime"),
            &self.root.join("model"),
            self.settings().threads,
        )?;
        let mut s = self.state.lock().unwrap();
        s.ready = true;
        s.model_loaded = true;
        if s.phase == Phase::Idle {
            s.status = "Bereit · Textfeld anklicken und Aufnahme-Kürzel drücken".into();
        }
        Ok(r)
    }
    fn decode_loop(self: Arc<Self>) {
        let _ = self.decoder.set(thread::current());
        let mut model: Option<Recognizer> = None;
        let mut idle_since: Option<Instant> = None;
        let mut session = 0;
        let mut segment_start = 0usize;
        let mut stable: Vec<String> = vec![];
        let mut seen = 0;
        let mut last_decode = Instant::now() - Duration::from_secs(1);
        while !self.quitting.load(Ordering::Relaxed) {
            let mut state = self.snapshot();
            if state.phase == Phase::Idle && self.reload.swap(false, Ordering::SeqCst) {
                model.take();
                self.state.lock().unwrap().model_loaded = false;
            }
            let demand = matches!(state.phase, Phase::Recording | Phase::Finalizing)
                || !self.imports.lock().unwrap().is_empty();
            if demand && model.is_none() {
                match self.load_model() {
                    Ok(r) => model = Some(r),
                    Err(e) => {
                        self.state.lock().unwrap().ready = true;
                        self.fail(e);
                        if state.phase == Phase::Finalizing {
                            self.state.lock().unwrap().phase = Phase::Idle;
                            self.insertion_active.store(false, Ordering::SeqCst);
                        } else if state.phase == Phase::Recording {
                            let _ = self.control.send(Control::Cancel);
                        } else {
                            self.imports.lock().unwrap().clear();
                        }
                        thread::park_timeout(Duration::from_millis(100));
                        continue;
                    }
                }
                state = self.snapshot();
            }
            if state.phase == Phase::Recording {
                let model = model.as_mut().unwrap();
                if state.session != session {
                    session = state.session;
                    segment_start = 0;
                    stable.clear();
                    seen = 0;
                    last_decode = Instant::now() - Duration::from_secs(1)
                }
                let audio_len = self.audio.lock().unwrap().pcm.len();
                if audio_len >= segment_start + RATE as usize / 2
                    && audio_len > seen + 1600
                    && last_decode.elapsed() >= Duration::from_millis(350)
                {
                    let pcm = {
                        let a = self.audio.lock().unwrap();
                        a.pcm[segment_start..].to_vec()
                    };
                    let energy = audio::rms(&pcm);
                    if energy > 0.00008 {
                        // Decode up to a real pause before committing it. Audio after that
                        // pause stays in the next segment instead of losing a word in flight.
                        let pause =
                            audio::speech_pause(&pcm[..pcm.len().min(16000 * 24)], 16000 * 12);
                        let commit =
                            pause.or_else(|| (pcm.len() >= 16000 * 24).then_some(16000 * 24));
                        let decode_end = commit.unwrap_or(pcm.len().min(16000 * 90));
                        let start = Instant::now();
                        match model.recognize(&pcm[..decode_end]) {
                            Ok(text) => {
                                let still = self.snapshot();
                                if still.phase == Phase::Recording
                                    && still.session == session
                                    && !self.quitting.load(Ordering::SeqCst)
                                {
                                    let mut parts = stable.clone();
                                    parts.push(text.clone());
                                    let raw = edit::join(&parts);
                                    let rules = self.data.lock().unwrap().dictionary.clone();
                                    let text = edit::dictionary(&raw, &rules);
                                    let mut s = self.state.lock().unwrap();
                                    if s.session == session && s.phase == Phase::Recording {
                                        let outcome = if s.output_mode == OutputMode::Live {
                                            platform::apply_text(
                                                s.anchor,
                                                &s.inserted,
                                                &text,
                                                &self.input_paused,
                                            )
                                        } else {
                                            Ok(false)
                                        };
                                        s.text = text.clone();
                                        s.inference_ms = start.elapsed().as_secs_f64() * 1000.;
                                        match outcome {
                                            Ok(true) => s.inserted = text,
                                            Ok(false) => {}
                                            Err(e) => s.warning = e,
                                        }
                                    }
                                    drop(s);
                                    if let Some(end) = commit {
                                        stable.push(parts.last().cloned().unwrap_or_default());
                                        segment_start += end;
                                    }
                                }
                            }
                            Err(e) => {
                                self.state.lock().unwrap().warning = format!("Live-Erkennung: {e}");
                            }
                        }
                    } else if pcm.len() > RATE as usize {
                        // Keep a short pre-roll instead of accumulating minutes of silence
                        // before the first word. The full recording is retained for final ASR.
                        segment_start = audio_len.saturating_sub(RATE as usize / 2);
                    }
                    seen = audio_len;
                    last_decode = Instant::now();
                }
            } else if state.phase == Phase::Finalizing {
                let model = model.as_mut().unwrap();
                let pcm = self.audio.lock().unwrap().pcm.clone();
                let result = self.finish(model, &pcm, &state, "Diktat", true);
                if let Err(e) = result {
                    self.fail(e)
                }
                self.insertion_active.store(false, Ordering::SeqCst);
                let mut s = self.state.lock().unwrap();
                if s.session == state.session {
                    s.phase = Phase::Idle;
                }
                self.audio.lock().unwrap().pcm = Vec::new();
            } else if state.phase == Phase::Idle && state.ready {
                let path = self.imports.lock().unwrap().pop_front();
                if let Some(path) = path {
                    if model.is_none() {
                        match self.load_model() {
                            Ok(r) => model = Some(r),
                            Err(e) => {
                                self.state.lock().unwrap().ready = true;
                                self.fail(e);
                                continue;
                            }
                        }
                    }
                    self.cancel_job.store(false, Ordering::SeqCst);
                    {
                        let mut s = self.state.lock().unwrap();
                        s.phase = Phase::Importing;
                        s.status = "Audiodatei wird lokal erkannt …".into();
                        s.progress = 0;
                        s.error.clear();
                    }
                    let result = audio::read_file(&path).and_then(|pcm| {
                        self.finish(
                            model.as_mut().unwrap(),
                            &pcm,
                            &self.snapshot(),
                            &path.file_name().unwrap_or_default().to_string_lossy(),
                            false,
                        )
                    });
                    if let Err(e) = result {
                        self.fail(e)
                    }
                    self.state.lock().unwrap().phase = Phase::Idle;
                }
            }
            let phase = self.snapshot().phase;
            if matches!(phase, Phase::Idle | Phase::Testing)
                && self.imports.lock().unwrap().is_empty()
            {
                let idle = idle_since.get_or_insert_with(Instant::now);
                if self.release_model.swap(false, Ordering::SeqCst)
                    || (!self.settings().keep_model_warm
                        && idle.elapsed() >= Duration::from_secs(15))
                {
                    model.take();
                    self.state.lock().unwrap().model_loaded = false;
                }
                if model.is_some() && !self.settings().keep_model_warm {
                    thread::park_timeout(Duration::from_secs(15).saturating_sub(idle.elapsed()));
                } else {
                    thread::park();
                }
            } else {
                idle_since = None;
                thread::park_timeout(Duration::from_millis(30));
            }
        }
    }
    fn finish(
        &self,
        model: &mut Recognizer,
        pcm: &[f32],
        state: &State,
        name: &str,
        insert: bool,
    ) -> Result<(), String> {
        audio::validate(pcm)?;
        let now = Instant::now();
        let original = model.recognize_full(pcm, &self.cancel_job)?;
        if original.trim().is_empty() {
            return Err(
                "Keine Sprache erkannt. Mikrofonpegel und Gerät in Settings prüfen.".into(),
            );
        }
        if self.quitting.load(Ordering::SeqCst) || self.cancel_job.load(Ordering::SeqCst) {
            return Err("Abgebrochen".into());
        }
        let settings = self.settings();
        let text = edit::dictionary(&original, &self.data.lock().unwrap().dictionary);
        let mut warning = state.warning.clone();
        // Paste/clipboard modes always retain the full final text. Live mode
        // can independently disable copying without touching clipboard contents.
        let copy = settings.copy_final || (insert && state.output_mode != OutputMode::Live);
        let clipboard_ready = if copy {
            match platform::copy_text(self.native_window.load(Ordering::Relaxed), &text) {
                Ok(()) => true,
                Err(e) => {
                    warning = format!("{warning} {e}").trim().into();
                    false
                }
            }
        } else {
            false
        };
        if insert
            && state.output_mode.inserts()
            && (state.output_mode != OutputMode::Paste || clipboard_ready)
        {
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                let outcome = match state.output_mode {
                    OutputMode::Live => platform::apply_text(
                        state.anchor,
                        &state.inserted,
                        &text,
                        &self.input_paused,
                    ),
                    OutputMode::Paste => {
                        platform::paste_clipboard(state.anchor, &self.input_paused)
                    }
                    OutputMode::Clipboard => unreachable!(),
                };
                match outcome {
                    Ok(true) => break,
                    Ok(false) if Instant::now() < deadline => {
                        thread::sleep(Duration::from_millis(10))
                    }
                    Ok(false) => {
                        warning = if clipboard_ready {
                            "Kürzeltasten noch gedrückt. Text ist in der Zwischenablage; Einfügen mit Strg+V."
                        } else { "Kürzeltasten noch gedrückt. Text ist im Verlauf gespeichert." }.into();
                        break;
                    }
                    Err(e) => {
                        warning = format!("{warning} {e}").trim().into();
                        break;
                    }
                }
            }
        }
        let id = store::new_id();
        if settings.retain_audio {
            audio::write_wav(&self.dir.join(format!("{id}.wav")), pcm)?
        }
        let entry = Entry {
            id,
            created: chrono::Local::now().to_rfc3339(),
            name: name.into(),
            text: text.clone(),
            original,
            duration: pcm.len() as f64 / RATE as f64,
            processing: now.elapsed().as_secs_f64(),
            warning,
            audio: settings.retain_audio,
        };
        {
            let mut d = self.data.lock().unwrap();
            d.history.insert(0, entry.clone());
            if let Err(e) = d.save(&self.dir) {
                d.history.remove(0);
                return Err(e);
            }
        }
        let mut s = self.state.lock().unwrap();
        s.text = text;
        s.warning = entry.warning;
        s.error.clear();
        s.progress = 100;
        s.status = if insert && state.output_mode == OutputMode::Clipboard {
            "Diktat abgeschlossen · Nur Zwischenablage"
        } else if insert && self.input_paused.load(Ordering::SeqCst) {
            "Diktat gespeichert · Automatische Eingabe war pausiert"
        } else if insert {
            "Diktat abgeschlossen"
        } else {
            "Audiodatei transkribiert"
        }
        .into();
        Ok(())
    }
}
