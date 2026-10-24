use crate::{
    app::{App, Control},
    edit,
    store::{self, Shortcut},
};
use std::{
    ptr::null_mut,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, OnceLock,
    },
    thread,
};
use windows_sys::Win32::{
    Foundation::*,
    System::Threading::*,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};
const OUR_INPUT: usize = 0xD17A7040;
static APP: OnceLock<Arc<App>> = OnceLock::new();
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Target {
    pub window: isize,
    pub focus: isize,
}
pub fn target() -> Target {
    unsafe {
        let top = GetForegroundWindow();
        if top.is_null() {
            return Target::default();
        }
        let mut pid = 0;
        let tid = GetWindowThreadProcessId(top, &mut pid);
        if pid == GetCurrentProcessId()
            || APP
                .get()
                .is_some_and(|a| a.main_window.load(Ordering::Relaxed) == top as isize)
        {
            return Target::default();
        }
        let mut info: GUITHREADINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
        if GetGUIThreadInfo(tid, &mut info) == 0 {
            return Target::default();
        }
        Target {
            window: top as isize,
            focus: info.hwndFocus as isize,
        }
    }
}
pub fn modifiers() -> u32 {
    unsafe {
        let down = |k| GetAsyncKeyState(k) as u16 & 0x8000 != 0;
        (if down(0x12) { 1 } else { 0 })
            | (if down(0x11) { 2 } else { 0 })
            | (if down(0x10) { 4 } else { 0 })
            | (if down(0x5b) || down(0x5c) { 8 } else { 0 })
    }
}
fn key(vk: u16, scan: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: OUR_INPUT,
            },
        },
    }
}
fn insertion_ready(anchor: Target, paused: &AtomicBool) -> Result<bool, String> {
    if paused.load(Ordering::SeqCst) {
        return Err("Automatische Eingabe pausiert: Textfeld oder Tastatur wurde während des Diktats bedient. Fertiger Text bleibt im Verlauf.".into());
    }
    if anchor.window == 0 || anchor.focus == 0 || target() != anchor {
        paused.store(true, Ordering::SeqCst);
        return Err(
            "Textfeld hat gewechselt. Es wird nichts in ein anderes Feld geschrieben.".into(),
        );
    }
    if modifiers() != 0 {
        return Ok(false);
    }
    Ok(true)
}
/// Paste only a clipboard value written successfully by the caller. The tagged
/// Ctrl+V sequence bypasses our physical-input pause hook, never the focus guard.
pub fn paste_clipboard(anchor: Target, paused: &AtomicBool) -> Result<bool, String> {
    if !insertion_ready(anchor, paused)? {
        return Ok(false);
    }
    let events = [
        key(VK_CONTROL, 0, 0),
        key(0x56, 0, 0),
        key(0x56, 0, KEYEVENTF_KEYUP),
        key(VK_CONTROL, 0, KEYEVENTF_KEYUP),
    ];
    let count = unsafe {
        SendInput(
            events.len() as u32,
            events.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    if count != events.len() as u32 {
        // Always release our Ctrl key if Windows accepted only part of the batch.
        let release = key(VK_CONTROL, 0, KEYEVENTF_KEYUP);
        unsafe {
            SendInput(1, &release, std::mem::size_of::<INPUT>() as i32);
        }
        paused.store(true, Ordering::SeqCst);
        return Err("Windows hat automatisches Einfügen blockiert. Text bleibt in der Zwischenablage und im Verlauf.".into());
    }
    Ok(true)
}
pub fn apply_text(
    anchor: Target,
    old: &str,
    new: &str,
    paused: &AtomicBool,
) -> Result<bool, String> {
    if old == new {
        return Ok(true);
    }
    if !insertion_ready(anchor, paused)? {
        return Ok(false);
    }
    let p = edit::patch(old, new);
    let mut events = Vec::with_capacity(p.backspaces * 2 + p.insert.len() * 2);
    for _ in 0..p.backspaces {
        events.push(key(8, 0, 0));
        events.push(key(8, 0, KEYEVENTF_KEYUP));
    }
    for ch in p.insert.encode_utf16() {
        events.push(key(0, ch, KEYEVENTF_UNICODE));
        events.push(key(0, ch, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
    }
    if paused.load(Ordering::SeqCst) || target() != anchor {
        paused.store(true, Ordering::SeqCst);
        return Err("Eingabe wurde vor dem Einfügen pausiert.".into());
    }
    if events.is_empty() {
        return Ok(true);
    }
    let count = unsafe {
        SendInput(
            events.len() as u32,
            events.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    if count != events.len() as u32 {
        paused.store(true, Ordering::SeqCst);
        return Err("Windows hat die Texteingabe blockiert. Das Zielprogramm darf nicht mit höheren Administratorrechten laufen.".into());
    }
    Ok(true)
}
struct Pending {
    key: u32,
    used: bool,
    cancel: bool,
}
thread_local! {static PENDING:std::cell::RefCell<Option<Pending>>=const{std::cell::RefCell::new(None)};static HELD:std::cell::Cell<u32>=const{std::cell::Cell::new(0)};static CAPTURE_KEY:std::cell::Cell<Option<Shortcut>>=const{std::cell::Cell::new(None)};}
unsafe extern "system" fn keyboard(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(null_mut(), code, wp, lp);
    }
    let Some(a) = APP.get() else {
        return CallNextHookEx(null_mut(), code, wp, lp);
    };
    let d = &*(lp as *const KBDLLHOOKSTRUCT);
    if d.dwExtraInfo == OUR_INPUT {
        return CallNextHookEx(null_mut(), code, wp, lp);
    }
    let down = wp as u32 == WM_KEYDOWN || wp as u32 == WM_SYSKEYDOWN;
    let up = wp as u32 == WM_KEYUP || wp as u32 == WM_SYSKEYUP;
    let capture = a.capture_shortcut.load(Ordering::Relaxed);
    if capture > 0 && GetForegroundWindow() as isize != a.main_window.load(Ordering::Relaxed) {
        a.capture_shortcut.store(0, Ordering::SeqCst);
        CAPTURE_KEY.with(|p| p.set(None));
        return CallNextHookEx(null_mut(), code, wp, lp);
    }
    if capture > 0 {
        if down {
            let s = Shortcut {
                key: store::normalize_key(d.vkCode),
                modifiers: modifiers() & !store::modifier(d.vkCode),
            };
            if store::modifier(d.vkCode) == 0 {
                a.capture_shortcut.store(0, Ordering::SeqCst);
                let _ = a.control.send(Control::SetShortcut {
                    cancel: capture == 2,
                    shortcut: s,
                });
                CAPTURE_KEY.with(|p| p.set(None));
            } else {
                CAPTURE_KEY.with(|p| p.set(Some(s)));
            }
        }
        if up {
            CAPTURE_KEY.with(|p| {
                if let Some(s) = p.take() {
                    a.capture_shortcut.store(0, Ordering::SeqCst);
                    let _ = a.control.send(Control::SetShortcut {
                        cancel: capture == 2,
                        shortcut: s,
                    });
                }
            })
        }
        return 1;
    }
    let mut consumed = false;
    PENDING.with(|slot| {
        let mut pending = slot.borrow_mut();
        if let Some(p) = pending.as_mut() {
            if store::normalize_key(d.vkCode) == store::normalize_key(p.key) {
                if up {
                    if !p.used {
                        let _ = a.control.send(if p.cancel {
                            Control::Cancel
                        } else {
                            Control::Toggle
                        });
                        consumed = true;
                    }
                    *pending = None;
                } else if down {
                    consumed = true;
                }
            } else if down && !p.used {
                p.used = true;
                if a.insertion_active.load(Ordering::SeqCst) {
                    a.input_paused.store(true, Ordering::SeqCst)
                }
                let events = [
                    key(p.key as u16, 0, 0),
                    key(d.vkCode as u16, d.scanCode as u16, 0),
                ];
                SendInput(2, events.as_ptr(), std::mem::size_of::<INPUT>() as i32);
                consumed = true;
            }
        }
    });
    if consumed {
        return 1;
    }
    if HELD
        .with(|h| h.get() != 0 && store::normalize_key(h.get()) == store::normalize_key(d.vkCode))
    {
        if up {
            HELD.with(|h| h.set(0))
        }
        return 1;
    }
    if down {
        let cancel = a.recording.load(Ordering::SeqCst)
            && store::shortcut_matches(
                App::decode_shortcut(a.cancel_key.load(Ordering::Relaxed)),
                d.vkCode,
                modifiers(),
            );
        let toggle = store::shortcut_matches(
            App::decode_shortcut(a.record_key.load(Ordering::Relaxed)),
            d.vkCode,
            modifiers(),
        );
        if cancel || toggle {
            let shortcut = App::decode_shortcut(if cancel {
                a.cancel_key.load(Ordering::Relaxed)
            } else {
                a.record_key.load(Ordering::Relaxed)
            });
            if shortcut.modifiers == 0 && store::modifier(d.vkCode) != 0 {
                PENDING.with(|p| {
                    *p.borrow_mut() = Some(Pending {
                        key: d.vkCode,
                        used: false,
                        cancel,
                    })
                });
            } else {
                HELD.with(|p| p.set(d.vkCode));
                let _ = a.control.send(if cancel {
                    Control::Cancel
                } else {
                    Control::Toggle
                });
            }
            return 1;
        }
        if store::modifier(d.vkCode) == 0 && a.insertion_active.load(Ordering::SeqCst) {
            a.input_paused.store(true, Ordering::SeqCst);
        }
    }
    CallNextHookEx(null_mut(), code, wp, lp)
}
unsafe extern "system" fn mouse(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code >= 0
        && matches!(
            wp as u32,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
        )
    {
        if let Some(a) = APP.get() {
            if a.insertion_active.load(Ordering::SeqCst) {
                let p = &*(lp as *const MSLLHOOKSTRUCT);
                let h = GetAncestor(WindowFromPoint(p.pt), GA_ROOT);
                if h as isize != a.overlay_window.load(Ordering::Relaxed) {
                    a.input_paused.store(true, Ordering::SeqCst)
                }
            }
        }
    }
    CallNextHookEx(null_mut(), code, wp, lp)
}
pub fn start_hooks(a: Arc<App>) {
    let _ = APP.set(a.clone());
    thread::spawn(move || unsafe {
        a.hook_thread.store(GetCurrentThreadId(), Ordering::SeqCst);
        let module = crate::platform::main_module();
        let keyboard = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard), module, 0);
        let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse), module, 0);
        if keyboard.is_null() || mouse.is_null() {
            a.fail("Globale Eingabeüberwachung konnte nicht gestartet werden. Diktieren ist zum Schutz des Zielfelds deaktiviert.".into());
            a.hooks_ready.store(false, Ordering::SeqCst);
        } else {
            a.hooks_ready.store(true, Ordering::SeqCst)
        }
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if !keyboard.is_null() {
            UnhookWindowsHookEx(keyboard);
        }
        if !mouse.is_null() {
            UnhookWindowsHookEx(mouse);
        }
    });
}
