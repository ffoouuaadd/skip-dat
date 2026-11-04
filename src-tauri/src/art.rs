//! Album art, stored once by content hash and served to the windows as
//! `skipart://localhost/<id>`. Views carry only the short id, so updates stay small and an
//! image never has to be decoded twice.

use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, OnceLock};

const CAP: usize = 400;

#[derive(Default)]
struct Store {
    map: HashMap<String, (String, Arc<Vec<u8>>)>,
    order: VecDeque<String>,
    by_url: HashMap<String, String>,
    pending: Vec<String>,
}

static STORE: Mutex<Option<Store>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut Store) -> R) -> R {
    let mut g = STORE.lock().unwrap();
    f(g.get_or_insert_with(Store::default))
}

pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn sniff(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        "image/png"
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        "image/jpeg"
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else if bytes.starts_with(b"GIF8") {
        "image/gif"
    } else {
        "application/octet-stream"
    }
}

/// Store bytes, get their id. The same picture always gets the same id.
pub fn put(bytes: Vec<u8>) -> Option<String> {
    if bytes.len() < 16 {
        return None;
    }
    let id = format!("{:016x}", fnv1a(&bytes));
    let mime = sniff(&bytes).to_string();
    with(|s| {
        if !s.map.contains_key(&id) {
            s.map.insert(id.clone(), (mime, Arc::new(bytes)));
            s.order.push_back(id.clone());
            while s.order.len() > CAP {
                if let Some(old) = s.order.pop_front() {
                    s.map.remove(&old);
                }
            }
        }
    });
    Some(id)
}

pub fn get(id: &str) -> Option<(String, Arc<Vec<u8>>)> {
    with(|s| s.map.get(id).cloned())
}

// ---------- pictures that live on the web (YouTube, SoundCloud, Spotify) ----------

static FETCH: OnceLock<Sender<String>> = OnceLock::new();

/// The id for a picture URL if it is already here; otherwise fetch it in the background and
/// call `done` when it arrives. Never blocks.
pub fn for_url(url: &str, done: fn()) -> Option<String> {
    if url.is_empty() {
        return None;
    }
    if let Some(id) = with(|s| s.by_url.get(url).cloned()) {
        if with(|s| s.map.contains_key(&id)) {
            return Some(id);
        }
    }
    let queued = with(|s| {
        if s.pending.iter().any(|u| u == url) {
            true
        } else {
            s.pending.push(url.to_string());
            false
        }
    });
    if !queued {
        let tx = FETCH.get_or_init(|| {
            let (tx, rx) = channel::<String>();
            std::thread::spawn(move || {
                for url in rx {
                    let bytes = ureq::get(&url)
                        .timeout(std::time::Duration::from_secs(6))
                        .call()
                        .ok()
                        .and_then(|r| {
                            let mut b = Vec::new();
                            use std::io::Read;
                            r.into_reader().take(6_000_000).read_to_end(&mut b).ok().map(|_| b)
                        });
                    let id = bytes.and_then(put);
                    with(|s| {
                        s.pending.retain(|u| u != &url);
                        if let Some(id) = &id {
                            s.by_url.insert(url.clone(), id.clone());
                            if s.by_url.len() > CAP * 2 {
                                s.by_url.clear();
                            }
                        }
                    });
                    if id.is_some() {
                        DONE.lock().unwrap().iter().for_each(|f| f());
                    }
                }
            });
            tx
        });
        let _ = tx.send(url.to_string());
        let mut d = DONE.lock().unwrap();
        if !d.contains(&done) {
            d.push(done);
        }
    }
    None
}

static DONE: Mutex<Vec<fn()>> = Mutex::new(Vec::new());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_dedupes_and_sniffs() {
        let png = [vec![0x89, b'P', b'N', b'G'], vec![7u8; 40]].concat();
        let a = put(png.clone()).unwrap();
        let b = put(png).unwrap();
        assert_eq!(a, b);
        assert_eq!(get(&a).unwrap().0, "image/png");
        assert!(put(vec![1, 2, 3]).is_none());
    }
}
