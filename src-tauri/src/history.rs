//! "What was I playing?": the songs you listened to, newest first, kept only on this PC
//! (%APPDATA%\app.skipdat.desktop\history.json, pictures in history-art\).
//!
//! A song counts once it has played for a few seconds, so songs passed on the way to another
//! (muted skips, quick flicks through a queue) don't fill the list.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::util::Lock;

pub const KEEP: usize = 500;
/// how long a song has to play before it is part of the history
pub const LISTENED: Duration = Duration::from_secs(8);

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    pub id: u64,
    pub title: String,
    pub artist: String,
    /// where it played: "Spotify", "YouTube Music"…
    pub app: String,
    /// the browser, for music in one
    #[serde(default)]
    pub via: String,
    /// seconds since 1970
    pub time: u64,
    /// picture file in history-art\ (also served as skipart://localhost/<art>)
    #[serde(default)]
    pub art: Option<String>,
    /// how to play it again: a page address (YouTube, YouTube Music, SoundCloud) or a Spotify URI
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub uri: Option<String>,
    /// the source it came from ("app:Spotify.exe", "tab:chrome:12"), to find it there first
    #[serde(default)]
    pub source: String,
}

/// Add a song, unless it is the same one as the newest entry; keep the newest `KEEP`.
pub fn push(list: &mut Vec<Entry>, mut e: Entry) -> bool {
    if let Some(last) = list.first() {
        if last.title == e.title && last.artist == e.artist && last.app == e.app {
            return false;
        }
    }
    e.id = list.iter().map(|x| x.id).max().unwrap_or(0) + 1;
    list.insert(0, e);
    list.truncate(KEEP);
    true
}

/// Waits for each source's song to have played long enough before it is recorded.
#[derive(Default)]
pub struct Listener {
    pending: HashMap<String, (Entry, Instant)>,
    recorded: HashMap<String, (String, String)>,
}

impl Listener {
    /// A source playing `e` now (call on every update); returns it once it has played long enough.
    pub fn saw(&mut self, source: &str, e: Entry, now: Instant) -> Option<Entry> {
        if e.title.is_empty() {
            return None;
        }
        let key = (e.title.clone(), e.artist.clone());
        if self.recorded.get(source) == Some(&key) {
            return None;
        }
        match self.pending.get_mut(source) {
            Some((p, since)) if p.title == e.title && p.artist == e.artist => {
                // keep the best details seen so far (the picture often arrives a moment later)
                if p.art.is_none() {
                    p.art = e.art;
                }
                if p.url.is_none() {
                    p.url = e.url;
                }
                if p.uri.is_none() {
                    p.uri = e.uri;
                }
                if now.duration_since(*since) >= LISTENED {
                    let done = p.clone();
                    self.pending.remove(source);
                    self.recorded.insert(source.to_string(), key);
                    return Some(done);
                }
                None
            }
            _ => {
                self.pending.insert(source.to_string(), (e, now));
                None
            }
        }
    }

    /// A source that stopped playing (or went away): its unfinished song doesn't count yet.
    pub fn paused(&mut self, source: &str) {
        self.pending.remove(source);
    }
}

// ---------- the file ----------

static FILE: Mutex<Option<(PathBuf, Vec<Entry>)>> = Mutex::new(None);

pub fn init(dir: PathBuf) {
    let path = dir.join("history.json");
    let list = std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    *FILE.guard() = Some((path, list));
}

pub fn art_dir() -> Option<PathBuf> {
    FILE.guard().as_ref().and_then(|(p, _)| p.parent().map(|d| d.join("history-art")))
}

fn save(path: &PathBuf, list: &[Entry]) {
    if let Ok(s) = serde_json::to_string(list) {
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, s).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Record a song (its picture is copied next to the history so it outlives the app's memory).
pub fn record(mut e: Entry) -> bool {
    if let (Some(id), Some(dir)) = (e.art.clone(), art_dir()) {
        let file = dir.join(&id);
        if !file.exists() {
            match crate::art::get(&id) {
                Some((_, bytes)) => {
                    let _ = std::fs::create_dir_all(&dir);
                    let _ = std::fs::write(&file, bytes.as_slice());
                }
                None => e.art = None,
            }
        }
    }
    e.time = now_secs();
    let mut g = FILE.guard();
    let Some((path, list)) = g.as_mut() else { return false };
    if !push(list, e) {
        return false;
    }
    save(path, list);
    // pictures no entry uses any more
    if list.len() == KEEP {
        if let Some(dir) = path.parent().map(|d| d.join("history-art")) {
            let used: std::collections::HashSet<&String> = list.iter().filter_map(|e| e.art.as_ref()).collect();
            if let Ok(rd) = std::fs::read_dir(&dir) {
                for f in rd.flatten() {
                    let name = f.file_name().to_string_lossy().to_string();
                    if !used.contains(&name) {
                        let _ = std::fs::remove_file(f.path());
                    }
                }
            }
        }
    }
    true
}

pub fn list() -> Vec<Entry> {
    FILE.guard().as_ref().map(|(_, l)| l.clone()).unwrap_or_default()
}

pub fn get(id: u64) -> Option<Entry> {
    FILE.guard().as_ref().and_then(|(_, l)| l.iter().find(|e| e.id == id).cloned())
}

pub fn clear() {
    let mut g = FILE.guard();
    if let Some((path, list)) = g.as_mut() {
        list.clear();
        save(path, list);
        if let Some(dir) = path.parent().map(|d| d.join("history-art")) {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// A history picture (for the skipart:// address), when it is no longer in memory.
pub fn art_bytes(id: &str) -> Option<Vec<u8>> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    std::fs::read(art_dir()?.join(id)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(t: &str) -> Entry {
        Entry { title: t.into(), artist: "A".into(), app: "Spotify".into(), ..Default::default() }
    }

    #[test]
    fn newest_first_no_repeats_capped() {
        let mut l = Vec::new();
        assert!(push(&mut l, e("one")));
        assert!(!push(&mut l, e("one")), "the same song twice in a row is one entry");
        assert!(push(&mut l, e("two")));
        assert_eq!(l[0].title, "two");
        assert!(l[0].id > l[1].id);
        for i in 0..KEEP + 20 {
            push(&mut l, e(&format!("s{i}")));
        }
        assert_eq!(l.len(), KEEP);
    }

    #[test]
    fn only_songs_that_played_a_while() {
        let mut ls = Listener::default();
        let t0 = Instant::now();
        assert!(ls.saw("app:Spotify", e("one"), t0).is_none());
        // passed through quickly (a skip): never recorded
        assert!(ls.saw("app:Spotify", e("two"), t0 + Duration::from_secs(1)).is_none());
        assert!(ls.saw("app:Spotify", e("three"), t0 + Duration::from_secs(2)).is_none());
        let got = ls.saw("app:Spotify", e("three"), t0 + Duration::from_secs(11));
        assert_eq!(got.map(|x| x.title), Some("three".into()));
        // and only once
        assert!(ls.saw("app:Spotify", e("three"), t0 + Duration::from_secs(30)).is_none());
        // a pause resets the clock
        assert!(ls.saw("app:Spotify", e("four"), t0 + Duration::from_secs(31)).is_none());
        ls.paused("app:Spotify");
        assert!(ls.saw("app:Spotify", e("four"), t0 + Duration::from_secs(40)).is_none());
        assert!(ls.saw("app:Spotify", e("four"), t0 + Duration::from_secs(49)).is_some());
    }
}
