//! The wheel's extra controls. They run on the actions thread (actions.rs), in order with every
//! other action, and say what they did in a short message on the wheel ("lg-toast").
//!
//! Volume: scrolling over the play/pause button changes the volume of the app in front, 5% a
//! notch. Desktop apps (Spotify, Apple Music…) through Windows' Volume mixer, so only that app
//! changes; a YouTube or YouTube Music tab through its own player, so only that tab changes.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::json;
use tauri::{AppHandle, Emitter};

use crate::util::{log, Lock};
use crate::view::{self, Item, Source, Target};
use crate::{audio, browser};

/// Volume change per wheel notch (0–1).
pub const VOL_STEP: f32 = 0.05;

pub fn toast(app: &AppHandle, text: &str) {
    let _ = app.emit_to("overlay", "lg-toast", text);
}

fn find(app: &AppHandle, source: &str) -> Option<Source> {
    view::latest(app).sources.into_iter().find(|s| s.key == source)
}

/// The level after `steps` notches (up is positive), never below 0 or above 1.
pub fn volume_after(level: f32, steps: i32) -> f32 {
    (level + steps as f32 * VOL_STEP).clamp(0.0, 1.0)
}

fn percent(level: f32) -> i32 {
    (level * 100.0).round() as i32
}

pub fn volume(app: &AppHandle, source: &str, steps: i32) {
    if steps == 0 {
        return;
    }
    let Some(s) = find(app, source) else { return };
    match &s.target {
        Target::Smtc(id) => match audio::nudge(id, |v| volume_after(v, steps)) {
            Some(v) => toast(app, &format!("{} volume {}%", s.name, percent(v))),
            None => toast(app, &format!("{} hasn't played any sound yet", s.name)),
        },
        Target::Tab(_) if s.site == "soundcloud" => toast(app, "SoundCloud's volume only moves with its own slider"),
        Target::Tab(key) => {
            let by = steps * percent(VOL_STEP);
            browser::push(key, json!({ "type": "volume", "by": by }));
            // the tab reports its volume with every update; say where this step takes it
            let now = browser::tabs().into_iter().find(|t| &t.key == key).and_then(|t| t.report.volume);
            match now {
                Some(v) => toast(app, &format!("{} volume {}%", s.name, percent(volume_after(v as f32 / 100.0, steps)))),
                None => toast(app, if steps > 0 { "Volume up" } else { "Volume down" }),
            }
        }
    }
}

// ---------- jumping within the song playing (scroll over its card) ----------

/// Where a jump of `secs` lands: never before the start, never past the song's last second
/// (`len` 0: length unknown, no upper limit).
pub fn seek_target(pos: f64, secs: f64, len: f64) -> f64 {
    let t = (pos + secs).max(0.0);
    if len > 1.0 { t.min(len - 1.0) } else { t }
}

