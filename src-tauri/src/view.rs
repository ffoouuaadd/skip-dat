//! What the overlay shows: one source per music app or music tab, each with only the songs it
//! can genuinely tell us about. Nothing is invented: a source that does not share its queue shows
//! the songs it played and the song playing now, and says so.
//!
//! Sources come from two places:
//! - Windows media sessions (media.rs): Spotify, Apple Music, other music apps, and browsers
//!   without the companion extension.
//! - The Skip Dat browser companion (browser.rs): YouTube Music, YouTube and SoundCloud tabs with
//!   their real queue, site and category.
//! A browser's own media session is dropped when a companion tab already describes it.
//!
//! Updates are pushed to the windows only when something visible changed, at most every 30 ms.

use std::collections::HashMap;
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::classify::{self, AppKind, Meta};
use crate::{art, browser, media, overlay, spotify, AppState};

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Item {
    /// stable while the song stays in the list (the overlay reuses its card)
    pub key: String,
    /// distance from the song playing now (0)
    pub rel: i32,
    pub title: String,
    pub artist: String,
    /// art id, served as skipart://localhost/<id>; None when the source has no picture (yet)
    pub art: Option<String>,
    /// position in the page's own queue, for jumping straight to the song
    #[serde(skip)]
    pub jump: Option<i64>,
    /// Spotify track URI (only with the optional Spotify connection)
    #[serde(skip)]
    pub uri: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// a Windows media session, by app id
    Smtc(String),
    /// a browser tab reached through the companion
    Tab(String),
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Source {
    pub key: String,
    pub name: String,
    /// the browser, for music playing in one ("Chrome", "Edge"…)
    pub via: String,
    pub playing: bool,
    /// "full": the real upcoming queue; "next": only the next song; "none": not shared by the app
    pub queue: &'static str,
    /// a song can be played straight from the list (not only by skipping)
    pub can_jump: bool,
    pub items: Vec<Item>,
    #[serde(skip)]
    pub target: Target,
}

/// A media source that is not shown because it is not music (Settings can show it anyway).
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Hidden {
    pub id: String,
    pub name: String,
    pub title: String,
    pub allowed: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Default)]
pub struct View {
    pub active: String,
    pub sources: Vec<Source>,
    pub hidden: Vec<Hidden>,
}

// ---------- optional Spotify queue (Spotify Web API) ----------

#[derive(Clone)]
struct SpotItem {
    rel: i32,
    title: String,
    artist: String,
    art_url: Option<String>,
    uri: Option<String>,
}

static SPOT: Mutex<Option<(String, Vec<SpotItem>)>> = Mutex::new(None);
static SPOT_AT: Mutex<Option<Instant>> = Mutex::new(None);

/// Reads the Spotify queue when a Spotify developer app is connected (never more than every 2 s).
pub fn refresh_spotify(app: &AppHandle, force: bool) {
    if !spotify::connected(app) {
        *SPOT.lock().unwrap() = None;
        return;
    }
    {
        let mut at = SPOT_AT.lock().unwrap();
        if !force && at.map(|t| t.elapsed() < Duration::from_secs(2)).unwrap_or(false) {
            return;
        }
        *at = Some(Instant::now());
    }
    let Ok(q) = spotify::get(app, "/v1/me/player/queue") else { return };
    let Some((now_title, artist, art_url, uri)) = spotify::describe(&q["currently_playing"]) else {
        *SPOT.lock().unwrap() = None;
        changed();
        return;
    };
    let mut items = Vec::new();
    if let Ok(rp) = spotify::get(app, "/v1/me/player/recently-played?limit=5") {
        if let Some(arr) = rp["items"].as_array() {
            let mut k = 0;
            for it in arr {
                if let Some((t, a, art, u)) = spotify::describe(&it["track"]) {
                    // recently-played can include the current song once it has played a while
                    if k == 0 && t == now_title {
                        continue;
                    }
                    k += 1;
                    items.push(SpotItem { rel: -k, title: t, artist: a, art_url: art, uri: u });
                    if k == 3 {
                        break;
                    }
                }
            }
        }
    }
    items.reverse();
    items.push(SpotItem { rel: 0, title: now_title.clone(), artist, art_url, uri });
    if let Some(arr) = q["queue"].as_array() {
        for (k, it) in arr.iter().enumerate() {
            if let Some((t, a, art, u)) = spotify::describe(it) {
                items.push(SpotItem { rel: k as i32 + 1, title: t, artist: a, art_url: art, uri: u });
            }
        }
    }
    *SPOT.lock().unwrap() = Some((now_title, items));
    changed();
}

