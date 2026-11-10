//! The Skip Dat browser companion talks to the app here (127.0.0.1 only).
//!
//! Each YouTube / YouTube Music / SoundCloud tab reports what it plays, its real queue and the
//! site category whenever something changes (and every few seconds as a heartbeat). Commands for
//! a tab (jump to a queue item, next, previous, play/pause) are delivered through a long-poll
//! that each tab keeps open, so they arrive at once even in a background tab that the browser
//! throttles. Requests from web pages are refused: only browser extensions
//! (chrome-extension:// or moz-extension:// origins) are answered.

use std::collections::HashMap;
use std::io::Read;
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::{json, Value};
use crate::util::Lock;

pub const PORT: u16 = 47231;
/// A paused tab in the background may only be heard from about once a minute (browsers throttle
/// hidden tabs), so tabs count as present for a while; closing a tab or leaving the site removes
/// it at once.
const FRESH: Duration = Duration::from_secs(90);
const WAIT: Duration = Duration::from_secs(20);

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QItem {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub art: Option<String>,
}

fn neg() -> i64 {
    -1
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub browser: String,
    pub tab_id: i64,
    pub site: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub album: String,
    #[serde(default)]
    pub art: Option<String>,
    #[serde(default)]
    pub playing: bool,
    #[serde(default)]
    pub has_media: bool,
    /// the whole queue the page shows, in order; `index` is the song playing now (-1: not in it)
    #[serde(default)]
    pub queue: Vec<QItem>,
    #[serde(default = "neg")]
    pub index: i64,
    #[serde(default)]
    pub can_jump: bool,
    /// the page address of the song playing (to play it again from the history)
    #[serde(default)]
    pub url: Option<String>,
    /// the page's shuffle is on
    #[serde(default)]
    pub shuffle: bool,
    /// the tab's own player volume, 0–100 (None: the page can't say, e.g. SoundCloud)
    #[serde(default)]
    pub volume: Option<f64>,
    /// the song playing is liked on the site (its like button is pressed); None: no button
    #[serde(default)]
    pub liked: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct Tab {
    pub key: String,
    pub report: Report,
    pub seen: Instant,
    /// songs this tab played before the current one, oldest first (title, artist, art url)
    pub history: Vec<(String, String, Option<String>)>,
}

#[derive(Default)]
struct State {
    tabs: HashMap<String, Tab>,
    commands: HashMap<String, Vec<Value>>,
    /// the newest long-poll per tab; an older one (from before a page reload) gives way
    waiters: HashMap<String, u64>,
    last_contact: Option<Instant>,
    error: Option<String>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);
static CV: Condvar = Condvar::new();

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut g = STATE.guard();
    f(g.get_or_insert_with(State::default))
}

pub fn start(on_change: fn()) {
    std::thread::spawn(move || match tiny_http::Server::http(("127.0.0.1", PORT)) {
        Ok(server) => {
            for req in server.incoming_requests() {
                handle(req, on_change);
            }
        }
        Err(e) => with(|s| s.error = Some(format!("Port {PORT} is in use ({e}). Close the program using it and restart Skip Dat."))),
    });
}

fn reply(req: tiny_http::Request, code: u16, body: Value) {
    let data = body.to_string();
    let resp = tiny_http::Response::from_string(data)
        .with_status_code(code)
        .with_header("Content-Type: application/json".parse::<tiny_http::Header>().unwrap());
    let _ = req.respond(resp);
}