/// 83.9 → "1:23"
pub fn clock(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

pub fn seek(app: &AppHandle, source: &str, secs: i32) {
    if secs == 0 {
        return;
    }
    let Some(s) = find(app, source) else { return };
    let dir = if secs > 0 { "Forward" } else { "Back" };
    match &s.target {
        Target::Smtc(id) => {
            // the player's own Windows media controls first (no network, no connection needed)
            if let Some((to, len)) = crate::media::with_session(id, |x| crate::media::seek_by(x, secs as f64)).flatten() {
                return toast(app, &format!("{dir} 10 s · {} / {}", clock(to), clock(len)));
            }
            // Spotify doesn't take it from Windows' controls: its Web API does, when connected
            if crate::media::is_spotify(id) && crate::spotify::connected(app) {
                if let Some((to, len)) = spotify_seek(app, secs) {
                    return toast(app, &format!("{dir} 10 s · {} / {}", clock(to), clock(len)));
                }
            }
            toast(app, &format!("{} doesn't let other apps jump within a song", s.name));
        }
        Target::Tab(_) if s.site == "soundcloud" => toast(app, "SoundCloud jumps within a song only with its own bar"),
        Target::Tab(key) => {
            browser::push(key, json!({ "type": "seek", "by": secs }));
            toast(app, &format!("{dir} {} s", secs.abs()));
        }
    }
}

fn spotify_seek(app: &AppHandle, secs: i32) -> Option<(f64, f64)> {
    let v = crate::spotify::get(app, "/v1/me/player").ok()?;
    let pos = v["progress_ms"].as_f64()? / 1000.0;
    let len = v["item"]["duration_ms"].as_f64().unwrap_or(0.0) / 1000.0;
    let to = seek_target(pos, secs as f64, len);
    crate::spotify::send(app, "PUT", &format!("/v1/me/player/seek?position_ms={}", (to * 1000.0) as u64), None).ok()?;
    Some((to, len))
}

// ---------- the song buttons: like, copy link, play next, continue on Spotify ----------

/// The same button on the same song within 1.5 s is a double click: done once.
static LAST: Mutex<Option<(String, Instant)>> = Mutex::new(None);

fn first_press(key: &str) -> bool {
    let mut g = LAST.guard();
    if g.as_ref().is_some_and(|(k, t)| k == key && t.elapsed() < Duration::from_millis(1500)) {
        return false;
    }
    *g = Some((key.to_string(), Instant::now()));
    true
}

fn wait_for(ms: u64, mut done: impl FnMut() -> bool) -> bool {
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_millis(ms) {
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    done()
}

/// What Spotify's refusals mean for the person.
fn spotify_error(app: &AppHandle, e: &str) {
    if e.contains("PREMIUM") {
        toast(app, "Spotify only lets Premium accounts do this");
    } else if e.starts_with("401") || e.starts_with("403") {
        toast(app, "Press Reconnect under Spotify in Settings to allow this");
    } else {
        log(&format!("spotify: {e}"));
        toast(app, "Spotify didn't do it (see skipdat.log)");
    }
}

/// Why a song button doesn't work for this source (the wheel shows it dimmed).
fn unavailable(s: &Source, what: &str) -> String {
    let spotify = matches!(&s.target, Target::Smtc(id) if crate::media::is_spotify(id));
    match what {
        _ if spotify && what != "spotify" => "Connect Spotify in Settings to use this".into(),
        "spotify" if spotify => "This is Spotify already".into(),
        "spotify" => "Connect Spotify in Settings to continue there".into(),
        "next" => "Play next works for Spotify songs".into(),
        _ => format!("{} doesn't share that with other apps", s.name),
    }
}

/// Returns true when Spotify's queue changed (the wheel reads it again).
pub fn song(app: &AppHandle, source: &str, item: &str, what: &str) -> bool {
    if !first_press(&format!("{what}\u{1f}{source}\u{1f}{item}")) {
        return false;
    }
    let Some(s) = find(app, source) else { return false };
    if !s.acts.contains(&what) {
        toast(app, &unavailable(&s, what));
        return false;
    }
    let now = s.items.iter().find(|i| i.rel == 0).cloned();
    let Some(it) = s.items.iter().find(|i| i.key == item).cloned().or_else(|| now.clone()) else { return false };
    match what {
        "like" => like(app, &s, &it),
        "link" => link(app, &s, &it),
        "next" => return play_next(app, &it),
        // the song playing in this app, whichever card is in front
        "spotify" => return now.is_some_and(|n| to_spotify(app, &s, &n)),
        _ => {}
    }
    false
}

fn tab_liked(key: &str) -> Option<bool> {
    browser::tabs().into_iter().find(|t| t.key == key).and_then(|t| t.report.liked)
}

/// Never takes a like back: Spotify is asked first, a site's button is read first.
fn like(app: &AppHandle, s: &Source, it: &Item) {
    match &s.target {
        Target::Smtc(_) => {
            let Some(uri) = it.uri.clone() else { return toast(app, "Spotify didn't say which song this is") };
            match crate::spotify::library_contains(app, &uri) {
                Ok(true) => {
                    view::set_spotify_liked(&uri, true);
                    toast(app, &format!("{} is already in your Liked Songs", it.title));
                }
                Ok(false) => match crate::spotify::library_save(app, &uri) {
                    Ok(()) if crate::spotify::library_contains(app, &uri) == Ok(true) => {
                        view::set_spotify_liked(&uri, true);
                        toast(app, &format!("Added {} to your Liked Songs", it.title));
                    }
                    Ok(()) => toast(app, "Spotify didn't keep the like"),
                    Err(e) => spotify_error(app, &e),
                },
                Err(e) => spotify_error(app, &e),
            }
        }
        Target::Tab(_) if it.rel != 0 => toast(app, "Only the song playing can be liked here"),
        Target::Tab(key) => match s.liked {
            Some(true) => toast(app, &format!("{} is already liked", it.title)),
            None => toast(app, &format!("{} shows no like button for this", s.name)),
            Some(false) => {
                browser::push(key, json!({ "type": "like" }));
                // the page reports the pressed button within a second
                if wait_for(2000, || tab_liked(key) == Some(true)) {
                    toast(app, &format!("Liked on {}", s.name));
                } else {
                    toast(app, &format!("{} didn't take the like: are you signed in there?", s.name));
                }
            }
        },
    }
}

/// "spotify:track:abc" → https://open.spotify.com/track/abc
pub fn spotify_link(uri: &str) -> Option<String> {
    let mut p = uri.split(':');
    match (p.next(), p.next(), p.next()) {
        (Some("spotify"), Some(kind), Some(id)) if !kind.is_empty() && !id.is_empty() => Some(format!("https://open.spotify.com/{kind}/{id}")),
        _ => None,
    }
}

fn link(app: &AppHandle, s: &Source, it: &Item) {
    let l = match &s.target {
        Target::Smtc(_) => it.uri.as_deref().and_then(spotify_link),
        Target::Tab(_) if it.rel == 0 => it.url.clone().filter(|u| u.starts_with("https://")),
        Target::Tab(_) => return toast(app, "Only the song playing has a link here"),
    };
    match l {
        // the site is enough to say which link it was; the whole address wouldn't fit
        Some(l) if set_clipboard(&l) => toast(app, &format!("Link copied · {}", l.trim_start_matches("https://").split('/').next().unwrap_or(""))),
        Some(_) => toast(app, "Couldn't reach the clipboard; try again"),
        None => toast(app, "No link for this song"),
    }
}

/// Puts text on the Windows clipboard (as Unicode, so any title or link survives).
pub fn set_clipboard(text: &str) -> bool {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData};
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::System::Ole::CF_UNICODETEXT;
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        // another app may be holding the clipboard for a moment
        if !wait_for(400, || OpenClipboard(None).is_ok()) {
            return false;
        }
        let put = || -> windows::core::Result<()> {
            EmptyClipboard()?;
            let h = GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2)?;
            let p = GlobalLock(h) as *mut u16;
            if p.is_null() {
                return Err(windows::core::Error::from_thread());
            }
            std::ptr::copy_nonoverlapping(wide.as_ptr(), p, wide.len());
            let _ = GlobalUnlock(h);
            // the clipboard owns the memory from here on
            SetClipboardData(CF_UNICODETEXT.0 as u32, Some(HANDLE(h.0)))?;
            Ok(())
        };
        let ok = put().is_ok();
        let _ = CloseClipboard();
        ok
    }
}

