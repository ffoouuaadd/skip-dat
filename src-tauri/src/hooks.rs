//! Global input: hold the trigger (Alt by default, configurable) and scroll anywhere in Windows.
//!
//! Low-level hooks run on their own thread with a message loop and do almost nothing inside the
//! callback (Windows silently removes hooks that take too long). Events go to a forwarder thread.
//!
//! Accuracy: the wheel reports multiples of WHEEL_DELTA (120) per notch, and high-resolution
//! wheels report smaller pieces. We add the raw deltas up and emit exactly one step per 120,
//! so one notch is always one song, whatever the mouse, driver or display scaling.
//!
//! Never stuck: whether a keyboard trigger is held is read from the real key state, not from
//! remembered key events, so a key release Windows never told us about (it happens when an
//! administrator window has focus) cannot leave Skip Dat thinking the key is still down. A
//! watchdog closes the overlay if the trigger is no longer held, if a mouse-button trigger sees no
//! input for a while, or if closing takes too long. Esc and clicking anywhere outside the overlay
//! always close it.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicU8, Ordering::Relaxed};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP,
    MOUSEINPUT, MOUSE_EVENT_FLAGS, VIRTUAL_KEY, VK_DOWN, VK_ESCAPE, VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN,
    VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SPACE, VK_TAB, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, GetSystemMetrics, SetTimer, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, LLKHF_INJECTED, LLMHF_INJECTED, MSG, MSLLHOOKSTRUCT,
    SM_CXDRAG, SM_SWAPBUTTON, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN,
    WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN,
    WM_SYSKEYUP, WM_TIMER, WM_XBUTTONDOWN, WM_XBUTTONUP,
};

use crate::overlay;

const WHEEL_DELTA: i32 = 120;

// ---------- the trigger ----------

const ALT: u8 = 1;
const CTRL: u8 = 2;
const SHIFT: u8 = 4;
const WIN: u8 = 8;
const M_RIGHT: u8 = 1;
const M_BACK: u8 = 2;
const M_FORWARD: u8 = 3;

/// (keys, mouse button): exactly one of the two is non-zero
pub fn parse_trigger(s: &str) -> Option<(u8, u8)> {
    Some(match s {
        "alt" => (ALT, 0),
        "ctrl" => (CTRL, 0),
        "shift" => (SHIFT, 0),
        "win" => (WIN, 0),
        "ctrl+alt" => (CTRL | ALT, 0),
        "ctrl+shift" => (CTRL | SHIFT, 0),
        "alt+shift" => (ALT | SHIFT, 0),
        "mouse-right" => (0, M_RIGHT),
        "mouse-back" => (0, M_BACK),
        "mouse-forward" => (0, M_FORWARD),
        _ => return None,
    })
}

/// How the trigger is written for people ("Alt", "Ctrl + Alt", "right mouse button").
pub fn trigger_label(s: &str) -> &'static str {
    match s {
        "ctrl" => "Ctrl",
        "shift" => "Shift",
        "win" => "Windows key",
        "ctrl+alt" => "Ctrl + Alt",
        "ctrl+shift" => "Ctrl + Shift",
        "alt+shift" => "Alt + Shift",
        "mouse-right" => "the right mouse button",
        "mouse-back" => "the mouse Back button",
        "mouse-forward" => "the mouse Forward button",
        _ => "Alt",
    }
}

static TRIG_KEYS: AtomicU8 = AtomicU8::new(ALT);
static TRIG_MOUSE: AtomicU8 = AtomicU8::new(0);

pub fn set_trigger(s: &str) {
    let (k, m) = parse_trigger(s).unwrap_or((ALT, 0));
    TRIG_KEYS.store(k, Relaxed);
    TRIG_MOUSE.store(m, Relaxed);
    // how far the held button may move before it counts as a drag (scaled with the display)
    let drag = unsafe { GetSystemMetrics(SM_CXDRAG) };
    DRAG_PX.store((drag * 3).max(10), Relaxed);
    // never carry a half-finished gesture over to the new trigger
    MB.store(MB_IDLE, Relaxed);
}

