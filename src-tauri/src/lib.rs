//! Skip Dat: a liquid glass music wheel for Windows.
//! Hold Alt (or the shortcut you pick) and scroll anywhere; release to play the song you landed on.

#[cfg(not(windows))]
compile_error!("Skip Dat runs on Windows only.");

mod actions;
mod art;
mod audio;
mod autostart;
mod browser;
mod classify;
mod config;
mod history;
mod hooks;
mod media;
mod overlay;
mod plan;
mod skins;
mod spotify;
mod util;
mod view;

use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};
use crate::util::Lock;

pub struct AppState {
    pub snapshot: Mutex<Option<overlay::Snapshot>>,
    pub cfg: Mutex<config::Config>,
    pub cfg_path: PathBuf,
}

pub(crate) fn save_cfg(state: &AppState) {
    let cfg = state.cfg.guard().clone();
    config::save(&state.cfg_path, &cfg);
}

// ---------- the overlay ----------

/// Desktop snapshot as raw bytes: width and height (u32, little-endian) then RGBA rows.
/// (async: copying several megabytes must not hold up the app's UI thread)
#[tauri::command]
async fn get_snapshot(state: tauri::State<'_, AppState>) -> Result<tauri::ipc::Response, String> {
    // handed over once and not kept: the overlay holds it from here on
    let taken = state.snapshot.guard().take();
    let mut out = Vec::new();
    if let Some(s) = taken.as_ref() {
        out.reserve(8 + s.rgba.len());
        out.extend_from_slice(&s.w.to_le_bytes());
        out.extend_from_slice(&s.h.to_le_bytes());
        out.extend_from_slice(&s.rgba);
    }
    Ok(tauri::ipc::Response::new(out))
}

#[tauri::command]
fn get_view(app: AppHandle) -> view::View {
    view::latest(&app)
}

#[tauri::command]
fn choose(source: String, item: String, rel: i32) {
    actions::send(actions::Action::Choose { source, item, rel });
}

#[tauri::command]
fn media_control(source: Option<String>, action: String) {
    actions::send(actions::Action::Control { source, action });
}

/// "Back to …": the song that was playing before the last skip or jump.
#[tauri::command]
fn go_back() {
    actions::send(actions::Action::Back);
}

#[tauri::command]
fn overlay_done(gen: u32) {
    hooks::overlay_done(gen);
}

#[tauri::command]
fn set_hit_rects(gen: u32, rects: Vec<overlay::Rect>) {
    overlay::set_hits(gen, rects);
}

// ---------- settings ----------

#[derive(serde::Serialize)]
struct Companion {
    /// seconds since a browser tab last talked to Skip Dat
    last_seen: Option<u64>,
    error: Option<String>,
    folder: String,
    folder_exists: bool,
}

#[derive(serde::Serialize)]
struct SettingsView {
    version: &'static str,
    trigger: String,
    trigger_label: &'static str,
    layout: String,
    pause_others: bool,
    autostart: bool,
    client_id: String,
    spotify_connected: bool,
    redirect_uri: &'static str,
    config_file: String,
    companion: Companion,
    look: Look,
    skins: Vec<skins::Skin>,
    skins_folder: String,
}

fn companion_folder() -> PathBuf {
    std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("browser-companion"))).unwrap_or_default()
}

#[tauri::command]
fn settings_get(app: AppHandle, state: tauri::State<'_, AppState>) -> SettingsView {
    let cfg = state.cfg.guard().clone();
    let (last_seen, error) = browser::status();
    let folder = companion_folder();
    SettingsView {
        version: env!("CARGO_PKG_VERSION"),
        trigger_label: hooks::trigger_label(&cfg.trigger),
        trigger: cfg.trigger,
        layout: cfg.layout,
        pause_others: cfg.pause_others,
        autostart: autostart::enabled(),
        client_id: cfg.client_id,
        spotify_connected: spotify::connected(&app),
        redirect_uri: spotify::REDIRECT,
        config_file: state.cfg_path.display().to_string(),
        companion: Companion { last_seen, error, folder_exists: folder.is_dir(), folder: folder.display().to_string() },
        look: look_of(&state),
        skins: skins::list(),
        skins_folder: skins::dir().map(|d| d.display().to_string()).unwrap_or_default(),
    }
}

// ---------- the carousel's look (1.4): style, size, depth, performance mode, skin ----------

#[derive(Clone, serde::Serialize)]
pub struct Look {
    style: String,
    layout: String,
    size: String,
    depth_blur: f64,
    depth_fade: f64,
    perf: bool,
    skin: skins::Skin,
}