// ---------- building the view ----------

/// The source the person last played something from; the overlay opens on it while it plays.
static CHOSEN: Mutex<Option<String>> = Mutex::new(None);

pub fn set_chosen(key: &str) {
    *CHOSEN.lock().unwrap() = Some(key.to_string());
}

fn item_key(title: &str, artist: &str, seen: &mut HashMap<u64, u32>) -> String {
    let h = art::fnv1a(format!("{title}\u{1f}{artist}").as_bytes());
    let n = seen.entry(h).or_insert(0);
    *n += 1;
    if *n == 1 { format!("{h:016x}") } else { format!("{h:016x}-{n}") }
}

fn art_url(url: &Option<String>) -> Option<String> {
    url.as_deref().filter(|u| u.starts_with("https://")).and_then(|u| art::for_url(u, changed))
}

fn history_items(history: &[(String, String, Option<String>)], seen: &mut HashMap<u64, u32>) -> Vec<Item> {
    let n = history.len().min(5);
    history[history.len() - n..]
        .iter()
        .enumerate()
        .map(|(i, (t, a, art))| Item { key: item_key(t, a, seen), rel: i as i32 - n as i32, title: t.clone(), artist: a.clone(), art: art.clone(), jump: None, uri: None })
        .collect()
}

fn smtc_source(app: &AppHandle, a: &media::AppMedia, name: &str, via: &str) -> Source {
    let mut seen = HashMap::new();
    let mut items = Vec::new();
    let mut queue = "none";
    // the full Spotify queue, but only when it describes the same song Windows says is playing
    if media::is_spotify(&a.app) && spotify::connected(app) {
        if let Some((now, list)) = SPOT.lock().unwrap().clone() {
            if classify::same_title(&now, &a.now.title) {
                for s in list {
                    let art = if s.rel == 0 && a.now.art.is_some() { a.now.art.clone() } else { art_url(&s.art_url) };
                    items.push(Item { key: item_key(&s.title, &s.artist, &mut seen), rel: s.rel, title: s.title, artist: s.artist, art, jump: None, uri: s.uri });
                }
                queue = "full";
            }
        }
    }
    if items.is_empty() {
        let hist: Vec<(String, String, Option<String>)> = a.history.iter().map(|t| (t.title.clone(), t.artist.clone(), t.art.clone())).collect();
        items = history_items(&hist, &mut seen);
        items.push(Item { key: item_key(&a.now.title, &a.now.artist, &mut seen), rel: 0, title: a.now.title.clone(), artist: a.now.artist.clone(), art: a.now.art.clone(), jump: None, uri: None });
    }
    Source {
        key: format!("app:{}", a.id),
        name: name.to_string(),
        via: via.to_string(),
        playing: a.playing,
        queue,
        can_jump: false,
        items,
        target: Target::Smtc(a.id.clone()),
    }
}

/// The browser's own media session that shows the same song as this companion tab.
fn same_browser(app: &str, tab_browser: &str) -> bool {
    match classify::app_kind(app) {
        // Arc and other Chromium browsers report themselves to the companion as Chrome
        AppKind::Browser(b) => b == tab_browser || (tab_browser == "Chrome" && !["Edge", "Opera", "Brave", "Vivaldi", "Firefox"].contains(&b)),
        _ => false,
    }
}