/// The shortcut needs the "harmless key" trick on release: Alt (menu bar), Win (Start menu), and
/// any two-key shortcut (Ctrl + Shift switches the keyboard layout, for example).
fn needs_mask(trig: u8) -> bool {
    trig & (ALT | WIN) != 0 || trig.count_ones() >= 2
}

fn key_down(vk: VIRTUAL_KEY) -> bool {
    unsafe { (GetAsyncKeyState(vk.0 as i32) as u16) & 0x8000 != 0 }
}

fn mods_now() -> u8 {
    let mut m = 0;
    if key_down(VK_LMENU) || key_down(VK_RMENU) {
        m |= ALT;
    }
    if key_down(VK_LCONTROL) || key_down(VK_RCONTROL) {
        m |= CTRL;
    }
    if key_down(VK_LSHIFT) || key_down(VK_RSHIFT) {
        m |= SHIFT;
    }
    if key_down(VK_LWIN) || key_down(VK_RWIN) {
        m |= WIN;
    }
    m
}

/// The keyboard trigger is held: its keys are down and no other of Alt / Ctrl / Win is (Shift is
/// free unless it is part of the trigger, so Shift + Tab still works).
fn keys_held(trig: u8) -> bool {
    let care = ALT | CTRL | WIN | (trig & SHIFT);
    mods_now() & care == trig
}

fn group_of(vk: u32) -> u8 {
    match VIRTUAL_KEY(vk as u16) {
        VK_LMENU | VK_RMENU => ALT,
        VK_LCONTROL | VK_RCONTROL => CTRL,
        VK_LSHIFT | VK_RSHIFT => SHIFT,
        VK_LWIN | VK_RWIN => WIN,
        _ => 0,
    }
}

fn other_side(vk: u32) -> VIRTUAL_KEY {
    match VIRTUAL_KEY(vk as u16) {
        VK_LMENU => VK_RMENU,
        VK_RMENU => VK_LMENU,
        VK_LCONTROL => VK_RCONTROL,
        VK_RCONTROL => VK_LCONTROL,
        VK_LSHIFT => VK_RSHIFT,
        VK_RSHIFT => VK_LSHIFT,
        VK_LWIN => VK_RWIN,
        _ => VK_LWIN,
    }
}

// ---------- state shared by the hooks and the forwarder ----------

enum Replay {
    Down,
    Up,
    Click,
}

enum Ev {
    Wheel(i32, i32, i32),
    /// switch source: sideways wheel tilt (delta, x, y)
    AppWheel(i32, i32, i32),
    /// switch source: Tab, arrows or a mouse side button while the overlay is open
    AppStep(i32),
    /// arrow keys while the trigger is held (the overlay maps them to its layout)
    Arrow(&'static str),
    Move(i32, i32),
    Release,
    Cancel,
    Toggle,
    Click(String),
    Replay(Replay),
    /// the page finished closing opening number n
    Done(u32),
}

static TX: OnceLock<Sender<Ev>> = OnceLock::new();
/// The trigger is held and the wheel has been used: the overlay belongs to this gesture.
static SESSION: AtomicBool = AtomicBool::new(false);
// mouse-button trigger: what happened since the button went down
const MB_IDLE: u8 = 0;
const MB_PENDING: u8 = 1; // down (held back from apps), no wheel yet
const MB_ACTIVE: u8 = 2; // the wheel was used: it is our gesture
const MB_PASS: u8 = 3; // dragged instead: given back to the app
const MB_DONE: u8 = 4; // the gesture ended (Esc, click away) while the button is still down: its release is ours
static MB: AtomicU8 = AtomicU8::new(MB_IDLE);
static MB_X: AtomicI32 = AtomicI32::new(0);
static MB_Y: AtomicI32 = AtomicI32::new(0);
/// when MB last changed (ms since start), to drop a state whose release never came
static MB_AT: AtomicU64 = AtomicU64::new(0);
static DRAG_PX: AtomicI32 = AtomicI32::new(10);
/// a held-back press or release older than this is assumed lost (the button is long up)
const MB_STALE_MS: u64 = 6000;

fn now_ms() -> u64 {
    static T0: OnceLock<Instant> = OnceLock::new();
    T0.get_or_init(Instant::now).elapsed().as_millis() as u64
}

fn set_mb(v: u8) {
    MB.store(v, Relaxed);
    MB_AT.store(now_ms(), Relaxed);
}
// clicks we took for the overlay: their button-up is ours too
static SWALLOW_L: AtomicBool = AtomicBool::new(false);
static SWALLOW_M: AtomicBool = AtomicBool::new(false);
static SWALLOW_X: AtomicBool = AtomicBool::new(false);

fn send(ev: Ev) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(ev);
    }
}