fn play_next(app: &AppHandle, it: &Item) -> bool {
    let Some(uri) = it.uri.clone() else {
        toast(app, "Spotify didn't say which song this is");
        return false;
    };
    match it.rel {
        0 => toast(app, "That's the song playing"),
        1 => toast(app, &format!("{} is already up next", it.title)),
        _ => match crate::spotify::player(app, "POST", &format!("/v1/me/player/queue?uri={}", crate::spotify::enc(&uri)), None) {
            Ok(()) => {
                toast(app, &format!("{} plays next", it.title));
                return true;
            }
            Err(e) => spotify_error(app, &e),
        },
    }
    false
}

/// Seconds into the song playing in this source: from Windows' media controls (a tab: its
/// browser's session showing the same song).
fn position_of(s: &Source, title: &str) -> f64 {
    let id = match &s.target {
        Target::Smtc(id) => Some(id.clone()),
        Target::Tab(_) => crate::media::snapshot()
            .0
            .into_iter()
            .find(|a| matches!(crate::classify::app_kind(&a.app), crate::classify::AppKind::Browser(_)) && crate::classify::same_title(&a.now.title, title))
            .map(|a| a.id),
    };
    id.and_then(|id| crate::media::with_session(&id, crate::media::position_secs)).unwrap_or(0.0)
}