fn tab_source(t: &browser::Tab, apps: &[media::AppMedia]) -> Source {
    let r = &t.report;
    let mut seen = HashMap::new();
    // the browser's own media session for this tab has the current picture as bytes already
    let smtc_art = apps
        .iter()
        .find(|a| same_browser(&a.app, &r.browser) && classify::same_title(&a.now.title, &r.title))
        .and_then(|a| a.now.art.clone());
    let now_art = art_url(&r.art).or(smtc_art);
    let now = |seen: &mut HashMap<u64, u32>, jump: Option<i64>| Item {
        key: item_key(&r.title, &r.artist, seen),
        rel: 0,
        title: r.title.clone(),
        artist: r.artist.clone(),
        art: now_art.clone(),
        jump,
        uri: None,
    };
    let mut items = Vec::new();
    let len = r.queue.len() as i64;
    let queue;
    if len > 0 && r.index >= 0 && r.index < len {
        // the page's own list with the current song in it (YouTube Music, playlists, SoundCloud).
        // Keys count repeats from the top of the whole list, so they stay put as the list plays.
        let keys: Vec<String> = r.queue.iter().map(|q| item_key(&q.title, &q.artist, &mut seen)).collect();
        let lo = (r.index - 60).max(0);
        let hi = (r.index + 300).min(len - 1);
        for i in lo..=hi {
            if i == r.index {
                items.push(Item { key: keys[i as usize].clone(), ..now(&mut HashMap::new(), Some(i)) });
                continue;
            }
            let q = &r.queue[i as usize];
            items.push(Item {
                key: keys[i as usize].clone(),
                rel: (i - r.index) as i32,
                title: q.title.clone(),
                artist: q.artist.clone(),
                art: art_url(&q.art),
                jump: Some(i),
                uri: None,
            });
        }
        queue = "full";
    } else {
        // the current song is not part of a list: what it played before, now, and what the page
        // says comes next (YouTube's autoplay)
        let hist: Vec<(String, String, Option<String>)> = t.history.iter().map(|(a, b, c)| (a.clone(), b.clone(), art_url(c))).collect();
        items = history_items(&hist, &mut seen);
        items.push(now(&mut seen, None));
        for (i, q) in r.queue.iter().take(300).enumerate() {
            items.push(Item {
                key: item_key(&q.title, &q.artist, &mut seen),
                rel: i as i32 + 1,
                title: q.title.clone(),
                artist: q.artist.clone(),
                art: art_url(&q.art),
                jump: Some(i as i64),
                uri: None,
            });
        }
        queue = match len {
            0 => "none",
            1 => "next",
            _ => "full",
        };
    }
    Source {
        key: format!("tab:{}", t.key),
        name: classify::web_site_name(&r.site).to_string(),
        via: r.browser.clone(),
        playing: r.playing,
        queue,
        can_jump: r.can_jump,
        items,
        target: Target::Tab(t.key.clone()),
    }
}

pub fn build(app: &AppHandle) -> View {
    let allow = app.state::<AppState>().cfg.lock().unwrap().allow.clone();
    let (apps, current) = media::snapshot();
    let tabs = browser::tabs();
    let mut sources = Vec::new();
    let mut hidden = Vec::new();

    for t in &tabs {
        let r = &t.report;
        if r.title.is_empty() {
            continue;
        }
        let id = format!("web:{}", r.site);
        let allowed = allow.contains(&id);
        // YouTube's own category says "Music"; if the page could not tell us, judge by the title
        let music = classify::web_is_music(&r.site, &r.category)
            || (r.site == "youtube"
                && r.category.is_empty()
                && classify::browser_is_music(&Meta { title: &r.title, artist: &r.artist, album: "", duration: 0.0 }).is_some());
        if !music {
            hidden.push(Hidden { id, name: format!("{} videos", classify::web_site_name(&r.site)), title: r.title.clone(), allowed });
            if !allowed {
                continue;
            }
        }
        sources.push(tab_source(t, &apps));
    }

    for a in &apps {
        if a.now.title.is_empty() && !a.playing {
            continue;
        }
        let allowed = allow.contains(&a.app);
        match classify::app_kind(&a.app) {
            AppKind::Music(name) => sources.push(smtc_source(app, a, name, "")),
            AppKind::Browser(bname) => {
                // a companion tab in this browser already shows this, with its real queue
                if tabs.iter().any(|t| same_browser(&a.app, &t.report.browser) && classify::same_title(&t.report.title, &a.now.title)) {
                    continue;
                }
                let meta = Meta { title: &a.now.title, artist: &a.now.artist, album: &a.now.album, duration: a.duration };
                match classify::browser_is_music(&meta) {
                    Some(label) => sources.push(smtc_source(app, a, label, bname)),
                    None => {
                        hidden.push(Hidden { id: a.app.clone(), name: format!("{bname} (other tabs)"), title: a.now.title.clone(), allowed });
                        if allowed {
                            sources.push(smtc_source(app, a, bname, bname));
                        }
                    }
                }
            }
            AppKind::Other => {
                hidden.push(Hidden { id: a.app.clone(), name: classify::app_name(&a.app), title: a.now.title.clone(), allowed });
                if allowed {
                    sources.push(smtc_source(app, a, &classify::app_name(&a.app), ""));
                }
            }
        }
    }

    // a steady order, so the switcher does not reshuffle; two tabs of one site get numbers
    sources.sort_by(|a, b| a.name.cmp(&b.name).then(a.key.cmp(&b.key)));
    let mut count: HashMap<String, u32> = HashMap::new();
    for s in sources.iter_mut() {
        let n = count.entry(s.name.clone()).or_insert(0);
        *n += 1;
        if *n > 1 {
            s.name = format!("{} {}", s.name, n);
        }
    }
    hidden.sort_by(|a, b| a.name.cmp(&b.name));
    hidden.dedup_by(|a, b| a.id == b.id);

    let active = pick_active(&sources, current.as_deref());
    View { active, sources, hidden }
}