fn end_session() {
    SESSION.store(false, Relaxed);
    // the button is still down: its coming release must not reach the app on its own
    // (a lone right-button release opens a context menu, a side button goes Back)
    if MB.load(Relaxed) == MB_ACTIVE {
        set_mb(MB_DONE);
    }
}

// ---------- hooks ----------

unsafe extern "system" fn mouse_proc(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let ms = &*(l.0 as *const MSLLHOOKSTRUCT);
        if ms.flags & LLMHF_INJECTED == 0 && on_mouse(w.0 as u32, ms) {
            return LRESULT(1); // the app under the cursor never sees it
        }
    }
    CallNextHookEx(None, code, w, l)
}

/// true: swallow the event
fn on_mouse(msg: u32, ms: &MSLLHOOKSTRUCT) -> bool {
    let (x, y) = (ms.pt.x, ms.pt.y);
    let hi = (ms.mouseData >> 16) & 0xFFFF;
    let tm = TRIG_MOUSE.load(Relaxed);
    let phase = overlay::phase();

    // the trigger itself, when it is a mouse button
    if tm != 0 {
        let (d, u) = match tm {
            M_RIGHT => (WM_RBUTTONDOWN, WM_RBUTTONUP),
            M_BACK if hi == 1 => (WM_XBUTTONDOWN, WM_XBUTTONUP),
            M_FORWARD if hi == 2 => (WM_XBUTTONDOWN, WM_XBUTTONUP),
            _ => (u32::MAX, u32::MAX),
        };
        // a held-back press or finished gesture whose release never arrived: forget it
        let st = MB.load(Relaxed);
        if matches!(st, MB_PENDING | MB_DONE) && now_ms().saturating_sub(MB_AT.load(Relaxed)) > MB_STALE_MS {
            set_mb(MB_IDLE);
        }
        if msg == d {
            // a new press: whatever was before is over (a press during a gesture means its
            // release was lost, so that gesture closes without playing anything)
            if SESSION.load(Relaxed) {
                SESSION.store(false, Relaxed);
                send(Ev::Cancel);
            }
            set_mb(MB_PENDING);
            MB_X.store(x, Relaxed);
            MB_Y.store(y, Relaxed);
            return true;
        }
        if msg == u {
            let st = MB.load(Relaxed);
            set_mb(MB_IDLE);
            return match st {
                // a plain click: give it back to the app
                MB_PENDING => {
                    send(Ev::Replay(Replay::Click));
                    true
                }
                MB_ACTIVE => {
                    SESSION.store(false, Relaxed);
                    send(Ev::Release);
                    true
                }
                // a drag we handed back: its release goes out after the handed-back press
                // (the forwarder sends both, in order)
                MB_PASS => {
                    send(Ev::Replay(Replay::Up));
                    true
                }
                MB_DONE => true,
                _ => false,
            };
        }
        let lim = DRAG_PX.load(Relaxed);
        if msg == WM_MOUSEMOVE && MB.load(Relaxed) == MB_PENDING && ((x - MB_X.load(Relaxed)).abs() > lim || (y - MB_Y.load(Relaxed)).abs() > lim) {
            // a drag, not our gesture: hand the button back
            set_mb(MB_PASS);
            send(Ev::Replay(Replay::Down));
        }
    }

    let held = if tm != 0 { matches!(MB.load(Relaxed), MB_PENDING | MB_ACTIVE | MB_DONE) } else { keys_held(TRIG_KEYS.load(Relaxed)) };
    let session = SESSION.load(Relaxed);
    match msg {
        WM_MOUSEWHEEL | WM_MOUSEHWHEEL if held => {
            if tm != 0 {
                set_mb(MB_ACTIVE);
            }
            SESSION.store(true, Relaxed);
            let delta = hi as u16 as i16 as i32;
            send(if msg == WM_MOUSEWHEEL { Ev::Wheel(delta, x, y) } else { Ev::AppWheel(delta, x, y) });
            true
        }
        WM_MOUSEMOVE => {
            if phase != overlay::HIDDEN {
                send(Ev::Move(x, y));
            }
            false
        }
        // a new press means an earlier taken click's release was lost: don't eat this one's
        WM_LBUTTONDOWN if phase != overlay::OPEN => {
            SWALLOW_L.store(false, Relaxed);
            false
        }
        WM_LBUTTONDOWN => match overlay::hit(x, y) {
            Some(id) => {
                SWALLOW_L.store(true, Relaxed);
                send(Ev::Click(id));
                true
            }
            None => {
                // clicking anywhere else closes the overlay; the click still goes where it was aimed
                SWALLOW_L.store(false, Relaxed);
                end_session();
                send(Ev::Cancel);
                false
            }
        },
        WM_LBUTTONUP if SWALLOW_L.swap(false, Relaxed) => true,
        WM_RBUTTONDOWN if phase == overlay::OPEN => {
            end_session();
            send(Ev::Cancel);
            false
        }
        WM_MBUTTONDOWN => {
            SWALLOW_M.store(session, Relaxed);
            if session {
                send(Ev::Toggle);
            }
            session
        }
        WM_MBUTTONUP if SWALLOW_M.swap(false, Relaxed) => true,
        // side buttons: Back = previous source, Forward = next source
        WM_XBUTTONDOWN => {
            SWALLOW_X.store(session, Relaxed);
            if session {
                send(Ev::AppStep(if hi == 1 { -1 } else { 1 }));
            }
            session
        }
        WM_XBUTTONUP if SWALLOW_X.swap(false, Relaxed) => true,
        _ => false,
    }
}

