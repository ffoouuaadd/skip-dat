//! Everything Skip Dat does to a player runs here, one action at a time and in order, so rapid
//! clicks and scrolls can never overlap (two skips racing each other is how players end up on the
//! wrong song). Each action looks the song up again in the freshest view first, and after an
//! action that changes the song the worker waits until the player has really moved on before the
//! next one, so positions are never stale.
//!
//! Reaching a song further away (plan.rs decides how): straight there when the player can do
//! that without reordering its queue; otherwise next/previous presses with that player muted, so
//! none of the songs in between is heard. Spotify (with its optional connection) always goes
//! straight there, like an ender pearl: it starts the song inside the playlist or album.
//!
//! Every skip and jump remembers the song that was playing, so "Back to …" under the play/pause
//! button can return to it (and back again).

use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::json;
use tauri::{AppHandle, Manager};

use crate::plan::{self, Caps, Kind, Route};
use crate::util::{log, Lock};
use crate::view::{self, Item, Source, Target};
use crate::{audio, browser, classify, history, media, spotify, AppState};

pub enum Action {
    /// play the song with `item` key (or, if it is gone, the one `rel` places from now)
    Choose { source: String, item: String, rel: i32 },
    /// play, pause, toggle, next, previous; `source` None means the active source
    Control { source: Option<String>, action: String },
    /// a song from the play history (history.rs)
    Replay { id: u64 },
    /// back to the song that was playing before the last skip or jump
    Back,
}

static TX: OnceLock<Sender<Action>> = OnceLock::new();

pub fn send(a: Action) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(a);
    }
}

/// After a direct jump: what should come next if the site kept its order.
struct OrderCheck {
    source: String,
    site: String,
    expected: Vec<String>,
}

struct Outcome {
    /// the source, and the song it played before (to wait for the change)
    moved: Option<(String, Option<String>)>,
    spotify: bool,
    check: Option<OrderCheck>,
}

const NOTHING: Outcome = Outcome { moved: None, spotify: false, check: None };

pub fn start(app: AppHandle) {
    let (tx, rx) = channel::<Action>();
    let _ = TX.set(tx);
    std::thread::Builder::new()
        .name("actions".into())
        .spawn(move || {
            media::init_thread();
            for a in rx {
                let out = match a {
                    Action::Choose { source, item, rel } => choose(&app, &source, &item, rel),
                    Action::Control { source, action } => control(&app, source.as_deref(), &action),
                    Action::Replay { id } => replay(&app, id),
                    Action::Back => go_back(&app),
                };
                media::poke();
                view::changed();
                if let Some((source, before)) = &out.moved {
                    settle(&app, source, before.clone());
                }
                if let Some(c) = out.check {
                    check_order(&app, c);
                }
                if out.spotify && spotify::connected(&app) {
                    let app2 = app.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(600));
                        view::refresh_spotify(&app2, true);
                    });
                }
            }
        })
        .expect("actions thread");
}

// ---------- "Back to …": the song playing before the last skip ----------

#[derive(Clone)]
struct BackTo {
    source: String,
    item: Item,
    /// Spotify: the playlist or album it was playing from
    ctx: Option<String>,
}

static BACK: Mutex<Option<BackTo>> = Mutex::new(None);

/// Remember the song `s` is playing now, before moving away from it.
fn remember(s: &Source, ctx: Option<String>) {
    if let Some(now) = s.items.iter().find(|i| i.rel == 0) {
        if !now.title.is_empty() {
            *BACK.guard() = Some(BackTo { source: s.key.clone(), item: now.clone(), ctx });
        }
    }
}

/// What the overlay shows under the play/pause button: only while that app is still there and
/// isn't playing that very song.
pub fn back_view(sources: &[Source]) -> Option<view::Back> {
    let b = BACK.guard().clone()?;
    let s = sources.iter().find(|s| s.key == b.source)?;
    let now = s.items.iter().find(|i| i.rel == 0)?;
    if classify::same_title(&now.title, &b.item.title) && (now.artist == b.item.artist || b.item.artist.is_empty()) {
        return None;
    }
    Some(view::Back { source: b.source, title: b.item.title, artist: b.item.artist, art: b.item.art })
}