pub(crate) fn look_of(state: &AppState) -> Look {
    let c = state.cfg.guard().clone();
    Look { style: c.style, layout: c.layout, size: c.size, depth_blur: c.depth_blur, depth_fade: c.depth_fade, perf: c.perf, skin: skins::get(&c.skin) }
}

/// Liquid glass shows the desktop through the cards, so the overlay photographs it when it
/// opens; performance mode and solid skins skip that.
pub(crate) fn needs_desktop(state: &AppState) -> bool {
    let (perf, skin) = {
        let c = state.cfg.guard();
        (c.perf, c.skin.clone())
    };
    !perf && skins::needs_desktop(&skins::get(&skin))
}

fn send_look(app: &AppHandle) {
    let look = look_of(&app.state::<AppState>());
    let _ = app.emit("lg-look", look);
}

#[tauri::command]
fn get_look(state: tauri::State<'_, AppState>) -> Look {
    look_of(&state)
}

/// Change any of: style, layout, size, depth_blur, depth_fade, perf, skin.
#[tauri::command]
fn settings_set_look(app: AppHandle, state: tauri::State<'_, AppState>, patch: serde_json::Value) -> Result<(), String> {
    {
        let mut c = state.cfg.guard();
        if let Some(v) = patch["style"].as_str() {
            if !["wheel", "stack", "coverflow", "orbit", "compact"].contains(&v) {
                return Err("Unknown style".into());
            }
            c.style = v.into();
        }
        if let Some(v) = patch["layout"].as_str() {
            if v != "vertical" && v != "horizontal" {
                return Err("Unknown layout".into());
            }
            c.layout = v.into();
        }
        if let Some(v) = patch["size"].as_str() {
            if !["small", "medium", "large"].contains(&v) {
                return Err("Unknown size".into());
            }
            c.size = v.into();
        }
        if let Some(v) = patch["depth_blur"].as_f64() {
            c.depth_blur = v.clamp(0.0, 1.0);
        }
        if let Some(v) = patch["depth_fade"].as_f64() {
            c.depth_fade = v.clamp(0.0, 1.0);
        }
        if let Some(v) = patch["perf"].as_bool() {
            c.perf = v;
        }
        if let Some(v) = patch["skin"].as_str() {
            if !skins::list().iter().any(|s| s.id == v) {
                return Err("That skin isn't there any more".into());
            }
            c.skin = v.into();
        }
    }
    save_cfg(&state);
    send_look(&app);
    Ok(())
}

#[tauri::command]
fn skins_list() -> Vec<skins::Skin> {
    skins::list()
}

#[tauri::command]
async fn open_skins_folder() -> Result<(), String> {
    let d = skins::dir().ok_or("no skins folder")?;
    let _ = std::fs::create_dir_all(&d);
    open::that(d).map_err(|e| e.to_string())
}

#[tauri::command]
async fn open_skins_guide() -> Result<(), String> {
    let d = skins::dir().ok_or("no skins folder")?;
    open::that(d.join("SKINS.md")).map_err(|e| e.to_string())
}

// ---------- play history ----------

#[tauri::command]
fn history_list() -> Vec<history::Entry> {
    history::list()
}

#[tauri::command]
fn history_clear(app: AppHandle) {
    history::clear();
    let _ = app.emit("lg-history", ());
}

#[tauri::command]
fn history_play(id: u64) {
    actions::send(actions::Action::Replay { id });
}

/// Opens "What was I playing?". A command that makes a window must be async: a plain one runs on
/// the main thread, and making a window there locks Skip Dat up on Windows.
#[tauri::command]
async fn open_history(app: AppHandle) {
    show_history(&app);
}

fn show_history(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("history") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let mut b = WebviewWindowBuilder::new(app, "history", WebviewUrl::App("history.html".into()))
        .title("Skip Dat: what was I playing?")
        .inner_size(460.0, 640.0)
        .min_inner_size(360.0, 360.0)
        .resizable(true)
        .theme(Some(tauri::Theme::Dark))
        .center();
    if windows_build() >= 22000 {
        b = b
            .transparent(true)
            .effects(tauri::window::EffectsBuilder::new().effect(tauri::window::Effect::Mica).build())
            .initialization_script("window.__SKIPDAT_MICA = true;");
    }
    let _ = b.build();
}

fn tooltip(trigger: &str) -> String {
    format!("Skip Dat: hold {} and scroll", hooks::trigger_label(trigger))
}

