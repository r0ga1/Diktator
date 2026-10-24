//! Small, event-driven Windows tray host. The HTML UI lives in a disposable child process.
use crate::{
    app::{App, Control, Phase},
    platform::{self, wide},
    store::Store,
};
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex, OnceLock,
    },
    thread,
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::*,
    System::Threading::*,
    UI::{HiDpi::*, Shell::*, WindowsAndMessaging::*},
};
// Win32 MAKEINTRESOURCEW(1): an integer resource identifier, never dereferenced.
#[allow(clippy::manual_dangling_ptr)]
const APP_ICON: *const u16 = 1usize as *const u16;
const SHOW: u32 = WM_APP + 1;
const QUIT: u32 = WM_APP + 2;
const CLOSED: u32 = WM_APP + 3;
const TRAY: u32 = WM_APP + 4;
const VERSION_QUERY: u32 = WM_APP + 6;
fn version_code() -> usize {
    crate::VERSION
        .split('.')
        .take(3)
        .fold(0, |n, p| n * 1000 + p.parse::<usize>().unwrap_or(0))
}
struct Host {
    a: Arc<App>,
    child: Mutex<Option<Child>>,
    window: isize,
    taskbar: AtomicU32,
    quitting: AtomicBool,
    tray_ready: AtomicBool,
}
static HOST: OnceLock<Host> = OnceLock::new();
pub fn request_quit() {
    if let Some(h) = HOST.get() {
        unsafe {
            PostMessageW(h.window as HWND, QUIT, 0, 0);
        }
    }
}
fn icon_data(h: HWND) -> NOTIFYICONDATAW {
    unsafe {
        let mut n: NOTIFYICONDATAW = std::mem::zeroed();
        n.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        n.hWnd = h;
        n.uID = 1;
        n.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP | NIF_SHOWTIP;
        n.uCallbackMessage = TRAY;
        n.hIcon = LoadIconW(platform::main_module(), APP_ICON);
        let text = wide("Silbenlot · Öffnen oder Rechtsklick");
        n.szTip[..text.len()].copy_from_slice(&text);
        n
    }
}
fn add_icon(h: HWND) {
    unsafe {
        let mut n = icon_data(h);
        let ok = Shell_NotifyIconW(NIM_ADD, &n) != 0;
        if let Some(host) = HOST.get() {
            host.tray_ready.store(ok, Ordering::SeqCst);
        }
        n.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        Shell_NotifyIconW(NIM_SETVERSION, &n);
    }
}
fn runtime_ready(root: &std::path::Path) -> Result<(), String> {
    let runtime = root.join("webview2");
    if !runtime.join("msedgewebview2.exe").is_file() {
        return Err("Die eingebettete Oberflächen-Laufzeit fehlt. Bitte Silbenlot erneut vollständig herunterladen.".into());
    }
    let marker = runtime.join(".diktator-appcontainer-rx");
    if !marker.exists() {
        use std::os::windows::process::CommandExt;
        let system = std::env::var_os("SystemRoot").ok_or("Windows-Systemverzeichnis fehlt")?;
        // Microsoft requires read/execute on the fixed runtime for Windows 10's renderer AppContainer.
        // Grant only these runtime files, never the Silbenlot data or model directories.
        let result = Command::new(PathBuf::from(system).join("System32/icacls.exe"))
            .arg(&runtime)
            .args([
                "/grant",
                "*S-1-15-2-2:(OI)(CI)(RX)",
                "*S-1-15-2-1:(OI)(CI)(RX)",
                "/T",
                "/Q",
            ])
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| format!("Runtime-Leserechte konnten nicht gesetzt werden: {e}"))?;
        if !result.status.success() {
            return Err(format!(
                "Windows konnte die Runtime-Leserechte nicht vorbereiten (Code {:?}).",
                result.status.code()
            ));
        }
        fs::write(marker, b"runtime-only RX").map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn show_ui() {
    let Some(h) = HOST.get() else { return };
    let mut child = h.child.lock().unwrap();
    if let Some(c) = child.as_mut() {
        if c.try_wait().ok().flatten().is_none() {
            let w = h.a.main_window.load(Ordering::Relaxed) as HWND;
            if !w.is_null() {
                unsafe {
                    ShowWindow(w, SW_RESTORE);
                    SetForegroundWindow(w);
                }
            }
            return;
        }
    }
    if let Err(e) = runtime_ready(&h.a.root) {
        platform::error_box(&e);
        return;
    }
    let log_path = h.a.dir.join("ui-start.log");
    let log = match fs::File::create(&log_path) {
        Ok(f) => f,
        Err(e) => {
            platform::error_box(&format!("Startprotokoll konnte nicht angelegt werden: {e}"));
            return;
        }
    };
    let mut command = Command::new(h.a.root.join("ui/Silbenlot-UI.exe"));
    use std::os::windows::process::CommandExt;
    command
        .creation_flags(0x08000000)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(log));
    match command.spawn() {
        Ok(mut c) => {
            let id = c.id();
            let input = c.stdout.take().unwrap();
            let output = c.stdin.take().unwrap();
            *child = Some(c);
            let a = h.a.clone();
            let window = h.window;
            thread::spawn(move || {
                crate::bridge::serve(a, input, output);
                unsafe {
                    PostMessageW(window as HWND, CLOSED, id as usize, 0);
                }
            });
        }
        Err(e) => platform::error_box(&format!("Oberfläche konnte nicht gestartet werden: {e}")),
    }
}
fn begin_quit() {
    let Some(h) = HOST.get() else { return };
    if h.quitting.swap(true, Ordering::SeqCst) {
        return;
    }
    let a = h.a.clone();
    let window = h.window;
    thread::spawn(move || {
        let phase = a.snapshot().phase;
        if phase == Phase::Recording {
            let _ = a.control.send(Control::Toggle);
        } else if phase == Phase::Testing {
            let _ = a.control.send(Control::Cancel);
        }
        while !matches!(a.snapshot().phase, Phase::Idle) {
            thread::sleep(Duration::from_millis(100));
        }
        a.shutdown();
        unsafe {
            PostMessageW(window as HWND, WM_CLOSE, 0, 0);
        }
    });
}
unsafe extern "system" fn wnd(w: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let Some(h) = HOST.get() else {
        return DefWindowProcW(w, m, wp, lp);
    };
    if m == h.taskbar.load(Ordering::Relaxed) {
        add_icon(w);
        return 0;
    }
    match m {
        VERSION_QUERY => version_code() as LRESULT,
        SHOW => {
            show_ui();
            0
        }
        QUIT => {
            begin_quit();
            0
        }
        CLOSED => {
            let mut child = h.child.lock().unwrap();
            if child.as_ref().is_some_and(|c| c.id() == wp as u32) {
                let mut status = child.as_mut().and_then(|c| c.try_wait().ok().flatten());
                for _ in 0..10 {
                    if status.is_some() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                    status = child.as_mut().and_then(|c| c.try_wait().ok().flatten());
                }
                if status.is_none() {
                    if let Some(c) = child.as_mut() {
                        let _ = c.kill();
                        status = c.wait().ok();
                    }
                }
                child.take();
                h.a.main_window.store(0, Ordering::SeqCst);
                h.a.capture_shortcut.store(0, Ordering::SeqCst);
                h.a.release_idle_model();
                if let Some(code) = status.filter(|s| !s.success()) {
                    let details =
                        fs::read_to_string(h.a.dir.join("ui-start.log")).unwrap_or_default();
                    let details = details.chars().take(1800).collect::<String>();
                    platform::error_box(&format!("Die Oberfläche konnte nicht gestartet werden (Code {:?}).\n{details}\nStartprotokoll: {}",code.code(),h.a.dir.join("ui-start.log").display()));
                }
                if !h.tray_ready.load(Ordering::Relaxed) || !h.a.settings().close_to_background {
                    drop(child);
                    begin_quit();
                }
            }
            0
        }
        TRAY => {
            let event = (lp as u32) & 0xffff;
            if event == WM_CONTEXTMENU || event == WM_RBUTTONUP {
                let menu = CreatePopupMenu();
                AppendMenuW(menu, MF_STRING, 1, wide("Silbenlot öffnen").as_ptr());
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
                AppendMenuW(menu, MF_STRING, 2, wide("Programm beenden").as_ptr());
                let mut p: POINT = std::mem::zeroed();
                GetCursorPos(&mut p);
                SetForegroundWindow(w);
                let selected = TrackPopupMenu(
                    menu,
                    TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
                    p.x,
                    p.y,
                    0,
                    w,
                    null(),
                );
                DestroyMenu(menu);
                PostMessageW(w, WM_NULL, 0, 0);
                match selected {
                    1 => show_ui(),
                    2 => begin_quit(),
                    _ => {}
                }
            } else if event == WM_LBUTTONUP || event == NIN_SELECT || event == (NIN_SELECT | 1) {
                show_ui();
            }
            0
        }
        WM_CLOSE => {
            if h.a.quitting.load(Ordering::Relaxed) {
                DestroyWindow(w);
            } else {
                begin_quit();
            }
            0
        }
        WM_DESTROY => {
            Shell_NotifyIconW(NIM_DELETE, &icon_data(w));
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(w, m, wp, lp),
    }
}
fn prepare(dir: &std::path::Path) -> Result<PathBuf, String> {
    if let Some(p) = std::env::var_os("DIKTATOR_DEV_ROOT") {
        return Ok(PathBuf::from(p));
    }
    unsafe {
        let mut boot: HWND = null_mut();
        let mut label: HWND = null_mut();
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        if let Some(installed) = crate::bundle::Bundle::installed(&exe)? {
            return Ok(installed.root);
        }
        let mut percentage = 101;
        let result = crate::bundle::Bundle::open(&exe, &dir.join("cache"), |done, total| {
            if boot.is_null() {
                boot = CreateWindowExW(
                    WS_EX_TOOLWINDOW,
                    wide("STATIC").as_ptr(),
                    wide("Silbenlot wird vorbereitet …").as_ptr(),
                    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    460,
                    145,
                    null_mut(),
                    null_mut(),
                    platform::main_module(),
                    null(),
                );
                label = CreateWindowExW(
                    0,
                    wide("STATIC").as_ptr(),
                    wide("Lokales Modell wird bereitgestellt …").as_ptr(),
                    WS_CHILD | WS_VISIBLE,
                    20,
                    15,
                    420,
                    70,
                    boot,
                    null_mut(),
                    platform::main_module(),
                    null(),
                );
                if boot.is_null() {
                    return false;
                }
            }
            let n = done * 100 / total.max(1);
            if n != percentage {
                percentage = n;
                SetWindowTextW(
                    label,
                    wide(&format!(
                        "Modell und Oberfläche werden lokal vorbereitet · {n} %"
                    ))
                    .as_ptr(),
                );
            }
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            IsWindow(boot) != 0
        });
        if !boot.is_null() {
            DestroyWindow(boot);
        }
        result.map(|b| b.root)
    }
}
/// Installer/uninstaller control: finish active dictation, then wait for the
/// existing tray process instead of terminating it or replacing locked files.
pub fn stop_existing() -> Result<(), String> {
    unsafe {
        let mut w = FindWindowW(wide("SilbenlotMainWindow").as_ptr(), null());
        if w.is_null() {
            w = FindWindowW(wide("DiktatorMainWindow").as_ptr(), null());
        }
        if w.is_null() {
            let mutex = OpenMutexW(PROCESS_SYNCHRONIZE, 0, wide("Local\\Silbenlot").as_ptr());
            if !mutex.is_null() {
                CloseHandle(mutex);
                return Err(
                    "Silbenlot startet noch. Bitte kurz warten und erneut versuchen.".into(),
                );
            }
            return Ok(());
        }
        if GetWindowLongPtrW(w, GWL_STYLE) as u32 & WS_CAPTION != 0 {
            return Err("Bitte die alte Silbenlot-Version vollständig schließen.".into());
        }
        let mut pid = 0;
        GetWindowThreadProcessId(w, &mut pid);
        let process = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if process.is_null() {
            return Err("Laufende Silbenlot-Version konnte nicht angesprochen werden.".into());
        }
        PostMessageW(w, QUIT, 0, 0);
        let done = WaitForSingleObject(process, 30000) == WAIT_OBJECT_0;
        CloseHandle(process);
        if done {
            Ok(())
        } else {
            Err(
                "Silbenlot schließt noch ein Diktat ab. Bitte anschließend erneut versuchen."
                    .into(),
            )
        }
    }
}
pub fn run() -> Result<(), String> {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let mut mutex = CreateMutexW(null(), 0, wide("Local\\Silbenlot").as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let w = FindWindowW(wide("SilbenlotMainWindow").as_ptr(), null());
            if w.is_null() {
                CloseHandle(mutex);
                return Err(
                    "Eine Silbenlot-Instanz startet bereits. Bitte den Erststart kurz abwarten."
                        .into(),
                );
            }
            let mut running_version = 0;
            let _ = SendMessageTimeoutW(
                w,
                VERSION_QUERY,
                0,
                0,
                SMTO_ABORTIFHUNG,
                1000,
                &mut running_version,
            );
            if running_version >= version_code() {
                PostMessageW(w, SHOW, 0, 0);
                CloseHandle(mutex);
                return Ok(());
            }
            if GetWindowLongPtrW(w, GWL_STYLE) as u32 & WS_CAPTION != 0 {
                ShowWindow(w, SW_RESTORE);
                SetForegroundWindow(w);
                CloseHandle(mutex);
                return Err("Bitte die ältere Silbenlot-Version über Programm beenden vollständig schließen und diese neue EXE erneut öffnen.".into());
            }
            // 0.5 has the same hidden tray host/QUIT protocol, but no version query.
            // Finish active dictation gracefully; never forcibly terminate the old process.
            let mut pid = 0;
            GetWindowThreadProcessId(w, &mut pid);
            let process = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
            CloseHandle(mutex);
            if process.is_null() {
                return Err(
                    "Die ältere Silbenlot-Version muss zuerst über ihr Tray-Menü beendet werden."
                        .into(),
                );
            }
            PostMessageW(w, QUIT, 0, 0);
            let completed = WaitForSingleObject(process, 30000) == WAIT_OBJECT_0;
            CloseHandle(process);
            if !completed {
                return Err("Die ältere Version schließt noch ein Diktat ab. Bitte anschließend diese EXE erneut öffnen.".into());
            }
            mutex = CreateMutexW(null(), 0, wide("Local\\Silbenlot").as_ptr());
            if GetLastError() == ERROR_ALREADY_EXISTS {
                let current = FindWindowW(wide("SilbenlotMainWindow").as_ptr(), null());
                if !current.is_null() {
                    PostMessageW(current, SHOW, 0, 0);
                }
                CloseHandle(mutex);
                return Ok(());
            }
        }
        let old = OpenMutexW(
            PROCESS_SYNCHRONIZE,
            0,
            wide("Local\\VoiceInkLocal").as_ptr(),
        );
        if !old.is_null() {
            CloseHandle(old);
            return Err("Die bisherige VoiceInk-App läuft noch. Bitte diese schließen und Silbenlot erneut starten, damit nicht zwei Apps dieselben Kürzel verwenden.".into());
        }
        let dir = platform::data_dir();
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let root = prepare(&dir)?;
        let mut data = Store::load(&dir, Some(&platform::old_dir()))?;
        if !data.settings.tray_setup {
            data.settings.close_to_background = true;
            data.settings.tray_setup = true;
            data.save(&dir)?;
        }
        let autostart_error = if data.settings.autostart {
            platform::set_autostart(true).err()
        } else {
            None
        };
        let a = App::new(root, dir, data);
        if let Some(e) = autostart_error {
            a.fail(format!("Autostart konnte nicht aktualisiert werden: {e}"));
        }
        let class = wide("SilbenlotMainWindow");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd),
            hInstance: platform::main_module(),
            hIcon: LoadIconW(platform::main_module(), APP_ICON),
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&wc);
        let w = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class.as_ptr(),
            wide("Silbenlot").as_ptr(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            platform::main_module(),
            null(),
        );
        if w.is_null() {
            return Err("Infobereich konnte nicht gestartet werden".into());
        }
        a.native_window.store(w as isize, Ordering::Relaxed);
        let _ = HOST.set(Host {
            a: a.clone(),
            child: Mutex::new(None),
            window: w as isize,
            taskbar: AtomicU32::new(RegisterWindowMessageW(wide("TaskbarCreated").as_ptr())),
            quitting: AtomicBool::new(false),
            tray_ready: AtomicBool::new(false),
        });
        add_icon(w);
        platform::start_hooks(a.clone());
        platform::overlay::start(a.clone());
        crate::update::start(a.clone());
        let stdio = std::env::args().any(|s| s == "--stdio");
        if stdio {
            let b = a.clone();
            thread::spawn(move || {
                crate::bridge::serve(b, std::io::stdin(), std::io::stdout());
                request_quit();
            });
        } else if !std::env::args().any(|s| s == "--autostart")
            || !HOST.get().unwrap().tray_ready.load(Ordering::Relaxed)
        {
            show_ui();
        }
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        a.shutdown();
        if let Some(mut child) = HOST.get().unwrap().child.lock().unwrap().take() {
            let ui = a.main_window.load(Ordering::Relaxed) as HWND;
            if !ui.is_null() {
                PostMessageW(ui, WM_CLOSE, 0, 0);
            }
            for _ in 0..20 {
                if child.try_wait().ok().flatten().is_some() {
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
        crate::update::apply_on_exit(&a);
        if !mutex.is_null() {
            CloseHandle(mutex);
        }
        std::process::exit(0)
    }
}