/// Finds the song on Spotify, starts it there at the same moment and, once Spotify really plays
/// it, pauses this app. If anything fails on the way, this app keeps playing untouched.
fn to_spotify(app: &AppHandle, s: &Source, now: &Item) -> bool {
    toast(app, &format!("Looking for {} on Spotify…", now.title));
    let Some(uri) = crate::spotify::search_track(app, &now.title, &now.artist) else {
        toast(app, &format!("Couldn't find {} on Spotify", now.title));
        return false;
    };
    let pos = position_of(s, &now.title);
    let body = json!({ "uris": [uri], "position_ms": (pos * 1000.0) as u64 });
    if let Err(e) = crate::spotify::player(app, "PUT", "/v1/me/player/play", Some(body)) {
        spotify_error(app, &e);
        return false;
    }
    if !crate::spotify::playing_uri(app, &uri, 5000) {
        toast(app, "Spotify didn't start it, so nothing was paused");
        return false;
    }
    if s.playing {
        pause(s);
    }
    toast(app, &format!("Playing on Spotify from {}", clock(pos)));
    true
}

fn pause(s: &Source) {
    match &s.target {
        Target::Smtc(id) => {
            crate::media::with_session(id, |x| crate::media::act(x, "pause"));
        }
        Target::Tab(key) => browser::push(key, json!({ "type": "pause" })),
    }
}