#[tauri::command]
fn settings_set_trigger(app: AppHandle, state: tauri::State<'_, AppState>, trigger: String) -> Result<String, String> {
    if hooks::parse_trigger(&trigger).is_none() {
        return Err("Unknown shortcut".into());
    }
    hooks::set_trigger(&trigger);
    state.cfg.guard().trigger = trigger.clone();
    save_cfg(&state);
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(tooltip(&trigger)));
    }
    Ok(hooks::trigger_label(&trigger).to_string())
}

/// Vertical or horizontal wheel; the overlay switches over smoothly, no restart.
#[tauri::command]
fn settings_set_layout(app: AppHandle, state: tauri::State<'_, AppState>, layout: String) -> Result<(), String> {
    if layout != "vertical" && layout != "horizontal" {
        return Err("Unknown layout".into());
    }
    state.cfg.guard().layout = layout.clone();
    save_cfg(&state);
    let _ = app.emit_to("overlay", "lg-layout", layout);
    send_look(&app);
    Ok(())
}

#[tauri::command]
fn settings_set_pause_others(state: tauri::State<'_, AppState>, on: bool) {
    state.cfg.guard().pause_others = on;
    save_cfg(&state);
}

/// "Show anyway" for a media source Skip Dat does not count as music (or hide it again).
#[tauri::command]
fn settings_allow(state: tauri::State<'_, AppState>, id: String, on: bool) {
    {
        let mut cfg = state.cfg.guard();
        cfg.allow.retain(|a| a != &id);
        if on {
            cfg.allow.push(id);
        }
    }
    save_cfg(&state);
    view::changed();
}

#[tauri::command]
fn settings_set_autostart(on: bool) -> Result<(), String> {
    autostart::set(on)
}

#[tauri::command]
fn settings_set_client_id(state: tauri::State<'_, AppState>, client_id: String) {
    {
        let mut cfg = state.cfg.guard();
        let id = client_id.trim().to_string();
        if id != cfg.client_id {
            cfg.refresh_token = None;
            cfg.access_token = None;
            cfg.expires_at = 0;
        }
        cfg.client_id = id;
    }
    save_cfg(&state);
}

#[tauri::command]
async fn spotify_connect(app: AppHandle) -> Result<(), String> {
    let app2 = app.clone();
    let res = tauri::async_runtime::spawn_blocking(move || spotify::connect(&app2)).await.map_err(|e| e.to_string())?;
    if res.is_ok() {
        std::thread::spawn(move || view::refresh_spotify(&app, true));
    }
    res
}

#[tauri::command]
fn spotify_disconnect(app: AppHandle) {
    spotify::disconnect(&app);
    std::thread::spawn(move || view::refresh_spotify(&app, true));
}

#[tauri::command]
async fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("only https links".into());
    }
    open::that(url).map_err(|e| e.to_string())
}

#[tauri::command]
async fn open_companion_folder() -> Result<(), String> {
    let f = companion_folder();
    if !f.is_dir() {
        return Err(format!("The browser-companion folder is missing next to Skip Dat ({}).", f.display()));
    }
    open::that(f).map_err(|e| e.to_string())
}

/// Windows' build number (22000 and up is Windows 11), from the registry; 0 if unknown.
fn windows_build() -> u32 {
    use windows::core::w;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
    let mut buf = [0u16; 32];
    let mut len = (buf.len() * 2) as u32;
    let ok = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"),
            w!("CurrentBuildNumber"),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut len),
        )
        .is_ok()
    };
    if !ok {
        return 0;
    }
    String::from_utf16_lossy(&buf[..(len as usize / 2).saturating_sub(1)]).trim().parse().unwrap_or(0)
}

fn open_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let mut b = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("Skip Dat")
        .inner_size(600.0, 820.0)
        .min_inner_size(460.0, 560.0)
        .resizable(true)
        .theme(Some(tauri::Theme::Dark))
        .center();
    // Windows 11: the window gets Mica, Windows' own frosted glass behind the page (drawn by
    // Windows, free for us). Earlier Windows keep the page's own opaque background.
    if windows_build() >= 22000 {
        b = b
            .transparent(true)
            .effects(tauri::window::EffectsBuilder::new().effect(tauri::window::Effect::Mica).build())
            .initialization_script("window.__SKIPDAT_MICA = true;");
    }
    let _ = b.build();
}