fn handle(mut req: tiny_http::Request, on_change: fn()) {
    let origin = req
        .headers()
        .iter()
        .find(|h| h.field.equiv("Origin"))
        .map(|h| h.value.as_str().to_string())
        .unwrap_or_default();
    let from_extension = ["chrome-extension://", "moz-extension://", "extension://"].iter().any(|p| origin.starts_with(p));
    if !from_extension {
        return reply(req, 403, json!({ "error": "only the Skip Dat browser companion may connect" }));
    }
    let method = req.method().clone();
    let url = req.url().split('?').next().unwrap_or("").to_string();
    let mut body = String::new();
    let _ = req.as_reader().take(2_000_000).read_to_string(&mut body);
    match (method, url.as_str()) {
        (tiny_http::Method::Post, "/ext/state") => match serde_json::from_str::<Report>(&body) {
            Ok(r) => {
                let (cmds, changed) = upsert(r);
                if changed {
                    on_change();
                }
                reply(req, 200, json!({ "commands": cmds }))
            }
            Err(e) => reply(req, 400, json!({ "error": e.to_string() })),
        },
        (tiny_http::Method::Post, "/ext/wait") => {
            let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
            let key = format!("{}:{}", v["browser"].as_str().unwrap_or(""), v["tabId"].as_i64().unwrap_or(-1));
            let nonce = v["nonce"].as_u64().unwrap_or(0);
            // each waiting tab gets its own thread; there are only ever a handful
            std::thread::spawn(move || {
                let cmds = wait_commands(&key, nonce, WAIT);
                reply(req, 200, json!({ "commands": cmds }))
            });
        }
        (tiny_http::Method::Post, "/ext/closed") => {
            let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
            let key = format!("{}:{}", v["browser"].as_str().unwrap_or(""), v["tabId"].as_i64().unwrap_or(-1));
            let removed = with(|s| {
                s.commands.remove(&key);
                s.waiters.remove(&key);
                s.tabs.remove(&key).is_some()
            });
            CV.notify_all();
            if removed {
                on_change();
            }
            reply(req, 200, json!({}))
        }
        (tiny_http::Method::Get, "/ext/ping") => {
            with(|s| s.last_contact = Some(Instant::now()));
            reply(req, 200, json!({ "app": "Skip Dat", "version": env!("CARGO_PKG_VERSION") }))
        }
        _ => reply(req, 404, json!({ "error": "not found" })),
    }
}

/// Store a tab's report; returns the commands waiting for it and whether anything visible changed.
pub fn upsert(r: Report) -> (Vec<Value>, bool) {
    let key = format!("{}:{}", r.browser, r.tab_id);
    with(|s| {
        s.last_contact = Some(Instant::now());
        let cmds = s.commands.remove(&key).unwrap_or_default();
        let changed = match s.tabs.get_mut(&key) {
            Some(t) => {
                let was_stale = t.seen.elapsed() > FRESH;
                let changed = t.report != r || was_stale;
                if !t.report.title.is_empty() && (t.report.title != r.title || t.report.artist != r.artist) {
                    if let Some(pos) = t.history.iter().position(|h| h.0 == r.title && h.1 == r.artist) {
                        // went back to a song played before: the list returns to that point
                        t.history.truncate(pos);
                    } else {
                        let prev = (t.report.title.clone(), t.report.artist.clone(), t.report.art.clone());
                        t.history.retain(|h| !(h.0 == prev.0 && h.1 == prev.1));
                        t.history.push(prev);
                        if t.history.len() > 8 {
                            t.history.remove(0);
                        }
                    }
                }
                t.report = r;
                t.seen = Instant::now();
                changed
            }
            None => {
                s.tabs.insert(key.clone(), Tab { key: key.clone(), report: r, seen: Instant::now(), history: Vec::new() });
                true
            }
        };
        s.tabs.retain(|_, t| t.seen.elapsed() < FRESH * 2);
        (cmds, changed)
    })
}

