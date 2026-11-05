//! Windows' media sessions (the list the volume flyout shows), watched through Windows' own
//! change events rather than polling: a session appearing or closing, its song changing, its
//! artwork arriving, play/pause. A slow fallback refresh catches apps that forget to announce.
//!
//! Each session is tracked as its own object (one app can have several, and an app can close its
//! session and open a new one), and every action goes to the exact session that was read, never
//! "the first one with that app's name". Every call into Windows has a time limit, so a frozen
//! player can't freeze Skip Dat.
//!
//! Stale artwork: browsers often publish a new song's title first and its picture a moment
//! later. After every song change the picture is read again a few times, and the card takes
//! whichever picture is current, so it never keeps the previous song's art.

use std::collections::HashMap;
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use windows::core::{IUnknown, Interface};
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionMediaProperties as Props,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Storage::Streams::DataReader;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use windows_future::{AsyncOperationCompletedHandler, AsyncStatus, IAsyncOperation};

use crate::art;
use crate::util::Lock;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub album: String,
    /// art id (see art.rs)
    pub art: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AppMedia {
    /// this session's own key: the app id, plus "#2", "#3"… when one app has several sessions
    pub id: String,
    /// the app id Windows gives (what decides whether it is a music app)
    pub app: String,
    pub playing: bool,
    pub now: Track,
    /// songs played before this one, oldest first
    pub history: Vec<Track>,
    pub duration: f64,
    /// the player accepts "previous" / "next" (some apps and stations don't)
    pub can_prev: bool,
    pub can_next: bool,
}

enum Sig {
    Resync,
    Props(String),
    Playback(String),
}

struct Sub {
    session: Session,
    app: String,
    ident: usize,
    tok_props: i64,
    tok_play: i64,
}

static APPS: Mutex<Vec<AppMedia>> = Mutex::new(Vec::new());
static CURRENT: Mutex<Option<String>> = Mutex::new(None);
/// the live session objects, by key, for actions
static SESSIONS: Mutex<Option<HashMap<String, Session>>> = Mutex::new(None);
static TX: OnceLock<Sender<Sig>> = OnceLock::new();
/// Sessions Skip Dat is skipping through (by key), with the song it started from: the songs only
/// passed on the way weren't listened to, so they don't go in the history.
static PASSING: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);
const HISTORY: usize = 8;
const RECHECKS: [u64; 3] = [350, 1100, 2600];
const READ_MS: u64 = 1500;
const ACT_MS: u64 = 2500;

/// Every thread that calls media functions does this once.
pub fn init_thread() {
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
}

/// Wait for a Windows async call, but never longer than `ms` (a hung player is cancelled).
fn wait<T: windows::core::RuntimeType + 'static>(op: windows::core::Result<IAsyncOperation<T>>, ms: u64) -> Option<T> {
    let op = op.ok()?;
    let (tx, rx) = channel::<()>();
    op.SetCompleted(&AsyncOperationCompletedHandler::new(move |_, _| {
        let _ = tx.send(());
        Ok(())
    }))
    .ok()?;
    if rx.recv_timeout(Duration::from_millis(ms)).is_err() {
        let _ = op.Cancel();
        return None;
    }
    if op.Status().ok()? != AsyncStatus::Completed {
        return None;
    }
    op.GetResults().ok()
}

fn manager() -> Option<Manager> {
    wait(Manager::RequestAsync(), 5000)
}

fn app_id(s: &Session) -> String {
    s.SourceAppUserModelId().map(|h| h.to_string()).unwrap_or_default()
}

/// COM identity: the same session object always gives the same number.
fn ident(s: &Session) -> usize {
    s.cast::<IUnknown>().map(|u| u.as_raw() as usize).unwrap_or(0)
}

fn is_playing(s: &Session) -> bool {
    s.GetPlaybackInfo().and_then(|i| i.PlaybackStatus()).map(|st| st == Status::Playing).unwrap_or(false)
}