/// Text inside (), [] and {} removed, spaces tidied: "Song (Official Video)" → "Song"
fn strip_brackets(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0u32;
    for c in s.chars() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A song's title and artist from another app, cleaned for finding it on Spotify (lowercase).
/// YouTube puts "Artist - Song (Official Video)" in the title and a channel name ("…VEVO",
/// "… - Topic") as the artist.
pub fn clean_song(title: &str, artist: &str) -> (String, String) {
    let channel = artist.ends_with("VEVO") || artist.ends_with(" - Topic");
    let mut a = artist.trim_end_matches(" - Topic").trim_end_matches("VEVO").trim().to_lowercase();
    let mut t = strip_brackets(&title.to_lowercase());
    if let Some((x, _)) = t.split_once(" | ") {
        t = x.trim().to_string();
    }
    for tail in [" ft. ", " feat. ", " ft ", " featuring "] {
        if let Some(i) = t.find(tail) {
            t.truncate(i);
        }
    }
    let squash = |s: &str| s.replace(' ', "");
    if let Some((x, y)) = t.clone().split_once(" - ") {
        let x = x.trim();
        if a.is_empty() || channel || squash(&a).contains(&squash(x)) || squash(x).contains(&squash(&a)) {
            if a.is_empty() || channel {
                a = x.to_string();
            }
            t = y.trim().to_string();
        }
    }
    (t.trim().to_string(), a)
}

/// A Spotify result (name, artists) is this song (cleaned title and artist): the same title,
/// ignoring brackets and " - Remastered…" tails, and an artist in common.
pub fn same_song(name: &str, artists: &[&str], title: &str, artist: &str) -> bool {
    let mut n = strip_brackets(&name.to_lowercase());
    if let Some((x, _)) = n.split_once(" - ") {
        n = x.trim().to_string();
    }
    let t2 = title.split_once(" - ").map(|(x, _)| x.trim()).unwrap_or(title);
    let squash = |s: &str| s.to_lowercase().replace(' ', "");
    let a = squash(artist);
    let artist_ok = a.is_empty() || artists.iter().map(|x| squash(x)).any(|x| !x.is_empty() && (a.contains(&x) || x.contains(&a)));
    !n.is_empty() && (n == title || n == t2) && artist_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn songs_from_youtube_are_cleaned_for_spotify() {
        assert_eq!(clean_song("Ochre Against Blue", "Memotone"), ("ochre against blue".into(), "memotone".into()));
        assert_eq!(clean_song("Daft Punk - Get Lucky (Official Audio) ft. Pharrell Williams", "DaftPunkVEVO"), ("get lucky".into(), "daft punk".into()));
        assert_eq!(clean_song("Get Lucky", "Daft Punk - Topic"), ("get lucky".into(), "daft punk".into()));
        assert_eq!(clean_song("Song Name | Official Video", "Some Band"), ("song name".into(), "some band".into()));
        assert_eq!(clean_song("Love - Live", "Lana"), ("love - live".into(), "lana".into()), "a dash that isn't 'artist - song' stays");
    }

    #[test]
    fn only_the_same_song_matches() {
        assert!(same_song("Get Lucky (feat. Pharrell Williams)", &["Daft Punk", "Pharrell Williams"], "get lucky", "daft punk"));
        assert!(same_song("Get Lucky - Radio Edit", &["Daft Punk"], "get lucky", "daft punk"));
        assert!(!same_song("Get Lucky", &["Someone Else"], "get lucky", "daft punk"), "another artist");
        assert!(!same_song("Love Story", &["Lana"], "love", "lana"), "not just the start of a title");
        assert!(same_song("Love", &["Anyone"], "love", ""), "no artist known: the title decides");
    }

    #[test]
    fn spotify_links() {
        assert_eq!(spotify_link("spotify:track:abc").as_deref(), Some("https://open.spotify.com/track/abc"));
        assert_eq!(spotify_link("spotify:episode:e1").as_deref(), Some("https://open.spotify.com/episode/e1"));
        assert_eq!(spotify_link("https://x"), None);
        assert_eq!(spotify_link("spotify:track:"), None);
    }

    #[test]
    fn a_double_click_is_one_press() {
        assert!(first_press("like\u{1f}test-src\u{1f}k"));
        assert!(!first_press("like\u{1f}test-src\u{1f}k"));
        assert!(first_press("link\u{1f}test-src\u{1f}k"), "another button is a new press");
    }

    /// Live: puts a link on this PC's clipboard and reads it back, then restores what was there.
    ///   cargo test --lib live_clipboard -- --ignored --nocapture
    #[test]
    #[ignore]
    fn live_clipboard_round_trip() {
        let before = std::process::Command::new("powershell").args(["-NoProfile", "-Command", "Get-Clipboard -Raw"]).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).to_string());
        let text = "https://open.spotify.com/track/abc · ملوك الدنيا";
        assert!(set_clipboard(text));
        let read = std::process::Command::new("powershell").args(["-NoProfile", "-Command", "[Console]::OutputEncoding = [Text.Encoding]::UTF8; Get-Clipboard -Raw"]).output().unwrap();
        let got = String::from_utf8_lossy(&read.stdout).trim().to_string();
        if let Some(b) = before.filter(|b| !b.trim().is_empty()) {
            set_clipboard(b.trim_end_matches(['\r', '\n']));
        }
        println!("clipboard read back: {got}");
        assert_eq!(got, text);
    }

    #[test]
    fn jumps_stay_inside_the_song() {
        assert_eq!(seek_target(30.0, 10.0, 200.0), 40.0);
        assert_eq!(seek_target(5.0, -10.0, 200.0), 0.0, "not before the start");
        assert_eq!(seek_target(195.0, 10.0, 200.0), 199.0, "not past the last second");
        assert_eq!(seek_target(50.0, 10.0, 0.0), 60.0, "length unknown: no upper limit");
        assert_eq!(clock(83.9), "1:23");
        assert_eq!(clock(5.0), "0:05");
    }

    #[test]
    fn five_percent_a_notch_and_never_out_of_range() {
        assert_eq!(percent(volume_after(0.5, 1)), 55);
        assert_eq!(percent(volume_after(0.5, -2)), 40);
        assert_eq!(volume_after(0.98, 3), 1.0);
        assert_eq!(volume_after(0.02, -3), 0.0);
        assert_eq!(percent(VOL_STEP), 5);
    }
}
