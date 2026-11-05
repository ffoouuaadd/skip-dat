//! The overlay window: transparent, always on top, click-through, never takes focus, and
//! excluded from screen capture so it can photograph the desktop behind itself for the glass.
//!
//! It is always in one of three phases. OPEN: the trigger is held and it takes the wheel, the
//! arrow keys and clicks on its own buttons. CLOSING: playing its closing animation; it takes
//! nothing but Esc. HIDDEN. Each opening gets a number, so a late "done closing" from an earlier
//! opening can never hide a newer one.
//!
//! Every change of phase happens on one thread (the input forwarder in hooks.rs), and the window
//! is shown and hidden asynchronously, so closing never waits for the app's UI thread.

use std::sync::atomic::{AtomicIsize, AtomicU32, AtomicU8, Ordering::Relaxed};
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits, GetMonitorInfoW,
    MonitorFromPoint, ReleaseDC, SelectObject, SetStretchBltMode, StretchBlt, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, HALFTONE, HGDIOBJ, MONITORINFO, MONITOR_DEFAULTTONEAREST, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, IsWindowVisible, SetWindowDisplayAffinity, SetWindowLongPtrW, SetWindowPos, ShowWindowAsync,
    GWL_EXSTYLE, HWND_TOPMOST, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, WDA_EXCLUDEFROMCAPTURE,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

use crate::AppState;
use crate::util::Lock;

pub const HIDDEN: u8 = 0;
pub const OPEN: u8 = 1;
pub const CLOSING: u8 = 2;

static PHASE: AtomicU8 = AtomicU8::new(HIDDEN);
static GEN: AtomicU32 = AtomicU32::new(0);

pub fn phase() -> u8 {
    PHASE.load(Relaxed)
}

pub fn set_phase(p: u8) {
    PHASE.store(p, Relaxed);
}