fn go_back(app: &AppHandle) -> Outcome {
    let Some(b) = BACK.guard().clone() else { return NOTHING };
    let Some(s) = find_source(app, Some(&b.source)) else { return NOTHING };
    let same = |i: &Item| classify::same_title(&i.title, &b.item.title) && (b.item.artist.is_empty() || i.artist == b.item.artist);
    // Spotify: straight back into the playlist or album it was playing from
    if is_spotify_source(&s) && spotify::connected(app) && b.item.uri.is_some() {
        let ctx_now = spotify::context(app);
        let mut it = b.item.clone();
        it.ctx = b.ctx.clone();
        it.rel = s.items.iter().filter(|i| i.rel < 0 && same(i)).map(|i| i.rel).max().unwrap_or(-1);
        let before = now_key(app, &s.key);
        remember(&s, ctx_now.clone());
        if spotify_jump(app, &s, &it, ctx_now.as_deref()) {
            view::set_chosen(&s.key);
            pause_others(app, &s);
            return Outcome { moved: Some((s.key.clone(), before)), spotify: true, check: None };
        }
    }
    // still in the app's list: the usual way for that player (choose remembers where we are now)
    if let Some(it) = s.items.iter().filter(|i| i.rel != 0 && same(i)).min_by_key(|i| (i.rel > 0, i.rel.abs())) {
        let (key, rel) = (it.key.clone(), it.rel);
        return choose(app, &s.key, &key, rel);
    }
    // a tab that has moved on: open the song's page there
    if let (Target::Tab(key), Some(url)) = (&s.target, b.item.url.as_deref()) {
        let before = now_key(app, &s.key);
        remember(&s, None);
        browser::push(key, json!({ "type": "open", "url": url }));
        return Outcome { moved: Some((s.key.clone(), before)), spotify: false, check: None };
    }
    log(&format!("back: \"{}\" is no longer in {}", b.item.title, s.name));
    NOTHING
}

