//! Spotify Web API: the real queue (and recently played) for the overlay.
//! Sign-in uses Authorization Code with PKCE and a loopback redirect, so no client secret is
//! stored anywhere. Each person registers their own free Spotify developer app and pastes its
//! Client ID in Settings.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};

use crate::config::save;
use crate::AppState;
use crate::util::Lock;

pub const REDIRECT: &str = "http://127.0.0.1:8898/callback";
const SCOPES: &str = "user-read-playback-state user-modify-playback-state user-read-currently-playing user-read-recently-played user-library-read user-library-modify";
const API: &str = "https://api.spotify.com";

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn random_string(n: usize) -> String {
    const CH: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut bytes = vec![0u8; n];
    let _ = getrandom::getrandom(&mut bytes);
    bytes.iter().map(|b| CH[*b as usize % CH.len()] as char).collect()
}

fn state(app: &AppHandle) -> tauri::State<'_, AppState> {
    app.state::<AppState>()
}

pub fn connected(app: &AppHandle) -> bool {
    let cfg = state(app).cfg.guard().clone();
    !cfg.client_id.is_empty() && cfg.refresh_token.is_some()
}

/// Opens the Spotify sign-in page and waits (up to 3 minutes) for the browser to come back.
pub fn connect(app: &AppHandle) -> Result<(), String> {
    let client_id = state(app).cfg.guard().client_id.trim().to_string();
    if client_id.is_empty() {
        return Err("Paste your Spotify Client ID first.".into());
    }
    let verifier = random_string(64);
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let st = random_string(16);
    let listener = TcpListener::bind("127.0.0.1:8898")
        .map_err(|_| "Port 8898 is busy. Close whatever is using it and try again.".to_string())?;
    let url = format!(
        "https://accounts.spotify.com/authorize?response_type=code&client_id={}&scope={}&redirect_uri={}&code_challenge_method=S256&code_challenge={}&state={}",
        enc(&client_id), enc(SCOPES), enc(REDIRECT), challenge, st
    );
    open::that(&url).map_err(|e| format!("Could not open the browser: {e}"))?;

    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(180);
    let code = loop {
        if Instant::now() > deadline {
            return Err("Sign-in timed out. Try Connect again.".into());
        }
        match listener.accept() {
            Ok((mut sock, _)) => {
                let _ = sock.set_nonblocking(false);
                let _ = sock.set_read_timeout(Some(Duration::from_secs(5)));
                let mut buf = [0u8; 4096];
                let n = sock.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = req.split_whitespace().nth(1).unwrap_or("").to_string();
                if !path.starts_with("/callback") {
                    let _ = sock.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
                    continue;
                }
                let q: HashMap<String, String> = path
                    .split_once('?')
                    .map(|(_, q)| q)
                    .unwrap_or("")
                    .split('&')
                    .filter_map(|kv| kv.split_once('='))
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                let ok = q.get("state") == Some(&st) && q.contains_key("code");
                let body = if ok {
                    "<!doctype html><meta charset=utf-8><title>Liquid Queue</title><body style=\"font:16px Segoe UI,sans-serif;background:#0f1520;color:#fafbfd;display:grid;place-items:center;height:100vh;margin:0\"><p>Spotify is connected. You can close this tab.</p>"
                } else {
                    "<!doctype html><meta charset=utf-8><title>Liquid Queue</title><body style=\"font:16px Segoe UI,sans-serif;background:#0f1520;color:#fafbfd;display:grid;place-items:center;height:100vh;margin:0\"><p>Spotify sign-in was cancelled or failed. Go back to Liquid Queue and try again.</p>"
                };
                let _ = sock.write_all(
                    format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body)
                        .as_bytes(),
                );
                if !ok {
                    return Err(q.get("error").cloned().unwrap_or_else(|| "Sign-in was cancelled.".into()));
                }
                break q["code"].clone();
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(80)),
            Err(e) => return Err(e.to_string()),
        }
    };

    let resp: Value = ureq::post("https://accounts.spotify.com/api/token")
        .timeout(Duration::from_secs(10))
        .send_form(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", REDIRECT),
            ("client_id", &client_id),
            ("code_verifier", &verifier),
        ])
        .map_err(|e| format!("Spotify refused the sign-in: {}", short_err(e)))?
        .into_json()
        .map_err(|e| e.to_string())?;
    store_tokens(app, &resp);
    Ok(())
}