/// Hold a tab's long-poll until a command for it arrives (or `max` passes).
fn wait_commands(key: &str, nonce: u64, max: Duration) -> Vec<Value> {
    let deadline = Instant::now() + max;
    let mut g = STATE.guard();
    {
        let s = g.get_or_insert_with(State::default);
        s.waiters.insert(key.to_string(), nonce);
    }
    CV.notify_all();
    loop {
        let s = g.get_or_insert_with(State::default);
        s.last_contact = Some(Instant::now());
        if let Some(t) = s.tabs.get_mut(key) {
            t.seen = Instant::now(); // a tab that is waiting is alive
        }
        if s.waiters.get(key) != Some(&nonce) {
            return Vec::new();
        }
        if let Some(c) = s.commands.remove(key) {
            if !c.is_empty() {
                return c;
            }
        }
        let now = Instant::now();
        if now >= deadline {
            return Vec::new();
        }
        g = CV.wait_timeout(g, deadline - now).unwrap_or_else(std::sync::PoisonError::into_inner).0;
    }
}

/// Tabs heard from recently that are playing or paused media.
pub fn tabs() -> Vec<Tab> {
    with(|s| s.tabs.values().filter(|t| t.seen.elapsed() < FRESH && t.report.has_media).cloned().collect())
}

pub fn push(key: &str, cmd: Value) {
    push_many(key, vec![cmd]);
}

/// Several commands that belong together (skips then play) reach the tab in one delivery.
pub fn push_many(key: &str, cmds: Vec<Value>) {
    with(|s| {
        let q = s.commands.entry(key.to_string()).or_default();
        q.extend(cmds);
        while q.len() > 40 {
            q.remove(0);
        }
    });
    CV.notify_all();
}

/// For Settings: seconds since the companion last reported, and any server error.
pub fn status() -> (Option<u64>, Option<String>) {
    with(|s| (s.last_contact.map(|t| t.elapsed().as_secs()), s.error.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(title: &str) -> Report {
        Report { browser: "chrome".into(), tab_id: 7, site: "ytmusic".into(), title: title.into(), artist: "A".into(), has_media: true, index: 0, ..Default::default() }
    }

    #[test]
    fn history_and_commands() {
        let (_, c1) = upsert(report("One"));
        assert!(c1);
        let (_, c2) = upsert(report("One"));
        assert!(!c2, "same report is not a change");
        push("chrome:7", json!({"type": "next"}));
        let (cmds, c3) = upsert(report("Two"));
        assert!(c3);
        assert_eq!(cmds.len(), 1);
        let t = tabs().into_iter().find(|t| t.key == "chrome:7").unwrap();
        assert_eq!(t.history.len(), 1);
        assert_eq!(t.history[0].0, "One");
        let (cmds2, _) = upsert(report("Two"));
        assert!(cmds2.is_empty(), "commands are delivered once");
    }

    #[test]
    fn going_back_cuts_history() {
        let r = |t: &str| Report { tab_id: 21, ..report(t) };
        for t in ["A", "B", "C", "D"] {
            upsert(r(t));
        }
        upsert(r("B"));
        let t = tabs().into_iter().find(|t| t.key == "chrome:21").unwrap();
        let names: Vec<&str> = t.history.iter().map(|h| h.0.as_str()).collect();
        assert_eq!(names, ["A"], "after going back to B, only A is before it");
    }

    #[test]
    fn long_poll_delivers_and_newer_poll_wins() {
        upsert(Report { tab_id: 9, ..report("Song") });
        let t = std::thread::spawn(|| wait_commands("chrome:9", 1, Duration::from_secs(5)));
        std::thread::sleep(Duration::from_millis(80));
        push("chrome:9", json!({"type": "toggle"}));
        assert_eq!(t.join().unwrap().len(), 1, "a waiting tab gets its command at once");
        let old = std::thread::spawn(|| wait_commands("chrome:9", 2, Duration::from_secs(5)));
        std::thread::sleep(Duration::from_millis(80));
        let newer = std::thread::spawn(|| wait_commands("chrome:9", 3, Duration::from_secs(1)));
        let started = Instant::now();
        assert!(old.join().unwrap().is_empty(), "a reloaded page's old poll gives way");
        assert!(started.elapsed() < Duration::from_millis(900));
        assert!(newer.join().unwrap().is_empty());
    }
}