/// Album art for the windows: `skipart://localhost/<id>` (content-addressed, so cached forever).
fn art_response(path: &str) -> tauri::http::Response<Cow<'static, [u8]>> {
    let id = path.trim_start_matches('/').split(['?', '#']).next().unwrap_or("");
    let found = art::get(id).map(|(m, b)| (m, b.as_ref().clone())).or_else(|| history::art_bytes(id).map(|b| (art::sniff(&b).to_string(), b)));
    match found {
        Some((mime, bytes)) => tauri::http::Response::builder()
            .status(200)
            .header("Content-Type", mime)
            .header("Cache-Control", "max-age=31536000, immutable")
            // the overlay draws covers with WebGL, which needs permission to read the pixels
            .header("Access-Control-Allow-Origin", "*")
            .body(Cow::Owned(bytes))
            .unwrap(),
        None => tauri::http::Response::builder()
            .status(404)
            .header("Access-Control-Allow-Origin", "*")
            .body(Cow::Borrowed(&[][..]))
            .unwrap(),
    }
}

/// A skin's own pictures and fonts: `skipskin://localhost/<folder>/<file>`.
fn skin_response(path: &str) -> tauri::http::Response<Cow<'static, [u8]>> {
    match skins::file(path) {
        Some((mime, bytes)) => tauri::http::Response::builder()
            .status(200)
            .header("Content-Type", mime)
            .header("Cache-Control", "no-cache")
            .header("Access-Control-Allow-Origin", "*")
            .body(Cow::Owned(bytes))
            .unwrap(),
        None => tauri::http::Response::builder().status(404).header("Access-Control-Allow-Origin", "*").body(Cow::Borrowed(&[][..])).unwrap(),
    }
}

pub fn run() {
    util::log_panics();
    let app = tauri::Builder::default()
        .register_uri_scheme_protocol("skipart", |_ctx, req| art_response(req.uri().path()))
        .register_uri_scheme_protocol("skipskin", |_ctx, req| skin_response(req.uri().path()))
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            let cfg_path = dir.join("config.json");
            let first_run = !cfg_path.exists();
            if first_run {
                // keep the settings of the earlier "Liquid Queue" builds
                if let Some(old) = dir.parent().map(|p| p.join("app.liquidqueue.desktop").join("config.json")) {
                    if old.exists() {
                        let _ = std::fs::create_dir_all(&dir);
                        let _ = std::fs::copy(&old, &cfg_path);
                    }
                }
            }
            util::set_log_dir(dir.clone());
            audio::init(dir.clone());
            history::init(dir.clone());
            skins::init(dir.join("skins"), include_str!("../../SKINS.md"), include_str!("../../ui/skins/example-skin.json"));
            let mut cfg = config::load(&cfg_path);
            if hooks::parse_trigger(&cfg.trigger).is_none() {
                cfg.trigger = "alt".into();
            }
            config::save(&cfg_path, &cfg);
            hooks::set_trigger(&cfg.trigger);
            let trigger = cfg.trigger.clone();
            app.manage(AppState { snapshot: Mutex::new(None), cfg: Mutex::new(cfg), cfg_path });

            let handle = app.handle().clone();
            overlay::init(&handle);
            view::start(handle.clone());
            actions::start(handle.clone());
            media::start(view::changed);
            browser::start(view::changed);
            hooks::start(handle.clone());
            let h2 = handle.clone();
            skins::watch(move || {
                let _ = h2.emit("lg-skins", skins::list());
                send_look(&h2);
            });

            let settings = MenuItem::with_id(app, "settings", "Settings and media…", true, None::<&str>)?;
            let hist = MenuItem::with_id(app, "history", "What was I playing?", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Skip Dat", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings, &hist, &quit])?;
            let mut tray = TrayIconBuilder::with_id("main")
                .tooltip(tooltip(&trigger))
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, e| match e.id().as_ref() {
                    "settings" => open_settings(app),
                    "history" => show_history(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, e| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = e {
                        open_settings(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            if first_run {
                open_settings(&handle);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            get_view,
            choose,
            media_control,
            go_back,
            overlay_done,
            set_hit_rects,
            settings_get,
            settings_set_trigger,
            settings_set_layout,
            settings_set_pause_others,
            settings_allow,
            settings_set_autostart,
            settings_set_client_id,
            spotify_connect,
            spotify_disconnect,
            open_url,
            open_companion_folder,
            get_look,
            settings_set_look,
            skins_list,
            open_skins_folder,
            open_skins_guide,
            history_list,
            history_clear,
            history_play,
            open_history
        ])
        .build(tauri::generate_context!())
        .expect("Skip Dat failed to start");

    app.run(|_app, event| {
        // closing the settings window must not quit the app; only the tray's Quit does
        if let RunEvent::ExitRequested { api, code, .. } = event {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}
