use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Shortcut {
    pub key: u32,
    pub modifiers: u32,
}
impl Default for Shortcut {
    fn default() -> Self {
        Self {
            key: 32,
            modifiers: 3,
        }
    }
}
/// A recording freezes its output mode at start, so changing settings cannot
/// append a second complete transcript over a partially typed live session.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OutputMode {
    #[default]
    Live,
    Paste,
    Clipboard,
}
impl OutputMode {
    pub fn inserts(self) -> bool {
        self != Self::Clipboard
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub microphone: String,
    pub threads: usize,
    pub shortcut: Shortcut,
    pub cancel_shortcut: Shortcut,
    pub show_overlay: bool,
    pub sounds: bool,
    pub start_volume: u8,
    pub stop_volume: u8,
    pub close_to_background: bool,
    pub autostart: bool,
    pub auto_update: bool,
    pub copy_final: bool,
    pub output_mode: OutputMode,
    pub retain_audio: bool,
    pub keep_model_warm: bool,
    #[serde(default)]
    pub tray_setup: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            microphone: String::new(),
            threads: std::thread::available_parallelism().map_or(4, |v| v.get().min(4)),
            shortcut: Shortcut::default(),
            cancel_shortcut: Shortcut {
                key: 27,
                modifiers: 0,
            },
            show_overlay: true,
            sounds: true,
            start_volume: 35,
            stop_volume: 35,
            close_to_background: true,
            autostart: false,
            auto_update: true,
            copy_final: true,
            output_mode: OutputMode::Live,
            retain_audio: false,
            keep_model_warm: false,
            tray_setup: true,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.start_volume > 100 || self.stop_volume > 100 {
            return Err("Tonlautstärke muss zwischen 0 und 100 liegen.".into());
        }
        if !(1..=32).contains(&self.threads) {
            return Err("CPU-Threads müssen zwischen 1 und 32 liegen.".into());
        }
        for s in [self.shortcut, self.cancel_shortcut] {
            if !(8..=254).contains(&s.key) || s.modifiers > 15 {
                return Err("Ungültige Tastenkombination.".into());
            }
        }
        if normalize_key(self.shortcut.key) == normalize_key(self.cancel_shortcut.key)
            && self.shortcut.modifiers == self.cancel_shortcut.modifiers
        {
            return Err("Aufnehmen und Verwerfen benötigen unterschiedliche Kürzel.".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub created: String,
    pub name: String,
    pub text: String,
    pub original: String,
    pub duration: f64,
    pub processing: f64,
    pub warning: String,
    pub audio: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Replacement {
    pub from: String,
    pub to: String,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Store {
    pub settings: Settings,
    pub history: Vec<Entry>,
    pub dictionary: Vec<Replacement>,
}
impl Store {
    pub fn load(dir: &Path, legacy: Option<&Path>) -> Result<Self, String> {
        let path = dir.join("data.json");
        if path.exists() {
            return serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("Lokale Daten beschädigt; data.json sichern: {e}"));
        }
        let mut data = Self::default();
        if let Some(old) = legacy.filter(|p| p.join("data.json").exists()) {
            let bytes = fs::read(old.join("data.json")).map_err(|e| e.to_string())?;
            data = serde_json::from_slice(&bytes)
                .map_err(|e| format!("Bisheriger Verlauf konnte nicht übernommen werden: {e}"))?;
            // Never migrate removed models, enhancement prompts or the old numeric WinMM device ID.
            if old.file_name().is_some_and(|n| n == "VoiceInkLocal") {
                data.settings.microphone.clear();
            }
            for e in &data.history {
                if e.audio {
                    let from = old.join(format!("{}.wav", e.id));
                    if safe_id(&e.id) && from.exists() {
                        fs::copy(from, dir.join(format!("{}.wav", e.id)))
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            data.save(dir)?;
        }
        Ok(data)
    }
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let temp = dir.join(format!("data-{}.tmp", new_id()));
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        f.write_all(&bytes)
            .and_then(|_| f.sync_all())
            .map_err(|e| e.to_string())?;
        drop(f);
        fs::rename(&temp, dir.join("data.json")).map_err(|e| {
            let _ = fs::remove_file(&temp);
            e.to_string()
        })
    }
}
pub fn safe_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 80 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}
pub fn new_id() -> String {
    static N: AtomicU64 = AtomicU64::new(0);
    format!(
        "{:x}{:x}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        N.fetch_add(1, Ordering::Relaxed)
    )
}
pub fn normalize_key(k: u32) -> u32 {
    match k {
        160 | 161 => 16,
        162 | 163 => 17,
        164 | 165 => 18,
        92 => 91,
        _ => k,
    }
}
pub fn modifier(k: u32) -> u32 {
    match normalize_key(k) {
        16 => 4,
        17 => 2,
        18 => 1,
        91 => 8,
        _ => 0,
    }
}
pub fn shortcut_matches(s: Shortcut, k: u32, mods: u32) -> bool {
    normalize_key(s.key) == normalize_key(k) && (mods & !modifier(k)) == s.modifiers
}
pub fn shortcut_label(s: Shortcut) -> String {
    let mut p = Vec::new();
    for (m, n) in [(2, "Strg"), (1, "Alt"), (4, "Umschalt"), (8, "Windows")] {
        if s.modifiers & m != 0 {
            p.push(n.to_string())
        }
    }
    p.push(match s.key {
        27 => "Esc".into(),
        32 => "Leertaste".into(),
        13 => "Enter".into(),
        16 => "Umschalt".into(),
        17 => "Strg".into(),
        18 => "Alt".into(),
        91 | 92 => "Windows".into(),
        112..=135 => format!("F{}", s.key - 111),
        k => char::from_u32(k).unwrap_or('?').to_string(),
    });
    p.join(" + ")
}
#[derive(Default, Debug)]
pub struct Metrics {
    pub sessions: usize,
    pub words: usize,
    pub seconds: f64,
    pub saved: f64,
    pub wpm: f64,
    pub keystrokes: usize,
}
pub fn metrics(h: &[Entry]) -> Metrics {
    let words = h
        .iter()
        .map(|e| e.text.split_whitespace().count())
        .sum::<usize>();
    let seconds = h.iter().map(|e| e.duration.max(0.)).sum::<f64>();
    Metrics {
        sessions: h.len(),
        words,
        seconds,
        saved: (words as f64 / 35. * 60. - seconds).max(0.),
        wpm: if seconds > 0. {
            words as f64 / seconds * 60.
        } else {
            0.
        },
        keystrokes: words * 5,
    }
}
