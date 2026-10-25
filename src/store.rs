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
        let s = self.shortcut;
        if !(8..=254).contains(&s.key) || s.modifiers & !0xfff != 0 {
            return Err("Ungültige Tastenkombination.".into());
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
/// Specific side bits coexist with the original generic modifier bits (1..8).
/// Existing shortcuts keep accepting either side; newly captured shortcuts are exact.
pub fn side_modifier(k: u32) -> u32 {
    match k {
        162 => 16,
        163 => 32,
        164 => 64,
        165 => 128,
        160 => 256,
        161 => 512,
        91 => 1024,
        92 => 2048,
        _ => 0,
    }
}
pub fn physical_key(vk: u32, scan: u32, extended: bool) -> u32 {
    match vk {
        17 => {
            if extended {
                163
            } else {
                162
            }
        }
        18 => {
            if extended {
                165
            } else {
                164
            }
        }
        16 => {
            if scan == 0x36 {
                161
            } else {
                160
            }
        }
        _ => vk,
    }
}
pub fn without_key(mods: u32, key: u32) -> u32 {
    mods & !side_modifier(key) & !modifier(key)
}
pub fn shortcut_matches(s: Shortcut, k: u32, mods: u32) -> bool {
    let same = if matches!(s.key, 16..=18) {
        s.key == normalize_key(k)
    } else {
        s.key == k
    };
    if !same {
        return false;
    }
    let actual = without_key(mods, k);
    for (generic, sides) in [(1, 192), (2, 48), (4, 768), (8, 3072)] {
        let expected = s.modifiers & (generic | sides);
        let held = actual & (generic | sides);
        if if expected & sides != 0 {
            held & sides != expected & sides
        } else if expected & generic != 0 {
            held == 0
        } else {
            held != 0
        } {
            return false;
        }
    }
    true
}
pub fn shortcut_label(s: Shortcut) -> String {
    let mut p = Vec::new();
    for (bit, label) in [
        (2, "Strg"),
        (16, "Strg links"),
        (32, "Strg rechts"),
        (1, "Alt"),
        (64, "Alt links"),
        (128, "Alt rechts / AltGr"),
        (4, "Umschalt"),
        (256, "Umschalt links"),
        (512, "Umschalt rechts"),
        (8, "Windows"),
        (1024, "Windows links"),
        (2048, "Windows rechts"),
    ] {
        if s.modifiers & bit != 0 {
            p.push(label.to_string());
        }
    }
    p.push(match s.key {
        8 => "Rücktaste".into(),
        9 => "Tab".into(),
        27 => "Esc".into(),
        32 => "Leertaste".into(),
        13 => "Enter".into(),
        20 => "Feststelltaste".into(),
        33 => "Bild auf".into(),
        34 => "Bild ab".into(),
        35 => "Ende".into(),
        36 => "Pos1".into(),
        37 => "Pfeil links".into(),
        38 => "Pfeil hoch".into(),
        39 => "Pfeil rechts".into(),
        40 => "Pfeil runter".into(),
        44 => "Druck".into(),
        45 => "Einfg".into(),
        46 => "Entf".into(),
        96..=105 => format!("Num {}", s.key - 96),
        106 => "Num *".into(),
        107 => "Num +".into(),
        109 => "Num −".into(),
        110 => "Num Dezimal".into(),
        111 => "Num /".into(),
        16 => "Umschalt".into(),
        17 => "Strg".into(),
        18 => "Alt".into(),
        160 => "Umschalt links".into(),
        161 => "Umschalt rechts".into(),
        162 => "Strg links".into(),
        163 => "Strg rechts".into(),
        164 => "Alt links".into(),
        165 => "Alt rechts / AltGr".into(),
        91 => "Windows links".into(),
        92 => "Windows rechts".into(),
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