pub fn disconnect(app: &AppHandle) {
    let st = state(app);
    let mut cfg = st.cfg.guard();
    cfg.refresh_token = None;
    cfg.access_token = None;
    cfg.expires_at = 0;
    save(&st.cfg_path, &cfg);
}

fn store_tokens(app: &AppHandle, v: &Value) {
    let st = state(app);
    let mut cfg = st.cfg.guard();
    if let Some(a) = v["access_token"].as_str() {
        cfg.access_token = Some(a.to_string());
        cfg.expires_at = now_secs() + v["expires_in"].as_u64().unwrap_or(3600).saturating_sub(60);
    }
    if let Some(r) = v["refresh_token"].as_str() {
        cfg.refresh_token = Some(r.to_string());
    }
    save(&st.cfg_path, &cfg);
}

fn short_err(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, r) => {
            let body = r.into_string().unwrap_or_default();
            format!("{code} {}", body.chars().take(160).collect::<String>())
        }
        ureq::Error::Transport(t) => t.to_string(),
    }
}

fn token(app: &AppHandle) -> Option<String> {
    let cfg = state(app).cfg.guard().clone();
    if let (Some(a), true) = (&cfg.access_token, cfg.expires_at > now_secs()) {
        return Some(a.clone());
    }
    let refresh = cfg.refresh_token?;
    let resp: Value = ureq::post("https://accounts.spotify.com/api/token")
        .timeout(Duration::from_secs(8))
        .send_form(&[("grant_type", "refresh_token"), ("refresh_token", &refresh), ("client_id", &cfg.client_id)])
        .ok()?
        .into_json()
        .ok()?;
    store_tokens(app, &resp);
    resp["access_token"].as_str().map(String::from)
}

pub fn get(app: &AppHandle, path: &str) -> Result<Value, String> {
    let t = token(app).ok_or("not connected")?;
    let r = ureq::get(&format!("{API}{path}"))
        .set("Authorization", &format!("Bearer {t}"))
        .timeout(Duration::from_secs(6))
        .call()
        .map_err(short_err)?;
    if r.status() == 204 {
        return Ok(Value::Null);
    }
    r.into_json().map_err(|e| e.to_string())
}

pub fn send(app: &AppHandle, method: &str, path: &str, body: Option<Value>) -> Result<(), String> {
    let t = token(app).ok_or("not connected")?;
    let req = ureq::request(method, &format!("{API}{path}"))
        .set("Authorization", &format!("Bearer {t}"))
        .timeout(Duration::from_secs(6));
    let res = match body {
        Some(b) => req.send_json(b),
        None => req.set("Content-Length", "0").call(),
    };
    res.map(|_| ()).map_err(short_err)
}

/// What Spotify is playing from right now (a playlist, album, Liked Songs…), as a URI.
pub fn context(app: &AppHandle) -> Option<String> {
    let v = get(app, "/v1/me/player").ok()?;
    v["context"]["uri"].as_str().filter(|s| !s.is_empty()).map(String::from)
}

/// The Spotify URI of the best match for a song (for playing it again from the history).
pub fn find_track(app: &AppHandle, title: &str, artist: &str) -> Option<String> {
    let q = enc(&format!("{title} {artist}"));
    let v = get(app, &format!("/v1/search?type=track&limit=1&q={q}")).ok()?;
    v["tracks"]["items"][0]["uri"].as_str().map(String::from)
}

// ---------- Liked Songs (the library endpoints since February 2026: Spotify URIs, max 40) ----------

/// Is this song (a Spotify URI) in your Liked Songs?
pub fn library_contains(app: &AppHandle, uri: &str) -> Result<bool, String> {
    let v = get(app, &format!("/v1/me/library/contains?uris={}", enc(uri)))?;
    v.get(0).and_then(|b| b.as_bool()).ok_or_else(|| "Spotify gave no answer".to_string())
}