unsafe extern "system" fn kbd_proc(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let k = &*(l.0 as *const KBDLLHOOKSTRUCT);
        if k.flags.0 & LLKHF_INJECTED.0 == 0 && on_key(w.0 as u32, k.vkCode) {
            return LRESULT(1);
        }
    }
    CallNextHookEx(None, code, w, l)
}

fn on_key(msg: u32, vk: u32) -> bool {
    let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
    if !down && !up {
        return false;
    }
    let session = SESSION.load(Relaxed);
    let trig = TRIG_KEYS.load(Relaxed);
    let g = group_of(vk);
    if g != 0 {
        // releasing the trigger plays the chosen song (the other side's key may still hold it)
        if up && session && TRIG_MOUSE.load(Relaxed) == 0 && trig & g != 0 && !key_down(other_side(vk)) {
            SESSION.store(false, Relaxed);
            send(Ev::Release);
        }
        return false;
    }
    let vk = VIRTUAL_KEY(vk as u16);
    if vk == VK_ESCAPE && overlay::phase() != overlay::HIDDEN {
        if down {
            end_session();
            send(Ev::Cancel);
        }
        return true;
    }
    if !session {
        return false;
    }
    // keys that steer the overlay while the trigger is held (and never reach the app below)
    let ev = match vk {
        // Shift + Tab goes back, unless Shift is part of the shortcut itself
        VK_TAB => Some(Ev::AppStep(if trig & SHIFT == 0 && (key_down(VK_LSHIFT) || key_down(VK_RSHIFT)) { -1 } else { 1 })),
        VK_LEFT => Some(Ev::Arrow("left")),
        VK_RIGHT => Some(Ev::Arrow("right")),
        VK_UP => Some(Ev::Arrow("up")),
        VK_DOWN => Some(Ev::Arrow("down")),
        VK_SPACE => Some(Ev::Toggle),
        VK_RETURN => {
            if down {
                end_session();
                send(Ev::Release);
            }
            return true;
        }
        _ => None,
    };
    match ev {
        Some(e) => {
            if down {
                send(e);
            }
            true
        }
        None => false,
    }
}