/// Whether the player has a working "previous" and "next" right now (unknown counts as yes).
fn controls(s: &Session) -> (bool, bool) {
    match s.GetPlaybackInfo().and_then(|i| i.Controls()) {
        Ok(c) => (c.IsPreviousEnabled().unwrap_or(true), c.IsNextEnabled().unwrap_or(true)),
        Err(_) => (true, true),
    }
}

/// The app id Windows gives this session (for its per-app volume).
pub fn app_of(id: &str) -> Option<String> {
    APPS.guard().iter().find(|a| a.id == id).map(|a| a.app.clone())
}

/// The title playing now in this session.
pub fn title_of(id: &str) -> Option<String> {
    APPS.guard().iter().find(|a| a.id == id).map(|a| a.now.title.clone())
}

pub fn is_spotify(app: &str) -> bool {
    app.to_ascii_lowercase().contains("spotify")
}

fn props(s: &Session) -> Option<Props> {
    wait(s.TryGetMediaPropertiesAsync(), READ_MS)
}

fn text_of(p: &Props) -> (String, String, String) {
    let title = p.Title().map(|h| h.to_string()).unwrap_or_default();
    let mut artist = p.Artist().map(|h| h.to_string()).unwrap_or_default();
    if artist.is_empty() {
        artist = p.AlbumArtist().map(|h| h.to_string()).unwrap_or_default();
    }
    let album = p.AlbumTitle().map(|h| h.to_string()).unwrap_or_default();
    (title.trim().to_string(), artist.trim().to_string(), album.trim().to_string())
}

fn read_art(p: &Props) -> Option<String> {
    let r = p.Thumbnail().ok()?;
    let stream = wait(r.OpenReadAsync(), READ_MS)?;
    let size = stream.Size().ok()? as u32;
    if size == 0 || size > 8_000_000 {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream).ok()?;
    let load = reader.LoadAsync(size).ok()?.cast::<IAsyncOperation<u32>>();
    wait(load, READ_MS)?;
    let mut buf = vec![0u8; size as usize];
    reader.ReadBytes(&mut buf).ok()?;
    art::put(buf)
}

fn duration_of(s: &Session) -> f64 {
    s.GetTimelineProperties().and_then(|t| t.EndTime()).map(|ts| ts.Duration as f64 / 1e7).unwrap_or(0.0)
}

/// Seconds into the current song. Players often update the position only now and then, so the
/// time since that update is added while playing.
pub fn position_secs(s: &Session) -> f64 {
    let Ok(t) = s.GetTimelineProperties() else { return 0.0 };
    let pos = t.Position().map(|p| p.Duration as f64 / 1e7).unwrap_or(0.0);
    let updated = t.LastUpdatedTime().map(|d| d.UniversalTime).unwrap_or(0);
    if updated > 0 && is_playing(s) {
        // Windows time: 100 ns steps since 1601
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| (d.as_nanos() / 100) as i64).unwrap_or(0) + 116_444_736_000_000_000;
        let since = (now - updated) as f64 / 1e7;
        if (0.0..3600.0).contains(&since) {
            return pos + since;
        }
    }
    pos
}

pub fn snapshot() -> (Vec<AppMedia>, Option<String>) {
    (APPS.guard().clone(), CURRENT.guard().clone())
}

/// Ask the watcher to look again now (after we pressed something).
pub fn poke() {
    if let Some(tx) = TX.get() {
        let _ = tx.send(Sig::Resync);
    }
}

/// Run `f` on the session with this key (the very object the watcher reads).
pub fn with_session<R>(id: &str, f: impl FnOnce(&Session) -> R) -> Option<R> {
    let s = SESSIONS.guard().as_ref().and_then(|m| m.get(id).cloned())?;
    Some(f(&s))
}

/// While Skip Dat skips through songs in this session (`from`: the song it started on), only that
/// song goes in the history, not the ones passed on the way. `None` ends it.
pub fn passing(id: &str, from: Option<String>) {
    let mut g = PASSING.guard();
    let m = g.get_or_insert_with(HashMap::new);
    match from {
        Some(t) => {
            m.insert(id.to_string(), t);
        }
        None => {
            m.remove(id);
        }
    }
}

