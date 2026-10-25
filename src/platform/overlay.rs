//! Per-pixel alpha overlay: native-resolution antialiasing, no window region/border.
use crate::{
    app::{App, Control, Phase},
    platform::{main_module, wide},
};
use std::{
    cell::RefCell,
    ptr::{null, null_mut},
    sync::{atomic::Ordering, Arc, OnceLock},
    thread,
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    UI::{HiDpi::*, WindowsAndMessaging::*},
};
static APP: OnceLock<Arc<App>> = OnceLock::new();
const REFRESH: u32 = WM_APP + 51;
thread_local! {static SURFACE:RefCell<Option<Surface>>=const{RefCell::new(None)};}
struct Surface {
    dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    pixels: *mut u32,
    w: i32,
    h: i32,
    bars: [f32; 24],
}
impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}
impl Surface {
    unsafe fn new(w: i32, h: i32) -> Self {
        let mut b: BITMAPINFO = std::mem::zeroed();
        b.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        b.bmiHeader.biWidth = w;
        b.bmiHeader.biHeight = -h;
        b.bmiHeader.biPlanes = 1;
        b.bmiHeader.biBitCount = 32;
        b.bmiHeader.biCompression = BI_RGB;
        let dc = CreateCompatibleDC(null_mut());
        let mut bits = null_mut();
        let bitmap = CreateDIBSection(dc, &b, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
        let old = SelectObject(dc, bitmap);
        Self {
            dc,
            bitmap,
            old,
            pixels: bits.cast(),
            w,
            h,
            bars: [0.; 24],
        }
    }
    unsafe fn round(&mut self, rect: [f32; 4], radius: f32, color: [u8; 3], alpha: f32) {
        let [x, y, w, h] = rect;
        let cx = x + w / 2.;
        let cy = y + h / 2.;
        for py in (y.floor() as i32 - 1).max(0)..((y + h).ceil() as i32 + 1).min(self.h) {
            for px in (x.floor() as i32 - 1).max(0)..((x + w).ceil() as i32 + 1).min(self.w) {
                let dx = ((px as f32 + 0.5 - cx).abs() - w / 2. + radius).max(0.);
                let dy = ((py as f32 + 0.5 - cy).abs() - h / 2. + radius).max(0.);
                let inner = ((px as f32 + 0.5 - cx).abs() - w / 2. + radius)
                    .max((py as f32 + 0.5 - cy).abs() - h / 2. + radius)
                    .min(0.);
                let d = (dx * dx + dy * dy).sqrt() + inner - radius;
                let a = ((0.5 - d).clamp(0., 1.) * alpha * 255.).round() as u32;
                if a == 0 {
                    continue;
                }
                let p = self.pixels.add((py * self.w + px) as usize);
                let old = *p;
                let inv = 255 - a;
                let ch = |shift: u32, v: u8| {
                    ((v as u32 * a + ((old >> shift) & 255) * inv + 127) / 255).min(255)
                };
                let oa = (a + ((old >> 24) * inv + 127) / 255).min(255);
                *p = (oa << 24)
                    | (ch(16, color[0]) << 16)
                    | (ch(8, color[1]) << 8)
                    | ch(0, color[2]);
            }
        }
    }
    unsafe fn frame(&mut self, scale: f32, levels: [f32; 24], paused: bool) {
        if self.pixels.is_null() {
            return;
        }
        std::slice::from_raw_parts_mut(self.pixels, (self.w * self.h) as usize).fill(0);
        let s = scale;
        self.round(
            [0., 0., self.w as f32, self.h as f32],
            30. * s,
            [18, 18, 20],
            0.96,
        );
        self.round(
            [160. * s, 12. * s, 36. * s, 36. * s],
            18. * s,
            [235, 96, 105],
            1.,
        );
        self.round([172. * s, 24. * s, 12. * s, 12. * s], 2. * s, [255; 3], 1.);
        for (i, level) in levels.iter().enumerate() {
            self.bars[i] += (level - self.bars[i]) * 0.34;
            let height = (3. + self.bars[i].sqrt() * 25.).clamp(3., 29.) * s;
            self.round(
                [
                    (32. + i as f32 * 4.2) * s,
                    (60. * s - height) / 2.,
                    2.25 * s,
                    height,
                ],
                1.125 * s,
                if paused {
                    [232, 185, 104]
                } else {
                    [240, 240, 244]
                },
                1.,
            );
        }
    }
}
pub fn refresh(a: &App) {
    let w = a.overlay_window.load(Ordering::Relaxed);
    if w != 0 {
        unsafe {
            PostMessageW(w as HWND, REFRESH, 0, 0);
        }
    }
}
unsafe fn update(h: HWND) {
    let Some(a) = APP.get() else { return };
    if a.quitting.load(Ordering::Relaxed) {
        DestroyWindow(h);
        return;
    }
    let state = a.snapshot();
    let on = matches!(state.phase, Phase::Recording | Phase::Testing) && a.settings().show_overlay;
    if !on {
        KillTimer(h, 1);
        ShowWindow(h, SW_HIDE);
        SURFACE.with(|s| {
            s.borrow_mut().take();
        });
        return;
    }
    if IsWindowVisible(h) == 0 {
        let target = if state.anchor.window != 0 {
            state.anchor.window
        } else {
            a.main_window.load(Ordering::Relaxed)
        } as HWND;
        let monitor = MonitorFromWindow(target, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        GetMonitorInfoW(monitor, &mut info);
        let dpi = GetDpiForWindow(target).max(96);
        let width = (204 * dpi / 96) as i32;
        let height = (60 * dpi / 96) as i32;
        SetWindowPos(
            h,
            HWND_TOPMOST,
            info.rcWork.left + (info.rcWork.right - info.rcWork.left - width) / 2,
            info.rcWork.bottom - height - (22 * dpi / 96) as i32,
            width,
            height,
            SWP_NOACTIVATE,
        );
        ShowWindow(h, SW_SHOWNOACTIVATE);
        SetTimer(h, 1, 33, None);
    }
    let mut r: RECT = std::mem::zeroed();
    GetWindowRect(h, &mut r);
    let w = r.right - r.left;
    let height = r.bottom - r.top;
    let levels = a.audio.lock().unwrap().bars;
    SURFACE.with(|s| {
        let mut surface = s.borrow_mut();
        if surface.as_ref().is_none_or(|s| s.w != w || s.h != height) {
            *surface = Some(Surface::new(w, height));
        }
        let b = surface.as_mut().unwrap();
        b.frame(
            w as f32 / 204.,
            levels,
            a.input_paused.load(Ordering::Relaxed),
        );
        let dst = POINT {
            x: r.left,
            y: r.top,
        };
        let size = SIZE { cx: w, cy: height };
        let src = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        UpdateLayeredWindow(h, null_mut(), &dst, &size, b.dc, &src, 0, &blend, ULW_ALPHA);
    });
}
unsafe extern "system" fn wnd(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match m {
        WM_MOUSEACTIVATE => MA_NOACTIVATE as isize,
        REFRESH | WM_TIMER => {
            update(h);
            0
        }
        WM_LBUTTONUP => {
            let width = {
                let mut r: RECT = std::mem::zeroed();
                GetClientRect(h, &mut r);
                r.right.max(1)
            };
            let x = l as u16 as i32 * 204 / width;
            if let Some(a) = APP.get() {
                if x > 155 {
                    let _ = a.control.send(Control::Toggle);
                }
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let mut p: PAINTSTRUCT = std::mem::zeroed();
            BeginPaint(h, &mut p);
            EndPaint(h, &p);
            0
        }
        WM_CLOSE => {
            DestroyWindow(h);
            0
        }
        WM_DESTROY => {
            KillTimer(h, 1);
            SURFACE.with(|s| {
                s.borrow_mut().take();
            });
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(h, m, w, l),
    }
}
pub fn start(a: Arc<App>) {
    let _ = APP.set(a.clone());
    thread::spawn(move || unsafe {
        let class = wide("DiktatorMiniPegel");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd),
            hInstance: main_module(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&wc);
        let h = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class.as_ptr(),
            wide("Diktator Pegel").as_ptr(),
            WS_POPUP,
            0,
            0,
            204,
            60,
            null_mut(),
            null_mut(),
            main_module(),
            null(),
        );
        if h.is_null() {
            a.fail("Pegelanzeige konnte nicht gestartet werden".into());
            return;
        }
        a.overlay_window.store(h as isize, Ordering::Relaxed);
        PostMessageW(h, REFRESH, 0, 0);
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        a.overlay_window.store(0, Ordering::Relaxed);
    });
}