// ---------- synthetic input ----------

/// Releasing Alt or Win on its own opens the app's menu bar or the Start menu. A harmless
/// unassigned key pressed while it is down tells Windows it was part of a combination.
fn inject_menu_mask() {
    let key = |flags: KEYBD_EVENT_FLAGS| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(0xE8), wScan: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
    };
    let inputs = [key(KEYBD_EVENT_FLAGS(0)), key(KEYEVENTF_KEYUP)];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Give a held-back trigger button back to the app (a plain click or the start of a drag).
fn inject_trigger(r: Replay) {
    let swapped = unsafe { GetSystemMetrics(SM_SWAPBUTTON) } != 0;
    let (d, u, data): (MOUSE_EVENT_FLAGS, MOUSE_EVENT_FLAGS, i32) = match TRIG_MOUSE.load(Relaxed) {
        // SendInput speaks physical buttons; with swapped buttons the logical right one is the left
        M_RIGHT if swapped => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, 0),
        M_RIGHT => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, 0),
        M_BACK => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, 1),
        M_FORWARD => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, 2),
        _ => return,
    };
    let ev = |f: MOUSE_EVENT_FLAGS| INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 { mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: data as _, dwFlags: f, time: 0, dwExtraInfo: 0 } },
    };
    let inputs: Vec<INPUT> = match r {
        Replay::Down => vec![ev(d)],
        Replay::Up => vec![ev(u)],
        Replay::Click => vec![ev(d), ev(u)],
    };
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// The page finished its closing animation (goes through the forwarder, which owns all phase changes).
pub fn overlay_done(gen: u32) {
    send(Ev::Done(gen));
}

// ---------- threads ----------

