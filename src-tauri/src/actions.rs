//! Everything Skip Dat does to a player runs here, one action at a time and in order, so rapid
//! clicks and scrolls can never overlap (two skips racing each other is how players end up on the
//! wrong song). Each action looks the song up again in the freshest view first, and after an
//! action that changes the song the worker waits until the player has really moved on before the
//! next one, so positions are never stale.

use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use serde_json::json;
use tauri::{AppHandle, Manager};

use crate::view::{self, Source, Target};
use crate::{browser, media, spotify, AppState};

pub enum Action {
    /// play the song with `item` key (or, if it is gone, the one `rel` places from now)
    Choose { source: String, item: String, rel: i32 },
    /// play, pause, toggle, next, previous; `source` None means the active source
    Control { source: Option<String>, action: String },
}

static TX: OnceLock<Sender<Action>> = OnceLock::new();

pub fn send(a: Action) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(a);
    }
}

pub fn start(app: AppHandle) {
    let (tx, rx) = channel::<Action>();
    let _ = TX.set(tx);
    std::thread::spawn(move || {
        media::init_thread();
        for a in rx {
            let (key, moved, spotify_touched) = match a {
                Action::Choose { source, item, rel } => choose(&app, &source, &item, rel),
                Action::Control { source, action } => control(&app, source.as_deref(), &action),
            };
            media::poke();
            view::changed();
            if moved {
                if let Some(k) = key {
                    settle(&app, &k.0, k.1);
                }
            }
            if spotify_touched && spotify::connected(&app) {
                let app2 = app.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(600));
                    view::refresh_spotify(&app2, true);
                });
            }
        }
    });
}

fn now_key(app: &AppHandle, source: &str) -> Option<String> {
    view::latest(app).sources.into_iter().find(|s| s.key == source)?.items.into_iter().find(|i| i.rel == 0).map(|i| i.key)
}

/// Wait (up to 1.5 s) until the source shows a different song than `before`.
fn settle(app: &AppHandle, source: &str, before: Option<String>) {
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_millis(1500) {
        std::thread::sleep(Duration::from_millis(40));
        if now_key(app, source) != before {
            // let the rest of the update (its queue) arrive too
            std::thread::sleep(Duration::from_millis(80));
            return;
        }
    }
}

fn find_source(app: &AppHandle, key: Option<&str>) -> Option<Source> {
    let v = view::latest(app);
    let key = key.map(String::from).unwrap_or(v.active.clone());
    v.sources.into_iter().find(|s| s.key == key)
}

fn tab_cmd(key: &str, kind: &str) {
    browser::push(key, json!({ "type": kind }));
}

/// The basic transport actions on any source.
fn transport(s: &Source, action: &str) {
    match &s.target {
        Target::Smtc(id) => {
            media::with_session(id, |sess| media::act(sess, action));
        }
        Target::Tab(key) => tab_cmd(key, action),
    }
}

type Outcome = (Option<(String, Option<String>)>, bool, bool);

fn is_spotify_source(s: &Source) -> bool {
    matches!(&s.target, Target::Smtc(id) if media::is_spotify(id))
}

fn control(app: &AppHandle, source: Option<&str>, action: &str) -> Outcome {
    let Some(s) = find_source(app, source) else { return (None, false, false) };
    let action = match action {
        "play" | "pause" | "next" | "previous" => action,
        _ => "toggle",
    };
    let before = now_key(app, &s.key);
    transport(&s, action);
    if action == "play" || (action == "toggle" && !s.playing) {
        view::set_chosen(&s.key);
        pause_others(app, &s);
    }
    let moved = action == "next" || action == "previous";
    (Some((s.key.clone(), before)), moved, is_spotify_source(&s))
}

fn pause_others(app: &AppHandle, keep: &Source) {
    if !app.state::<AppState>().cfg.lock().unwrap().pause_others {
        return;
    }
    for o in view::latest(app).sources.iter().filter(|o| o.key != keep.key && o.playing) {
        // a browser's own media controls act on whatever that browser played last, which may be
        // the very tab just chosen: leave them alone when the choice is a tab in that browser
        if matches!(keep.target, Target::Tab(_)) && matches!(o.target, Target::Smtc(_)) && !o.via.is_empty() && o.via == keep.via {
            continue;
        }
        transport(o, "pause");
    }
}

fn choose(app: &AppHandle, source: &str, item: &str, rel_hint: i32) -> Outcome {
    let Some(s) = find_source(app, Some(source)) else { return (None, false, false) };
    // by key; a song listed twice: the copy nearest to where it was picked
    let it = s
        .items
        .iter()
        .filter(|i| i.key == item || i.key.split('-').next() == item.split('-').next())
        .min_by_key(|i| (i.key != item, (i.rel - rel_hint).abs()))
        .or_else(|| s.items.iter().find(|i| i.rel == rel_hint));
    let Some(it) = it.cloned() else { return (None, false, false) };
    let rel = it.rel;
    let before = now_key(app, &s.key);
    let mut spotify_touched = false;
    match &s.target {
        Target::Smtc(id) => {
            let is_sp = media::is_spotify(id);
            spotify_touched = is_sp;
            // an earlier Spotify song can be played directly when Spotify is connected
            let played_uri = rel < 0
                && is_sp
                && it.uri.as_ref().map(|u| spotify::send(app, "PUT", "/v1/me/player/play", Some(json!({ "uris": [u] }))).is_ok()).unwrap_or(false);
            media::with_session(id, |sess| {
                if rel > 0 {
                    media::skip(sess, rel);
                } else if rel < 0 && !played_uri {
                    // most players restart the current song on the first "previous" once a few seconds in
                    let extra = if media::position_secs(sess) > 3.5 { 1 } else { 0 };
                    media::skip(sess, rel - extra);
                }
                if rel != 0 || !s.playing {
                    media::act(sess, "play");
                }
            });
        }
        Target::Tab(key) => {
            let mut cmds = Vec::new();
            if rel != 0 {
                if let (true, Some(index)) = (s.can_jump, it.jump) {
                    cmds.push(json!({ "type": "jump", "index": index, "title": it.title }));
                } else {
                    let kind = if rel > 0 { "next" } else { "previous" };
                    for _ in 0..rel.unsigned_abs().min(30) {
                        cmds.push(json!({ "type": kind }));
                    }
                }
            }
            if rel != 0 || !s.playing {
                cmds.push(json!({ "type": "play" }));
            }
            // one delivery, so the page spaces the skips evenly
            browser::push_many(key, cmds);
        }
    }
    view::set_chosen(&s.key);
    pause_others(app, &s);
    (Some((s.key.clone(), before)), rel != 0, spotify_touched)
}