/// Skip forward (n > 0) or back (n < 0) one song at a time.
pub fn skip(s: &Session, n: i32) {
    for i in 0..n.unsigned_abs() {
        let op = if n > 0 { s.TrySkipNextAsync() } else { s.TrySkipPreviousAsync() };
        wait(op, ACT_MS);
        if i + 1 < n.unsigned_abs() {
            std::thread::sleep(Duration::from_millis(170));
        }
    }
}

pub fn act(s: &Session, action: &str) {
    let op = match action {
        "play" => s.TryPlayAsync(),
        "pause" => s.TryPauseAsync(),
        "next" => s.TrySkipNextAsync(),
        "previous" => s.TrySkipPreviousAsync(),
        _ => s.TryTogglePlayPauseAsync(),
    };
    wait(op, ACT_MS);
}

/// The history after moving from the current song to (title, artist): going back to a song
/// already in the history returns the list to that point, otherwise the current song is added.
pub fn advance_history<T: Clone>(history: &mut Vec<T>, current: Option<T>, is_next: impl Fn(&T) -> bool, same_as_current: impl Fn(&T) -> bool) {
    if let Some(pos) = history.iter().position(|t| is_next(t)) {
        history.truncate(pos);
        return;
    }
    if let Some(cur) = current {
        history.retain(|t| !same_as_current(t));
        history.push(cur);
        if history.len() > HISTORY {
            history.remove(0);
        }
    }
}

// ---------- the watcher ----------

fn subscribe(s: &Session, key: &str, tx: &Sender<Sig>) -> (i64, i64) {
    let (ta, tb) = (tx.clone(), tx.clone());
    let (ka, kb) = (key.to_string(), key.to_string());
    let p = s
        .MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| {
            let _ = ta.send(Sig::Props(ka.clone()));
            Ok(())
        }))
        .unwrap_or(0);
    let b = s
        .PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
            let _ = tb.send(Sig::Playback(kb.clone()));
            Ok(())
        }))
        .unwrap_or(0);
    (p, b)
}

fn unsubscribe(sub: &Sub) {
    let _ = sub.session.RemoveMediaPropertiesChanged(sub.tok_props);
    let _ = sub.session.RemovePlaybackInfoChanged(sub.tok_play);
}

fn watch_manager(mgr: &Manager, tx: &Sender<Sig>) {
    let t1 = tx.clone();
    let _ = mgr.SessionsChanged(&TypedEventHandler::new(move |_, _| {
        let _ = t1.send(Sig::Resync);
        Ok(())
    }));
    let t2 = tx.clone();
    let _ = mgr.CurrentSessionChanged(&TypedEventHandler::new(move |_, _| {
        let _ = t2.send(Sig::Resync);
        Ok(())
    }));
}