fn pick_active(sources: &[Source], current: Option<&str>) -> String {
    let chosen = CHOSEN.lock().unwrap().clone();
    let is_current = |s: &Source| matches!((&s.target, current), (Target::Smtc(id), Some(c)) if id == c);
    let playing: Vec<&Source> = sources.iter().filter(|s| s.playing).collect();
    if let Some(c) = &chosen {
        if playing.iter().any(|s| &s.key == c) {
            return c.clone();
        }
    }
    if let Some(s) = playing.iter().find(|s| is_current(s)) {
        return s.key.clone();
    }
    if let Some(s) = playing.first() {
        return s.key.clone();
    }
    if let Some(c) = chosen {
        if sources.iter().any(|s| s.key == c) {
            return c;
        }
    }
    if let Some(s) = sources.iter().find(|s| is_current(s)) {
        return s.key.clone();
    }
    sources.first().map(|s| s.key.clone()).unwrap_or_else(|| "none".into())
}

// ---------- pushing updates ----------

static TX: OnceLock<Sender<()>> = OnceLock::new();
static LATEST: Mutex<Option<View>> = Mutex::new(None);

/// Something may have changed: rebuild soon (many calls in a burst become one rebuild).
pub fn changed() {
    if let Some(tx) = TX.get() {
        let _ = tx.send(());
    }
}

/// The view as last built (built now if there is none yet).
pub fn latest(app: &AppHandle) -> View {
    if let Some(v) = LATEST.lock().unwrap().clone() {
        return v;
    }
    let v = build(app);
    *LATEST.lock().unwrap() = Some(v.clone());
    v
}

fn emit(app: &AppHandle, v: &View) {
    let _ = app.emit_to("overlay", "lg-media", v);
    if app.get_webview_window("settings").is_some() {
        let _ = app.emit_to("settings", "lg-media", v);
    }
}

pub fn start(app: AppHandle) {
    let (tx, rx) = channel::<()>();
    let _ = TX.set(tx);
    std::thread::spawn(move || {
        let mut last_spotify = Instant::now();
        loop {
            match rx.recv_timeout(Duration::from_secs(5)) {
                Ok(()) => {
                    // gather a burst of changes into one update
                    let until = Instant::now() + Duration::from_millis(30);
                    while let Some(left) = until.checked_duration_since(Instant::now()) {
                        if rx.recv_timeout(left).is_err() {
                            break;
                        }
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            let v = build(&app);
            let differs = LATEST.lock().unwrap().as_ref() != Some(&v);
            if differs {
                *LATEST.lock().unwrap() = Some(v.clone());
                emit(&app, &v);
            }
            // the optional Spotify queue: every 20 s, more often while the overlay is open
            let every = if overlay::phase() == overlay::OPEN { 4 } else { 20 };
            if last_spotify.elapsed() > Duration::from_secs(every) && spotify::connected(&app) {
                last_spotify = Instant::now();
                let app2 = app.clone();
                std::thread::spawn(move || refresh_spotify(&app2, false));
            }
        }
    });
}

/// The overlay is opening: give it what we know right away, then look for anything newer.
pub fn on_summon(app: &AppHandle) {
    let v = latest(app);
    let _ = app.emit_to("overlay", "lg-media", &v);
    media::poke();
    changed();
    if spotify::connected(app) {
        let app2 = app.clone();
        std::thread::spawn(move || refresh_spotify(&app2, false));
    }
}
