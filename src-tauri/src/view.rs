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
use crate::{art, browser, history, media, overlay, spotify, AppState};
use crate::util::Lock;

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
    /// the song's page address (tabs; to play it again from the history)
    #[serde(skip)]
    pub url: Option<String>,
    /// Spotify: the playlist or album an earlier song was played from (recently played)
    #[serde(skip)]
    pub ctx: Option<String>,
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
    /// the player has "previous" / "next" (an app or station without them can't be moved)
    pub can_prev: bool,
    pub can_next: bool,
    pub items: Vec<Item>,
    #[serde(skip)]
    pub target: Target,
    /// for tabs: the site ("ytmusic", "youtube", "soundcloud")
    #[serde(skip)]
    pub site: String,
    /// the page's shuffle is on
    #[serde(skip)]
    pub shuffle: bool,
}

/// A media source that is not shown because it is not music (Settings can show it anyway).
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Hidden {
    pub id: String,
    pub name: String,
    pub title: String,
    pub allowed: bool,
}

/// The song that was playing before the last skip or jump, shown under the play/pause button so
/// one click goes back to it (actions.rs remembers it).
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Back {
    pub source: String,
    pub title: String,
    pub artist: String,
    pub art: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Default)]
pub struct View {
    pub active: String,
    pub sources: Vec<Source>,
    pub hidden: Vec<Hidden>,
    pub back: Option<Back>,
}

// ---------- optional Spotify queue (Spotify Web API) ----------

#[derive(Clone)]
struct SpotItem {
    rel: i32,
    title: String,
    artist: String,
    art_url: Option<String>,
    uri: Option<String>,
    ctx: Option<String>,
}

static SPOT: Mutex<Option<(String, Vec<SpotItem>)>> = Mutex::new(None);
static SPOT_AT: Mutex<Option<Instant>> = Mutex::new(None);
/// Every Spotify song seen playing or queued: its URI and the playlist or album it was in, by
/// title, so the songs that played before can be played again straight away.
static SPOT_SEEN: Mutex<Option<HashMap<String, (String, Option<String>, Option<String>)>>> = Mutex::new(None);

fn seen_key(title: &str) -> String {
    title.trim().to_lowercase()
}

/// (URI, playlist or album) of a Spotify song seen before, preferring the same artist.
fn spot_seen(title: &str, artist: &str) -> (Option<String>, Option<String>) {
    let g = SPOT_SEEN.guard();
    match g.as_ref().and_then(|m| m.get(&seen_key(title))) {
        Some((a, uri, ctx)) if artist.is_empty() || a.is_empty() || a.contains(artist) || artist.contains(a.as_str()) => (uri.clone(), ctx.clone()),
        _ => (None, None),
    }
}

/// Reads the Spotify queue when a Spotify developer app is connected (never more than every 2 s).
pub fn refresh_spotify(app: &AppHandle, force: bool) {
    if !spotify::connected(app) {
        *SPOT.guard() = None;
        return;
    }
    {
        let mut at = SPOT_AT.guard();
        if !force && at.map(|t| t.elapsed() < Duration::from_secs(2)).unwrap_or(false) {
            return;
        }
        *at = Some(Instant::now());
    }
    let Ok(q) = spotify::get(app, "/v1/me/player/queue") else { return };
    let Some((now_title, artist, art_url, uri)) = spotify::describe(&q["currently_playing"]) else {
        *SPOT.guard() = None;
        changed();
        return;
    };
    // the playlist or album playing, so its songs can be started inside it later
    let ctx = spotify::context(app);
    let mut items = Vec::new();
    // Spotify's "recently played" only counts songs heard for 30 s or more, so the songs before
    // this one come from what Skip Dat saw play (smtc_source); this is only for a fresh start
    let fresh = media::snapshot().0.iter().any(|a| media::is_spotify(&a.app) && a.history.is_empty());
    if let Some(rp) = fresh.then(|| spotify::get(app, "/v1/me/player/recently-played?limit=5").ok()).flatten() {
        if let Some(arr) = rp["items"].as_array() {
            let mut k = 0;
            for it in arr {
                if let Some((t, a, art, u)) = spotify::describe(&it["track"]) {
                    // recently-played can include the current song once it has played a while
                    if k == 0 && t == now_title {
                        continue;
                    }
                    k += 1;
                    let ctx = it["context"]["uri"].as_str().map(String::from);
                    items.push(SpotItem { rel: -k, title: t, artist: a, art_url: art, uri: u, ctx });
                    if k == 3 {
                        break;
                    }
                }
            }
        }
    }
    items.reverse();
    items.push(SpotItem { rel: 0, title: now_title.clone(), artist, art_url, uri, ctx: ctx.clone() });
    if let Some(arr) = q["queue"].as_array() {
        for (k, it) in arr.iter().enumerate() {
            if let Some((t, a, art, u)) = spotify::describe(it) {
                items.push(SpotItem { rel: k as i32 + 1, title: t, artist: a, art_url: art, uri: u, ctx: ctx.clone() });
            }
        }
    }
    {
        let mut g = SPOT_SEEN.guard();
        let m = g.get_or_insert_with(HashMap::new);
        if m.len() > 2000 {
            m.clear();
        }
        for it in &items {
            if it.uri.is_some() {
                m.insert(seen_key(&it.title), (it.artist.clone(), it.uri.clone(), it.ctx.clone()));
            }
        }
    }
    *SPOT.guard() = Some((now_title, items));
    changed();
}

// ---------- building the view ----------

/// The source the person last played something from; the overlay opens on it while it plays.
static CHOSEN: Mutex<Option<String>> = Mutex::new(None);