pub fn start(on_change: fn()) {
    let (tx, rx) = channel::<Sig>();
    let _ = TX.set(tx.clone());
    std::thread::spawn(move || {
        init_thread();
        let mut mgr = loop {
            if let Some(m) = manager() {
                break m;
            }
            std::thread::sleep(Duration::from_secs(5));
        };
        watch_manager(&mgr, &tx);
        let mut subs: HashMap<String, Sub> = HashMap::new();
        let mut rechecks: Vec<(Instant, String, usize)> = Vec::new();
        let mut last_full = Instant::now();
        let mut failures = 0;
        if resync(&mgr, &mut subs, &tx, &mut rechecks) == Some(true) {
            on_change();
        }
        loop {
            let next_recheck = rechecks.iter().map(|r| r.0).min();
            let wait_for = next_recheck
                .map(|t| t.saturating_duration_since(Instant::now()))
                .unwrap_or(Duration::from_secs(3))
                .min(Duration::from_secs(3));
            let mut changed = false;
            let mut full = false;
            match rx.recv_timeout(wait_for) {
                Ok(Sig::Resync) => full = true,
                Ok(Sig::Props(id)) => {
                    if let Some(sub) = subs.get(&id) {
                        changed |= refresh(&id, sub, &mut rechecks, true);
                    }
                }
                Ok(Sig::Playback(id)) => {
                    if let Some(sub) = subs.get(&id) {
                        changed |= refresh(&id, sub, &mut rechecks, false);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            // artwork re-checks that are due
            let now = Instant::now();
            let due: Vec<(Instant, String, usize)> = rechecks.iter().filter(|r| r.0 <= now).cloned().collect();
            rechecks.retain(|r| r.0 > now);
            for (_, id, n) in due {
                if let Some(sub) = subs.get(&id) {
                    changed |= recheck_art(&id, &sub.session);
                    if n + 1 < RECHECKS.len() {
                        rechecks.push((Instant::now() + Duration::from_millis(RECHECKS[n + 1] - RECHECKS[n]), id, n + 1));
                    }
                }
            }
            if full || last_full.elapsed() > Duration::from_secs(3) {
                last_full = Instant::now();
                match resync(&mgr, &mut subs, &tx, &mut rechecks) {
                    Some(c) => {
                        changed |= c;
                        failures = 0;
                    }
                    // Windows' media service stopped answering: keep what we have, and after a
                    // few tries connect to it afresh
                    None => {
                        failures += 1;
                        if failures >= 3 {
                            if let Some(m) = manager() {
                                subs.values().for_each(unsubscribe);
                                subs.clear();
                                mgr = m;
                                watch_manager(&mgr, &tx);
                                failures = 0;
                            }
                        }
                    }
                }
            }
            if changed {
                on_change();
            }
        }
    });
}

/// Match our list to Windows' sessions: follow each session object, subscribe to new ones, drop
/// closed ones, refresh all. None: Windows could not be asked (nothing is changed).
fn resync(mgr: &Manager, subs: &mut HashMap<String, Sub>, tx: &Sender<Sig>, rechecks: &mut Vec<(Instant, String, usize)>) -> Option<bool> {
    let sessions: Vec<Session> = mgr.GetSessions().ok()?.into_iter().collect();
    let mut changed = false;
    let mut seen: Vec<String> = Vec::new();
    let mut unmatched: Vec<(Session, String, usize)> = Vec::new();
    // the same objects as before
    for s in sessions {
        let app = app_id(&s);
        if app.is_empty() {
            continue;
        }
        let id = ident(&s);
        match subs.iter().find(|(k, v)| id != 0 && v.ident == id && !seen.contains(k)).map(|(k, _)| k.clone()) {
            Some(k) => seen.push(k),
            None => unmatched.push((s, app, id)),
        }
    }
    // new objects: an app's session that was closed and opened again keeps its place and history
    for (s, app, id) in unmatched {
        let reuse = subs.iter().find(|(k, v)| v.app == app && !seen.contains(k)).map(|(k, _)| k.clone());
        let key = match reuse {
            Some(k) => {
                if let Some(old) = subs.remove(&k) {
                    unsubscribe(&old);
                }
                k
            }
            None => {
                let mut k = app.clone();
                let mut n = 1;
                while subs.contains_key(&k) || seen.contains(&k) {
                    n += 1;
                    k = format!("{app}#{n}");
                }
                k
            }
        };
        let (tok_props, tok_play) = subscribe(&s, &key, tx);
        subs.insert(key.clone(), Sub { session: s, app, ident: id, tok_props, tok_play });
        seen.push(key);
        changed = true;
    }
    // closed sessions
    let gone: Vec<String> = subs.keys().filter(|k| !seen.contains(k)).cloned().collect();
    for k in gone {
        if let Some(sub) = subs.remove(&k) {
            unsubscribe(&sub);
        }
        rechecks.retain(|r| r.1 != k);
    }
    *SESSIONS.guard() = Some(subs.iter().map(|(k, v)| (k.clone(), v.session.clone())).collect());
    {
        let mut apps = APPS.guard();
        let before = apps.len();
        apps.retain(|a| seen.contains(&a.id));
        changed |= apps.len() != before;
    }
    for k in &seen {
        if let Some(sub) = subs.get(k) {
            changed |= refresh(k, sub, rechecks, true);
        }
    }
    let cur = mgr.GetCurrentSession().ok().and_then(|s| {
        let id = ident(&s);
        subs.iter().find(|(_, v)| v.ident == id).map(|(k, _)| k.clone()).or_else(|| Some(app_id(&s)))
    });
    let mut c = CURRENT.guard();
    if *c != cur {
        *c = cur;
        changed = true;
    }
    Some(changed)
}

/// Read one session. `with_props` also reads title/artist/art (needed on property events).
fn refresh(id: &str, sub: &Sub, rechecks: &mut Vec<(Instant, String, usize)>, with_props: bool) -> bool {
    let s = &sub.session;
    let playing = is_playing(s);
    let (can_prev, can_next) = controls(s);
    let p = if with_props { props(s) } else { None };
    let mut apps = APPS.guard();
    let i = match apps.iter().position(|a| a.id == id) {
        Some(i) => i,
        None => {
            apps.push(AppMedia { id: id.to_string(), app: sub.app.clone(), playing, now: Track::default(), history: Vec::new(), duration: 0.0, can_prev, can_next });
            apps.len() - 1
        }
    };
    let mut changed = false;
    if apps[i].playing != playing {
        apps[i].playing = playing;
        changed = true;
    }
    if (apps[i].can_prev, apps[i].can_next) != (can_prev, can_next) {
        apps[i].can_prev = can_prev;
        apps[i].can_next = can_next;
        changed = true;
    }
    let Some(p) = p else { return changed };
    let (title, artist, album) = text_of(&p);
    let e = &mut apps[i];
    if e.now.title != title || e.now.artist != artist {
        let mut current = if e.now.title.is_empty() { None } else { Some(e.now.clone()) };
        // a song only passed while Skip Dat skipped through: not something that was played
        if let Some(from) = PASSING.guard().as_ref().and_then(|m| m.get(id)) {
            if current.as_ref().map(|c| &c.title != from).unwrap_or(false) {
                current = None;
            }
        }
        let (t2, a2) = (title.clone(), artist.clone());
        let (ct, ca) = (e.now.title.clone(), e.now.artist.clone());
        advance_history(&mut e.history, current, |t| t.title == t2 && t.artist == a2, |t| t.title == ct && t.artist == ca);
        // the new song's details right away; its picture follows below
        e.now = Track { title: title.clone(), artist: artist.clone(), album, art: None };
        drop(apps);
        let art = read_art(&p);
        let duration = duration_of(s);
        let mut apps = APPS.guard();
        if let Some(e) = apps.iter_mut().find(|a| a.id == id) {
            if e.now.title == title && e.now.artist == artist {
                e.now.art = art;
                e.duration = duration;
            }
        }
        rechecks.retain(|r| r.1 != id);
        rechecks.push((Instant::now() + Duration::from_millis(RECHECKS[0]), id.to_string(), 0));
        return true;
    }
    if e.now.album != album {
        e.now.album = album;
        changed = true;
    }
    drop(apps);
    // same song: the picture may have changed (it often arrives after the title)
    let art = read_art(&p);
    if art.is_some() {
        let mut apps = APPS.guard();
        if let Some(e) = apps.iter_mut().find(|a| a.id == id) {
            if e.now.title == title && e.now.artist == artist && e.now.art != art {
                e.now.art = art;
                changed = true;
            }
        }
    }
    changed
}

fn recheck_art(id: &str, s: &Session) -> bool {
    let Some(p) = props(s) else { return false };
    let (title, artist, _) = text_of(&p);
    let art = read_art(&p);
    let mut apps = APPS.guard();
    if let Some(e) = apps.iter_mut().find(|a| a.id == id) {
        if e.now.title == title && e.now.artist == artist && art.is_some() && e.now.art != art {
            e.now.art = art;
            return true;
        }
    }
    false
}