/// Desktop pixels behind the overlay, at half size (the glass only ever shows it blurred).
pub struct Snapshot {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

static MONITOR: Mutex<(i32, i32, i32, i32)> = Mutex::new((0, 0, 0, 0));

pub fn origin() -> (i32, i32) {
    let m = MONITOR.guard();
    (m.0, m.1)
}

// ---------- the overlay's own buttons ----------

struct HitRect {
    l: i32,
    t: i32,
    r: i32,
    b: i32,
    id: String,
}

static HITS: Mutex<Vec<HitRect>> = Mutex::new(Vec::new());

#[derive(serde::Deserialize)]
pub struct Rect {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// The page tells us where its buttons are (in screen pixels from the window's corner).
pub fn set_hits(gen: u32, rects: Vec<Rect>) {
    if gen != GEN.load(Relaxed) || phase() != OPEN {
        return;
    }
    let (ox, oy) = origin();
    let list = rects
        .into_iter()
        .take(64)
        .filter(|r| r.w > 0.0 && r.h > 0.0)
        .map(|r| HitRect {
            l: ox + r.x.floor() as i32,
            t: oy + r.y.floor() as i32,
            r: ox + (r.x + r.w).ceil() as i32,
            b: oy + (r.y + r.h).ceil() as i32,
            id: r.id,
        })
        .collect();
    *HITS.guard() = list;
}

/// Which button (if any) is under this screen point. Called from the mouse hook: quick.
pub fn hit(x: i32, y: i32) -> Option<String> {
    let hits = HITS.guard();
    // the list is front to back
    hits.iter().find(|h| x >= h.l && x < h.r && y >= h.t && y < h.b).map(|h| h.id.clone())
}

// ---------- showing and hiding ----------

/// The window handle, read once at start (asking Tauri for it later would wait for the UI thread).
static HWND_RAW: AtomicIsize = AtomicIsize::new(0);

fn hwnd() -> Option<HWND> {
    match HWND_RAW.load(Relaxed) {
        0 => None,
        h => Some(HWND(h as *mut _)),
    }
}

/// Is the window on screen right now (whatever the phase says)?
pub fn visible() -> bool {
    hwnd().map(|h| unsafe { IsWindowVisible(h).as_bool() }).unwrap_or(false)
}

pub fn init(app: &AppHandle) {
    let Some(win) = app.get_webview_window("overlay") else { return };
    let _ = win.set_ignore_cursor_events(true);
    if let Ok(h) = win.hwnd() {
        HWND_RAW.store(h.0 as isize, Relaxed);
        unsafe {
            let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
            SetWindowLongPtrW(h, GWL_EXSTYLE, ex | (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0 | WS_EX_TOPMOST.0) as isize);
            let _ = SetWindowDisplayAffinity(h, WDA_EXCLUDEFROMCAPTURE);
        }
    }
}

#[derive(Clone, serde::Serialize)]
struct ShowPayload {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    gen: u32,
    layout: String,
}

/// Cover the monitor under the cursor and appear there without stealing focus.
pub fn summon(app: &AppHandle, x: i32, y: i32) {
    let Some(h) = hwnd() else { return };
    let r = unsafe {
        let mon = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(mon, &mut mi);
        mi.rcMonitor
    };
    let (w, hgt) = (r.right - r.left, r.bottom - r.top);
    *MONITOR.guard() = (r.left, r.top, w, hgt);
    let gen = GEN.fetch_add(1, Relaxed) + 1;
    HITS.guard().clear();
    set_phase(OPEN);

    // photograph the desktop behind the overlay (our own window is excluded from capture);
    // performance mode and solid skins don't show it, so they skip this
    let wants_desktop = app.try_state::<AppState>().map(|st| crate::needs_desktop(&st)).unwrap_or(true);
    let app2 = app.clone();
    if wants_desktop { std::thread::spawn(move || {
        if let Some(snap) = capture(r) {
            // a capture from an earlier opening that finished late must not replace this one
            if gen != GEN.load(Relaxed) {
                return;
            }
            if let Some(state) = app2.try_state::<AppState>() {
                *state.snapshot.guard() = Some(snap);
            }
            let _ = app2.emit_to("overlay", "lg-snapshot", ());
        }
    }); }

    unsafe {
        let _ = SetWindowPos(h, Some(HWND_TOPMOST), r.left, r.top, w, hgt, SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_ASYNCWINDOWPOS);
    }
    let layout = app.try_state::<AppState>().map(|s| s.cfg.guard().layout.clone()).unwrap_or_default();
    let _ = app.emit_to("overlay", "lg-show", ShowPayload { x: x - r.left, y: y - r.top, w, h: hgt, gen, layout });
    crate::view::on_summon(app);
}

fn hide() {
    set_phase(HIDDEN);
    HITS.guard().clear();
    if let Some(h) = hwnd() {
        unsafe {
            let _ = ShowWindowAsync(h, SW_HIDE);
        }
    }
}

/// The page finished its closing animation for opening `gen` (called on the forwarder thread).
pub fn done(gen: u32) {
    if gen == GEN.load(Relaxed) && phase() == CLOSING {
        hide();
    }
}

/// Hide now, whatever the page is doing, and tell it to reset.
pub fn force_hide(app: &AppHandle) {
    hide();
    let _ = app.emit_to("overlay", "lg-force-hide", GEN.load(Relaxed));
}

fn capture(r: RECT) -> Option<Snapshot> {
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    let (dw, dh) = ((w / 2).max(1), (h / 2).max(1));
    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return None;
        }
        let mem = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, dw, dh);
        let old = SelectObject(mem, HGDIOBJ(bmp.0));
        SetStretchBltMode(mem, HALFTONE);
        let ok = StretchBlt(mem, 0, 0, dw, dh, Some(screen), r.left, r.top, w, h, SRCCOPY).as_bool();
        let mut bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: dw,
                biHeight: -dh, // top-down rows
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut buf = vec![0u8; (dw * dh * 4) as usize];
        // the bitmap must not be selected into a DC while its bits are read
        SelectObject(mem, old);
        let lines = GetDIBits(mem, bmp, 0, dh as u32, Some(buf.as_mut_ptr() as *mut _), &mut bi, DIB_RGB_COLORS);
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        if !ok || lines == 0 {
            return None;
        }
        for px in buf.chunks_exact_mut(4) {
            px.swap(0, 2); // BGRA -> RGBA
            px[3] = 255;
        }
        Some(Snapshot { w: dw as u32, h: dh as u32, rgba: buf })
    }
}