pub fn start(app: AppHandle) {
    let (tx, rx) = channel();
    let _ = TX.set(tx);
    std::thread::spawn(|| unsafe {
        let hinst = GetModuleHandleW(None).ok().map(|m| HINSTANCE(m.0));
        let install = || -> (Option<HHOOK>, Option<HHOOK>) {
            (SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), hinst, 0).ok(), SetWindowsHookExW(WH_KEYBOARD_LL, Some(kbd_proc), hinst, 0).ok())
        };
        let mut hooks = install();
        // Windows quietly drops a low-level hook that once took too long (a frozen PC, a debugger).
        // Re-installing now and then, between gestures, makes sure Skip Dat keeps listening.
        SetTimer(None, 0, 5 * 60 * 1000, None);
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            if msg.message == WM_TIMER && !SESSION.load(Relaxed) && overlay::phase() == overlay::HIDDEN {
                let fresh = install();
                if let (Some(m), Some(k)) = fresh {
                    for h in [hooks.0, hooks.1].into_iter().flatten() {
                        let _ = UnhookWindowsHookEx(h);
                    }
                    hooks = (Some(m), Some(k));
                    SWALLOW_L.store(false, Relaxed);
                    SWALLOW_M.store(false, Relaxed);
                    SWALLOW_X.store(false, Relaxed);
                } else {
                    // only half of it worked: keep the old pair rather than listening twice
                    for h in [fresh.0, fresh.1].into_iter().flatten() {
                        let _ = UnhookWindowsHookEx(h);
                    }
                }
                continue;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
    std::thread::spawn(move || forwarder(app, rx));
}

#[derive(Clone, serde::Serialize)]
struct Pt {
    x: i32,
    y: i32,
}

struct Fw {
    app: AppHandle,
    acc: i32,
    acc_h: i32,
    masked: bool,
    closing_since: Option<Instant>,
    /// closing after a choice (the "now playing" pill shows for a moment) or after a cancel
    closing_long: bool,
    /// the last wheel, click or key (not mouse movement) of the gesture
    last_gesture: Instant,
}

impl Fw {
    fn release(&mut self) {
        self.masked = false;
        self.acc = 0;
        self.acc_h = 0;
        if overlay::phase() == overlay::OPEN {
            overlay::set_phase(overlay::CLOSING);
            self.closing_since = Some(Instant::now());
            self.closing_long = true;
            let _ = self.app.emit_to("overlay", "lg-commit", ());
        }
    }

    fn cancel(&mut self) {
        self.masked = false;
        self.acc = 0;
        self.acc_h = 0;
        match overlay::phase() {
            overlay::OPEN => {
                overlay::set_phase(overlay::CLOSING);
                self.closing_since = Some(Instant::now());
                self.closing_long = false;
                let _ = self.app.emit_to("overlay", "lg-cancel", ());
            }
            // Esc while it is already closing: the emergency exit, gone at once
            overlay::CLOSING => {
                overlay::force_hide(&self.app);
                self.closing_since = None;
            }
            _ => {}
        }
    }

    fn open_if_needed(&mut self, x: i32, y: i32) {
        if !self.masked && TRIG_MOUSE.load(Relaxed) == 0 && needs_mask(TRIG_KEYS.load(Relaxed)) {
            inject_menu_mask();
            self.masked = true;
        }
        if overlay::phase() != overlay::OPEN {
            self.acc = 0;
            self.acc_h = 0;
            self.closing_since = None;
            overlay::summon(&self.app, x, y);
        }
    }
}

/// Whole notches from accumulated wheel deltas.
fn notches(acc: &mut i32, delta: i32) -> i32 {
    *acc += delta;
    let mut steps = 0;
    while *acc >= WHEEL_DELTA {
        *acc -= WHEEL_DELTA;
        steps += 1;
    }
    while *acc <= -WHEEL_DELTA {
        *acc += WHEEL_DELTA;
        steps -= 1;
    }
    steps
}

fn forwarder(app: AppHandle, rx: Receiver<Ev>) {
    let mut f = Fw { app: app.clone(), acc: 0, acc_h: 0, masked: false, closing_since: None, closing_long: false, last_gesture: Instant::now() };
    let mut pending: Option<(i32, i32)> = None;
    let mut last_move = Instant::now();
    let mut last_check = Instant::now();
    let mut not_held = 0;
    let mut orphan = 0;
    let mut stray = 0;
    loop {
        // wake often only while something is going on; idle, a sleeping thread costs nothing
        let busy = pending.is_some() || SESSION.load(Relaxed) || overlay::phase() != overlay::HIDDEN || overlay::visible();
        let wait = if pending.is_some() { 6 } else if busy { 50 } else { 500 };
        let ev = match rx.recv_timeout(Duration::from_millis(wait)) {
            Ok(e) => Some(e),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        if ev.is_some() && !matches!(ev, Some(Ev::Move(..)) | Some(Ev::Done(_)) | Some(Ev::Replay(_))) {
            f.last_gesture = Instant::now();
        }
        match ev {
            Some(Ev::Wheel(delta, x, y)) => {
                f.open_if_needed(x, y);
                // wheel rolled toward you (negative delta) moves down the queue, to the next song
                let steps = -notches(&mut f.acc, delta);
                if steps != 0 {
                    let _ = app.emit_to("overlay", "lg-wheel", steps);
                }
            }
            Some(Ev::AppWheel(delta, x, y)) => {
                f.open_if_needed(x, y);
                let steps = notches(&mut f.acc_h, delta);
                if steps != 0 {
                    let _ = app.emit_to("overlay", "lg-app", steps);
                }
            }
            Some(Ev::AppStep(n)) => {
                let _ = app.emit_to("overlay", "lg-app", n);
            }
            Some(Ev::Arrow(dir)) => {
                let _ = app.emit_to("overlay", "lg-arrow", dir);
            }
            Some(Ev::Move(x, y)) => pending = Some((x, y)),
            Some(Ev::Release) => {
                if let Some((x, y)) = pending.take() {
                    let (ox, oy) = overlay::origin();
                    let _ = app.emit_to("overlay", "lg-cursor", Pt { x: x - ox, y: y - oy });
                }
                f.release();
            }
            Some(Ev::Cancel) => f.cancel(),
            Some(Ev::Toggle) => {
                // the overlay knows which source is selected and asks for play/pause itself
                let _ = app.emit_to("overlay", "lg-toggle", ());
            }
            Some(Ev::Click(id)) => {
                let _ = app.emit_to("overlay", "lg-click", id);
            }
            Some(Ev::Replay(r)) => inject_trigger(r),
            Some(Ev::Done(gen)) => {
                overlay::done(gen);
                if overlay::phase() == overlay::HIDDEN {
                    f.closing_since = None;
                }
            }
            None => {}
        }
        // cursor updates at most every ~7 ms, always the latest position
        if let Some((x, y)) = pending {
            if last_move.elapsed() >= Duration::from_millis(7) {
                let (ox, oy) = overlay::origin();
                let _ = app.emit_to("overlay", "lg-cursor", Pt { x: x - ox, y: y - oy });
                pending = None;
                last_move = Instant::now();
            }
        }

        // ---- watchdog, ten times a second ----
        if last_check.elapsed() < Duration::from_millis(100) {
            continue;
        }
        last_check = Instant::now();
        let phase = overlay::phase();
        if SESSION.load(Relaxed) {
            if TRIG_MOUSE.load(Relaxed) == 0 {
                // a keyboard trigger let go without us hearing about it
                if keys_held(TRIG_KEYS.load(Relaxed)) {
                    not_held = 0;
                } else {
                    not_held += 1;
                    if not_held >= 2 {
                        not_held = 0;
                        SESSION.store(false, Relaxed);
                        f.release();
                    }
                }
            } else if f.last_gesture.elapsed() > Duration::from_secs(12) {
                // a mouse-button trigger whose release went missing: no wheel, click or key for 12 s
                SESSION.store(false, Relaxed);
                set_mb(MB_DONE);
                f.cancel();
            }
        } else {
            not_held = 0;
        }
        // open with no gesture behind it (the release was lost between threads)
        if phase == overlay::OPEN && !SESSION.load(Relaxed) {
            orphan += 1;
            if orphan >= 3 {
                orphan = 0;
                f.release();
            }
        } else {
            orphan = 0;
        }
        // closing never finished (the page stopped answering): hide it anyway
        if phase == overlay::CLOSING {
            let limit = Duration::from_millis(if f.closing_long { 4000 } else { 1500 });
            if f.closing_since.map(|t| t.elapsed() > limit).unwrap_or(true) {
                overlay::force_hide(&app);
                f.closing_since = None;
            }
        }
        // on screen although nothing has it open: hide it
        if phase == overlay::HIDDEN && overlay::visible() {
            stray += 1;
            if stray >= 3 {
                stray = 0;
                overlay::force_hide(&app);
            }
        } else {
            stray = 0;
        }
    }
}