/// Spotify, connected: play `it` straight away. Inside the playlist or album it belongs to when
/// Spotify allows that (the rest of the playlist then follows as usual); otherwise the song and
/// everything listed after it, in the order shown. False if Spotify refused both.
fn spotify_jump(app: &AppHandle, s: &Source, it: &Item, ctx_now: Option<&str>) -> bool {
    let Some(uri) = it.uri.as_deref() else { return false };
    let ctx = it.ctx.as_deref().or(ctx_now);
    if let Some(c) = ctx.filter(|c| plan::spotify_offset_ok(c)) {
        match spotify::send(app, "PUT", "/v1/me/player/play", Some(json!({ "context_uri": c, "offset": { "uri": uri }, "position_ms": 0 }))) {
            Ok(()) => return true,
            Err(e) => log(&format!("spotify: couldn't start {c} at the song ({e}); playing it as a list")),
        }
    }
    let shown: Vec<(i32, Option<String>)> = s.items.iter().map(|i| (i.rel, i.uri.clone())).collect();
    let mut list = plan::spotify_list(&shown, it.rel);
    if list.first().map(String::as_str) != Some(uri) {
        // a song that isn't in the list any more ("Back to …"): it, then what's playing and next
        list = std::iter::once(uri.to_string()).chain(plan::spotify_list(&shown, 0)).take(50).collect();
    }
    match spotify::send(app, "PUT", "/v1/me/player/play", Some(json!({ "uris": list, "position_ms": 0 }))) {
        Ok(()) => true,
        Err(e) => {
            log(&format!("spotify: couldn't play the song directly ({e}); skipping to it muted"));
            false
        }
    }
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

/// The basic transport actions on any source.
fn transport(s: &Source, action: &str) {
    match &s.target {
        Target::Smtc(id) => {
            media::with_session(id, |sess| media::act(sess, action));
        }
        Target::Tab(key) => browser::push(key, json!({ "type": action })),
    }
}

fn is_spotify_source(s: &Source) -> bool {
    matches!(&s.target, Target::Smtc(id) if media::is_spotify(id))
}

fn control(app: &AppHandle, source: Option<&str>, action: &str) -> Outcome {
    let Some(s) = find_source(app, source) else { return NOTHING };
    let action = match action {
        "play" | "pause" | "next" | "previous" => action,
        _ => "toggle",
    };
    // a player without the button: pressing it would do nothing useful (or something odd)
    if (action == "previous" && !s.can_prev) || (action == "next" && !s.can_next) {
        return NOTHING;
    }
    let before = now_key(app, &s.key);
    let moves = action == "next" || action == "previous";
    if moves {
        remember(&s, None);
    }
    transport(&s, action);
    if moves && is_spotify_source(&s) && spotify::connected(app) {
        // the playlist it plays from, so "Back to …" can go straight back into it
        let ctx = spotify::context(app);
        if let Some(b) = BACK.guard().as_mut() {
            b.ctx = ctx;
        }
    }
    if action == "play" || (action == "toggle" && !s.playing) {
        view::set_chosen(&s.key);
        pause_others(app, &s);
    }
    let moved = action == "next" || action == "previous";
    Outcome { moved: moved.then(|| (s.key.clone(), before)), spotify: is_spotify_source(&s), check: None }
}

fn pause_others(app: &AppHandle, keep: &Source) {
    if !app.state::<AppState>().cfg.guard().pause_others {
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

fn caps(app: &AppHandle, s: &Source, it: &Item) -> Caps {
    match &s.target {
        Target::Smtc(id) => Caps {
            kind: Kind::App,
            can_jump: false,
            in_list: false,
            jump_reorders: false,
            can_prev: s.can_prev,
            can_next: s.can_next,
            position: media::with_session(id, media::position_secs).unwrap_or(0.0),
        },
        Target::Tab(_) => Caps {
            kind: Kind::Tab,
            can_jump: s.can_jump,
            in_list: it.jump.is_some(),
            // SoundCloud with shuffle on rebuilds what comes next when a song in its list is
            // clicked (checked live); other sites only if Skip Dat has seen them do it
            jump_reorders: (s.shuffle && s.site == "soundcloud") || app.state::<AppState>().cfg.guard().jump_reorders.contains(&s.site),
            can_prev: s.can_prev,
            can_next: s.can_next,
            position: 0.0,
        },
    }
}

/// Wait until this app plays `title` (or `ms` pass).
fn wait_for_title(id: &str, title: &str, ms: u64) -> bool {
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_millis(ms) {
        if media::title_of(id).map(|t| classify::same_title(&t, title)).unwrap_or(false) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(40));
    }
    false
}

/// Next / previous `count` times with the app muted; unmuted once the picked song plays.
fn app_skip(id: &str, count: i32, title: &str) {
    let app_id = media::app_of(id).unwrap_or_else(|| id.to_string());
    let muted = audio::mute(&app_id);
    media::passing(id, media::title_of(id));
    media::with_session(id, |sess| media::skip(sess, count));
    let reached = wait_for_title(id, title, 3500);
    media::passing(id, None);
    media::with_session(id, |sess| media::act(sess, "play"));
    // the new song's first moments are already playing; let them through before unmuting
    std::thread::sleep(Duration::from_millis(if reached { 160 } else { 0 }));
    if !reached {
        log(&format!("skip: {app_id} did not reach \"{title}\" after {count} presses"));
    }
    drop(muted);
}

fn choose(app: &AppHandle, source: &str, item: &str, rel_hint: i32) -> Outcome {
    let Some(s) = find_source(app, Some(source)) else { return NOTHING };
    // by key; a song listed twice: the copy nearest to where it was picked
    let it = s
        .items
        .iter()
        .filter(|i| i.key == item || i.key.split('-').next() == item.split('-').next())
        .min_by_key(|i| (i.key != item, (i.rel - rel_hint).abs()))
        .or_else(|| s.items.iter().find(|i| i.rel == rel_hint));
    let Some(it) = it.cloned() else { return NOTHING };
    let rel = it.rel;
    let before = now_key(app, &s.key);
    // Spotify, connected: straight to the song (whatever its buttons allow), no skipping through.
    // An earlier song Skip Dat has no Spotify link for is looked up by name.
    let mut it = it;
    if rel < 0 && it.uri.is_none() && is_spotify_source(&s) && spotify::connected(app) {
        it.uri = spotify::find_track(app, &it.title, &it.artist);
    }
    if rel != 0 && it.uri.is_some() && is_spotify_source(&s) && spotify::connected(app) {
        let ctx_now = spotify::context(app);
        remember(&s, ctx_now.clone());
        if spotify_jump(app, &s, &it, ctx_now.as_deref()) {
            view::set_chosen(&s.key);
            pause_others(app, &s);
            return Outcome { moved: Some((s.key.clone(), before)), spotify: true, check: None };
        }
    }
    let route = plan::route(rel, &caps(app, &s, &it));
    if !matches!(route, Route::Stay | Route::Cannot) && !(it.uri.is_some() && is_spotify_source(&s) && spotify::connected(app)) {
        remember(&s, None);
    }
    let mut check = None;
    match (&route, &s.target) {
        (Route::Cannot, _) => {
            log(&format!("choose: {} can't move {rel} songs (no previous/next)", s.name));
            return NOTHING;
        }
        (Route::Stay, Target::Smtc(id)) => {
            if !s.playing {
                media::with_session(id, |sess| media::act(sess, "play"));
            }
        }
        (Route::AppSkip { count }, Target::Smtc(id)) => app_skip(id, *count, &it.title),
        (_, Target::Tab(key)) => {
            let mut cmds = Vec::new();
            match route {
                Route::Jump => {
                    if let Some(index) = it.jump {
                        cmds.push(json!({ "type": "jump", "index": index, "title": it.title }));
                    }
                    // what should follow the picked song if the site keeps its order
                    let mut after: Vec<&Item> = s.items.iter().filter(|i| i.rel > rel).collect();
                    after.sort_by_key(|i| i.rel);
                    check = Some(OrderCheck { source: s.key.clone(), site: s.site.clone(), expected: after.iter().take(10).map(|i| i.title.clone()).collect() });
                }
                Route::TabBack { count } => cmds.push(json!({ "type": "back", "count": count, "title": it.title })),
                Route::TabSkip { count } => cmds.push(json!({ "type": "skipto", "count": count, "title": it.title })),
                _ => {}
            }
            if rel != 0 || !s.playing {
                cmds.push(json!({ "type": "play" }));
            }
            browser::push_many(key, cmds);
        }
        _ => {}
    }
    view::set_chosen(&s.key);
    pause_others(app, &s);
    Outcome { moved: (rel != 0).then(|| (s.key.clone(), before)), spotify: is_spotify_source(&s), check }
}

/// Did the jump keep the queue's order? If a site reorders it (a shuffle that reshuffles on a
/// jump), its jumps go the muted-skip way from now on, and that is remembered.
fn check_order(app: &AppHandle, c: OrderCheck) {
    if c.expected.len() < 3 || c.site.is_empty() {
        return;
    }
    std::thread::sleep(Duration::from_millis(400));
    let Some(s) = view::latest(app).sources.into_iter().find(|s| s.key == c.source) else { return };
    let mut next: Vec<&Item> = s.items.iter().filter(|i| i.rel > 0).collect();
    next.sort_by_key(|i| i.rel);
    let after: Vec<String> = next.iter().take(10).map(|i| i.title.clone()).collect();
    if after.len() < 3 || plan::order_kept(&c.expected, &after) {
        return;
    }
    log(&format!("jump: {} reordered its queue after a direct jump; using muted skips there from now on", c.site));
    let state = app.state::<AppState>();
    {
        let mut cfg = state.cfg.guard();
        if !cfg.jump_reorders.contains(&c.site) {
            cfg.jump_reorders.push(c.site.clone());
        }
    }
    crate::save_cfg(&state);
}

/// Play a song from the history again: from its app's list when it is still there; otherwise
/// its page (YouTube, YouTube Music, SoundCloud) in an open tab of that site or the browser;
/// Spotify by its URI or a search; anything else as a search.
fn replay(app: &AppHandle, id: u64) -> Outcome {
    let Some(e) = history::get(id) else { return NOTHING };
    let v = view::latest(app);
    let same = |i: &Item| classify::same_title(&i.title, &e.title) && (e.artist.is_empty() || i.artist == e.artist);
    for s in v.sources.iter().filter(|s| s.key == e.source || s.name == e.app) {
        if let Some(it) = s.items.iter().filter(|i| same(i)).min_by_key(|i| i.rel.abs()) {
            return choose(app, &s.key, &it.key, it.rel);
        }
    }
    if let Some(url) = e.url.as_deref().filter(|u| u.starts_with("https://")) {
        let site = if url.contains("music.youtube.com") {
            "ytmusic"
        } else if url.contains("youtube.com") {
            "youtube"
        } else if url.contains("soundcloud.com") {
            "soundcloud"
        } else {
            ""
        };
        if let Some(t) = browser::tabs().into_iter().find(|t| !site.is_empty() && t.report.site == site) {
            browser::push(&t.key, json!({ "type": "open", "url": url }));
        } else {
            let _ = open::that(url);
        }
        return NOTHING;
    }
    let spotify = e.app.eq_ignore_ascii_case("spotify") || e.source.to_lowercase().contains("spotify");
    if spotify {
        if spotify::connected(app) {
            if let Some(uri) = e.uri.clone().or_else(|| spotify::find_track(app, &e.title, &e.artist)) {
                if spotify::send(app, "PUT", "/v1/me/player/play", Some(json!({ "uris": [uri] }))).is_ok() {
                    return NOTHING;
                }
            }
        }
        let _ = open::that(format!("spotify:search:{}", spotify::enc(&format!("{} {}", e.title, e.artist))));
        return NOTHING;
    }
    let q = spotify::enc(&format!("{} {}", e.title, e.artist));
    let url = if e.app.to_lowercase().contains("apple") { format!("https://music.apple.com/search?term={q}") } else { format!("https://music.youtube.com/search?q={q}") };
    let _ = open::that(url);
    NOTHING
}