/// Adds the song to your Liked Songs. Saving never removes anything.
pub fn library_save(app: &AppHandle, uri: &str) -> Result<(), String> {
    send(app, "PUT", &format!("/v1/me/library?uris={}", enc(uri)), None)
}

// ---------- playing on this PC ----------

/// This PC's Spotify app as a playback device: the one named like the PC, else any computer.
fn this_pc_device(app: &AppHandle) -> Option<String> {
    let v = get(app, "/v1/me/player/devices").ok()?;
    let devs = v["devices"].as_array()?;
    let host = std::env::var("COMPUTERNAME").unwrap_or_default().to_lowercase();
    let pick = devs
        .iter()
        .find(|d| d["type"] == "Computer" && d["name"].as_str().is_some_and(|n| n.to_lowercase() == host))
        .or_else(|| devs.iter().find(|d| d["type"] == "Computer"))
        .or_else(|| devs.first())?;
    pick["id"].as_str().map(String::from)
}

/// A player command. When Spotify has no active device (it played nothing for a while), it is
/// sent again to this PC's Spotify app, which is opened first if it isn't running.
pub fn player(app: &AppHandle, method: &str, path: &str, body: Option<Value>) -> Result<(), String> {
    match send(app, method, path, body.clone()) {
        Err(e) if e.contains("NO_ACTIVE_DEVICE") => {
            let mut dev = this_pc_device(app);
            if dev.is_none() {
                let _ = open::that("spotify:");
                for _ in 0..20 {
                    std::thread::sleep(Duration::from_millis(500));
                    dev = this_pc_device(app);
                    if dev.is_some() {
                        break;
                    }
                }
            }
            let dev = dev.ok_or(e)?;
            let sep = if path.contains('?') { '&' } else { '?' };
            send(app, method, &format!("{path}{sep}device_id={}", enc(&dev)), body)
        }
        r => r,
    }
}

/// Waits (up to `ms`) until Spotify reports it is playing this URI.
pub fn playing_uri(app: &AppHandle, uri: &str, ms: u64) -> bool {
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_millis(ms) {
        if let Ok(v) = get(app, "/v1/me/player") {
            if v["is_playing"].as_bool() == Some(true) && v["item"]["uri"].as_str() == Some(uri) {
                return true;
            }
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    false
}

/// The Spotify song that really is this one from another app: same title (after cleaning
/// YouTube extras) and an artist in common. None rather than a wrong song.
pub fn search_track(app: &AppHandle, title: &str, artist: &str) -> Option<String> {
    let (t, a) = crate::extras::clean_song(title, artist);
    let tries = if a.is_empty() { vec![t.clone()] } else { vec![format!("track:{t} artist:{a}"), format!("{t} {a}")] };
    for q in tries {
        let Ok(v) = get(app, &format!("/v1/search?type=track&limit=10&q={}", enc(&q))) else { continue };
        for it in v["tracks"]["items"].as_array().into_iter().flatten() {
            let name = it["name"].as_str().unwrap_or("");
            let artists: Vec<&str> = it["artists"].as_array().into_iter().flatten().filter_map(|x| x["name"].as_str()).collect();
            if crate::extras::same_song(name, &artists, &t, &a) {
                return it["uri"].as_str().map(String::from);
            }
        }
    }
    None
}

/// Title, artist line, art URL and URI for a track or podcast episode object.
pub fn describe(item: &Value) -> Option<(String, String, Option<String>, Option<String>)> {
    if item.is_null() {
        return None;
    }
    let title = item["name"].as_str()?.to_string();
    let artist = if let Some(arr) = item["artists"].as_array() {
        arr.iter().filter_map(|a| a["name"].as_str()).collect::<Vec<_>>().join(", ")
    } else {
        item["show"]["name"].as_str().unwrap_or("").to_string()
    };
    let images = item["album"]["images"].as_array().or_else(|| item["images"].as_array()).or_else(|| item["show"]["images"].as_array());
    let art = images.and_then(|imgs| {
        imgs.iter()
            .min_by_key(|i| (i["width"].as_i64().unwrap_or(640) - 300).abs())
            .and_then(|i| i["url"].as_str())
            .map(String::from)
    });
    let uri = item["uri"].as_str().map(String::from);
    Some((title, artist, art, uri))
}
