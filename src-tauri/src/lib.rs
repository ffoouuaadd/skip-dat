//! Skip Dat: a liquid glass music wheel for Windows.
//! Hold Alt (or the shortcut you pick) and scroll anywhere; release to play the song you landed on.

#[cfg(not(windows))]
compile_error!("Skip Dat runs on Windows only.");

mod actions;
mod art;
mod autostart;
mod browser;
mod classify;
mod config;
mod hooks;
mod media;
mod overlay;
mod spotify;
mod view;

use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};

pub struct AppState {
    pub snapshot: Mutex<Option<overlay::Snapshot>>,
    pub cfg: Mutex<config::Config>,
    pub cfg_path: PathBuf,
}

fn save_cfg(state: &AppState) {
    let cfg = state.cfg.lock().unwrap().clone();
    config::save(&state.cfg_path, &cfg);
}

// ---------- the overlay ----------

/// Desktop snapshot as raw bytes: width and height (u32, little-endian) then RGBA rows.
/// (async: copying several megabytes must not hold up the app's UI thread)
#[tauri::command]
async fn get_snapshot(state: tauri::State<'_, AppState>) -> Result<tauri::ipc::Response, String> {
    let g = state.snapshot.lock().unwrap();
    let mut out = Vec::new();
    if let Some(s) = g.as_ref() {
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
}

fn companion_folder() -> PathBuf {
    std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("browser-companion"))).unwrap_or_default()
}

#[tauri::command]
fn settings_get(app: AppHandle, state: tauri::State<'_, AppState>) -> SettingsView {
    let cfg = state.cfg.lock().unwrap().clone();
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
    }
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
    state.cfg.lock().unwrap().trigger = trigger.clone();
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
    state.cfg.lock().unwrap().layout = layout.clone();
    save_cfg(&state);
    let _ = app.emit_to("overlay", "lg-layout", layout);
    Ok(())
}

#[tauri::command]
fn settings_set_pause_others(state: tauri::State<'_, AppState>, on: bool) {
    state.cfg.lock().unwrap().pause_others = on;
    save_cfg(&state);
}

/// "Show anyway" for a media source Skip Dat does not count as music (or hide it again).
#[tauri::command]
fn settings_allow(state: tauri::State<'_, AppState>, id: String, on: bool) {
    {
        let mut cfg = state.cfg.lock().unwrap();
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
        let mut cfg = state.cfg.lock().unwrap();
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
    match art::get(id) {
        Some((mime, bytes)) => tauri::http::Response::builder()
            .status(200)
            .header("Content-Type", mime)
            .header("Cache-Control", "max-age=31536000, immutable")
            // the overlay draws covers with WebGL, which needs permission to read the pixels
            .header("Access-Control-Allow-Origin", "*")
            .body(Cow::Owned(bytes.as_ref().clone()))
            .unwrap(),
        None => tauri::http::Response::builder()
            .status(404)
            .header("Access-Control-Allow-Origin", "*")
            .body(Cow::Borrowed(&[][..]))
            .unwrap(),
    }
}

pub fn run() {
    let app = tauri::Builder::default()
        .register_uri_scheme_protocol("skipart", |_ctx, req| art_response(req.uri().path()))
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

            let settings = MenuItem::with_id(app, "settings", "Settings and media…", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Skip Dat", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings, &quit])?;
            let mut tray = TrayIconBuilder::with_id("main")
                .tooltip(tooltip(&trigger))
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, e| match e.id().as_ref() {
                    "settings" => open_settings(app),
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
            open_companion_folder
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