pub fn set_chosen(key: &str) {
    *CHOSEN.guard() = Some(key.to_string());
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
        .map(|(i, (t, a, art))| Item { key: item_key(t, a, seen), rel: i as i32 - n as i32, title: t.clone(), artist: a.clone(), art: art.clone(), jump: None, uri: None, url: None, ctx: None })
        .collect()
}

fn smtc_source(app: &AppHandle, a: &media::AppMedia, name: &str, via: &str) -> Source {
    let mut seen = HashMap::new();
    let mut items = Vec::new();
    let mut queue = "none";
    // the full Spotify queue, but only when it describes the same song Windows says is playing
    if media::is_spotify(&a.app) && spotify::connected(app) {
        if let Some((now, list)) = SPOT.guard().clone() {
            if classify::same_title(&now, &a.now.title) {
                // the songs before this one: what really played here, in order (not Spotify's
                // "recently played", which skips songs heard for less than 30 s)
                if !a.history.is_empty() {
                    let n = a.history.len().min(5);
                    for (i, t) in a.history[a.history.len() - n..].iter().enumerate() {
                        let (uri, ctx) = spot_seen(&t.title, &t.artist);
                        items.push(Item { key: item_key(&t.title, &t.artist, &mut seen), rel: i as i32 - n as i32, title: t.title.clone(), artist: t.artist.clone(), art: t.art.clone(), jump: None, uri, url: None, ctx });
                    }
                }
                let have_before = !items.is_empty();
                for s in list {
                    if s.rel < 0 && have_before {
                        continue;
                    }
                    let art = if s.rel == 0 && a.now.art.is_some() { a.now.art.clone() } else { art_url(&s.art_url) };
                    items.push(Item { key: item_key(&s.title, &s.artist, &mut seen), rel: s.rel, title: s.title, artist: s.artist, art, jump: None, uri: s.uri, url: None, ctx: s.ctx });
                }
                queue = "full";
            }
        }
    }
    if items.is_empty() {
        let hist: Vec<(String, String, Option<String>)> = a.history.iter().map(|t| (t.title.clone(), t.artist.clone(), t.art.clone())).collect();
        items = history_items(&hist, &mut seen);
        items.push(Item { key: item_key(&a.now.title, &a.now.artist, &mut seen), rel: 0, title: a.now.title.clone(), artist: a.now.artist.clone(), art: a.now.art.clone(), jump: None, uri: None, url: None, ctx: None });
    }
    Source {
        key: format!("app:{}", a.id),
        name: name.to_string(),
        via: via.to_string(),
        playing: a.playing,
        queue,
        can_jump: false,
        can_prev: a.can_prev,
        can_next: a.can_next,
        items,
        target: Target::Smtc(a.id.clone()),
        site: String::new(),
        shuffle: false,
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
        url: r.url.clone(),
        ctx: None,
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
                url: None,
                ctx: None,
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
                url: None,
                ctx: None,
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
        can_prev: true,
        can_next: true,
        items,
        target: Target::Tab(t.key.clone()),
        site: r.site.clone(),
        shuffle: r.shuffle,
    }
}

pub fn build(app: &AppHandle) -> View {
    let allow = app.state::<AppState>().cfg.guard().allow.clone();
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
    let back = crate::actions::back_view(&sources);
    View { active, sources, hidden, back }
}

fn pick_active(sources: &[Source], current: Option<&str>) -> String {
    let chosen = CHOSEN.guard().clone();
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
    if let Some(v) = LATEST.guard().clone() {
        return v;
    }
    let v = build(app);
    *LATEST.guard() = Some(v.clone());
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
    let _ = APP.set(app.clone());
    std::thread::spawn(move || {
        let mut last_spotify = Instant::now();
        let mut listener = history::Listener::default();
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
            note_history(&mut listener, &v);
            let differs = LATEST.guard().as_ref() != Some(&v);
            if differs {
                *LATEST.guard() = Some(v.clone());
                emit(&app, &v);
            }
            // the optional Spotify queue: every 20 s, more often while the overlay is open
            let every = if overlay::phase() == overlay::OPEN { 4 } else { 20 };
            // …and as soon as Spotify moves to another song, so its list never lags behind
            let moved_on = v.sources.iter().any(|s| s.key.to_lowercase().contains("spotify") && s.queue == "none")
                && SPOT.guard().as_ref().map(|(now, _)| !now.is_empty()).unwrap_or(false);
            if (moved_on && last_spotify.elapsed() > Duration::from_secs(2) || last_spotify.elapsed() > Duration::from_secs(every)) && spotify::connected(&app) {
                last_spotify = Instant::now();
                let app2 = app.clone();
                std::thread::spawn(move || refresh_spotify(&app2, false));
            }
        }
    });
}

/// Songs that play long enough go into the play history.
fn note_history(listener: &mut history::Listener, v: &View) {
    let now = Instant::now();
    for s in &v.sources {
        if !s.playing {
            listener.paused(&s.key);
            continue;
        }
        let Some(it) = s.items.iter().find(|i| i.rel == 0) else { continue };
        let e = history::Entry {
            title: it.title.clone(),
            artist: it.artist.clone(),
            app: s.name.clone(),
            via: s.via.clone(),
            art: it.art.clone(),
            url: it.url.clone(),
            uri: it.uri.clone(),
            source: s.key.clone(),
            ..Default::default()
        };
        if let Some(done) = listener.saw(&s.key, e, now) {
            if history::record(done) {
                if let Some(app) = APP.get() {
                    let _ = app.emit("lg-history", ());
                }
            }
        }
    }
}

static APP: OnceLock<AppHandle> = OnceLock::new();

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
