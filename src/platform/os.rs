use std::{
    ffi::c_void,
    path::{Path, PathBuf},
    ptr::{null, null_mut},
    thread,
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::*,
    Media::Audio::*,
    System::{DataExchange::*, LibraryLoader::*, Memory::*, Registry::*},
    UI::{Controls::Dialogs::*, Shell::*, WindowsAndMessaging::*},
};
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub fn error_box(s: &str) {
    unsafe {
        MessageBoxW(
            null_mut(),
            wide(s).as_ptr(),
            wide("Diktator").as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
pub fn copy_text(owner: isize, text: &str) -> Result<(), String> {
    unsafe {
        let mut opened = false;
        for _ in 0..15 {
            if OpenClipboard(owner as HWND) != 0 {
                opened = true;
                break;
            }
            thread::sleep(Duration::from_millis(20))
        }
        if !opened {
            return Err("Zwischenablage ist belegt. Text bleibt im Verlauf.".into());
        }
        let u = wide(text);
        let handle = GlobalAlloc(GMEM_MOVEABLE, u.len() * 2);
        if handle.is_null() {
            CloseClipboard();
            return Err("Zwischenablagespeicher fehlt.".into());
        }
        let ptr = GlobalLock(handle);
        if ptr.is_null() {
            GlobalFree(handle);
            CloseClipboard();
            return Err("Zwischenablage nicht verfügbar.".into());
        }
        std::ptr::copy_nonoverlapping(u.as_ptr(), ptr.cast(), u.len());
        GlobalUnlock(handle);
        if EmptyClipboard() == 0 || SetClipboardData(13, handle).is_null() {
            GlobalFree(handle);
            CloseClipboard();
            return Err("Text konnte nicht kopiert werden.".into());
        }
        CloseClipboard();
        Ok(())
    }
}
pub fn open_resource(owner: isize, value: &str) -> Result<(), String> {
    unsafe {
        let r = ShellExecuteW(
            owner as HWND,
            wide("open").as_ptr(),
            wide(value).as_ptr(),
            null(),
            null(),
            SW_SHOWNORMAL,
        );
        if r as isize <= 32 {
            Err("Windows konnte das Ziel nicht öffnen.".into())
        } else {
            Ok(())
        }
    }
}
pub fn file_dialog(owner: isize, save: bool) -> Option<PathBuf> {
    unsafe {
        let mut file = [0u16; 32768];
        if save {
            let n = wide("Diktat.txt");
            file[..n.len()].copy_from_slice(&n)
        }
        let filter = if save {
            wide("Textdateien\0*.txt\0JSON-Dateien\0*.json\0\0")
        } else {
            wide("Audio\0*.wav;*.mp3;*.m4a;*.aac;*.flac;*.ogg;*.aiff\0Alle Dateien\0*.*\0\0")
        };
        let mut info: OPENFILENAMEW = std::mem::zeroed();
        info.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        info.hwndOwner = owner as HWND;
        info.lpstrFilter = filter.as_ptr();
        info.lpstrFile = file.as_mut_ptr();
        info.nMaxFile = file.len() as u32;
        info.Flags = OFN_NOCHANGEDIR
            | OFN_PATHMUSTEXIST
            | if save {
                OFN_OVERWRITEPROMPT
            } else {
                OFN_FILEMUSTEXIST
            };
        let ext = wide(if save { "txt" } else { "wav" });
        info.lpstrDefExt = ext.as_ptr();
        let ok = if save {
            GetSaveFileNameW(&mut info)
        } else {
            GetOpenFileNameW(&mut info)
        };
        if ok == 0 {
            None
        } else {
            Some(PathBuf::from(String::from_utf16_lossy(
                &file[..file.iter().position(|v| *v == 0).unwrap_or(file.len())],
            )))
        }
    }
}
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    unsafe {
        let mut key: HKEY = null_mut();
        let path = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let code = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            path.as_ptr(),
            0,
            null(),
            0,
            KEY_SET_VALUE | KEY_QUERY_VALUE,
            null(),
            &mut key,
            null_mut(),
        );
        if code != 0 {
            return Err(format!("Autostart: Windows-Fehler {code}"));
        }
        let name = wide("Diktator");
        let result = if enabled {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let value = wide(&format!("\"{}\" --autostart", exe.display()));
            RegSetValueExW(
                key,
                name.as_ptr(),
                0,
                REG_SZ,
                value.as_ptr().cast(),
                (value.len() * 2) as u32,
            )
        } else {
            RegDeleteValueW(key, name.as_ptr())
        };
        if enabled {
            let old = wide("VoiceInkLocal");
            let mut size = 0u32;
            if RegQueryValueExW(key, old.as_ptr(), null(), null_mut(), null_mut(), &mut size) == 0
                && size < 32768
            {
                let mut value = vec![0u16; (size as usize).div_ceil(2)];
                if RegQueryValueExW(
                    key,
                    old.as_ptr(),
                    null(),
                    null_mut(),
                    value.as_mut_ptr().cast(),
                    &mut size,
                ) == 0
                {
                    let s = String::from_utf16_lossy(&value).to_lowercase();
                    if s.contains("\\programs\\voiceinklocal\\") && s.contains("voiceink-local.exe")
                    {
                        RegDeleteValueW(key, old.as_ptr());
                    }
                }
            }
        }
        RegCloseKey(key);
        if result == 0 || (!enabled && result == 2) {
            Ok(())
        } else {
            Err(format!("Autostart konnte nicht geändert werden: {result}"))
        }
    }
}
pub fn play_sound(start: bool, enabled: bool, volume: u8) {
    if !enabled || volume == 0 {
        return;
    }
    // Stop/preview playback owns its scaled buffer until synchronous PlaySound returns.
    // Never pass a temporary Vec to SND_ASYNC or change the system-wide output volume.
    let play = move || {
        let bytes = crate::audio::cue_wav(start, volume);
        unsafe {
            PlaySoundW(
                bytes.as_ptr().cast(),
                null_mut(),
                SND_MEMORY | SND_NODEFAULT | SND_SYNC,
            );
        }
    };
    if start {
        play();
    } else {
        std::thread::spawn(play);
    }
}
pub fn data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Diktator")
}
pub fn old_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let previous = base.join("Silbenlot");
    if previous.join("data.json").exists() {
        previous
    } else {
        base.join("VoiceInkLocal")
    }
}
pub fn main_module() -> HINSTANCE {
    unsafe { GetModuleHandleW(null()) }
}
pub fn output_wav(path: &Path) {
    let _ = open_resource(0, &path.to_string_lossy());
}
pub fn _unused(_: *const c_void) {}
