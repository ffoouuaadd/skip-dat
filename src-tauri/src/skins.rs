//! Skins: how the carousel looks. The built-in ones ship inside the app (ui/skins/<id>/); anyone
//! can add their own as a folder with a skin.json in %APPDATA%\app.skipdat.desktop\skins (see
//! SKINS.md). That folder is watched, so saving a skin's files updates the carousel right away.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, UNIX_EPOCH};

use serde::Serialize;
use serde_json::Value;

use crate::util::Lock;

const BUILTIN: &[(&str, &str)] = &[
    ("liquid-glass", include_str!("../../ui/skins/liquid-glass/skin.json")),
    ("lime-pod", include_str!("../../ui/skins/lime-pod/skin.json")),
    ("lcd-night", include_str!("../../ui/skins/lcd-night/skin.json")),
    ("brushed-chrome", include_str!("../../ui/skins/brushed-chrome/skin.json")),
    ("pixel-party", include_str!("../../ui/skins/pixel-party/skin.json")),
    ("quicksilver", include_str!("../../ui/skins/quicksilver/skin.json")),
    ("bare-knuckle", include_str!("../../ui/skins/bare-knuckle/skin.json")),
    ("bubble-gum", include_str!("../../ui/skins/bubble-gum/skin.json")),
    ("claude-ish", include_str!("../../ui/skins/claude-ish/skin.json")),
];

#[derive(Clone, Debug, Serialize)]
pub struct Skin {
    pub id: String,
    pub name: String,
    pub author: String,
    pub builtin: bool,
    /// where its files are: "skins/<id>/" inside the app, or the folder name in the skins folder
    pub folder: String,
    /// changes whenever a file of the skin changes (pictures and fonts are loaded again)
    pub rev: u64,
    pub data: Value,
    /// what was wrong with a skin.json that could not be read
    pub error: Option<String>,
}

static DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn init(dir: PathBuf, guide: &str, example: &str) {
    let _ = std::fs::create_dir_all(&dir);
    // the guide, and a skin to start from, in the folder people open from Settings
    let _ = std::fs::write(dir.join("SKINS.md"), guide);
    let ex = dir.join("my-first-skin");
    if !ex.exists() {
        let _ = std::fs::create_dir_all(&ex);
        let _ = std::fs::write(ex.join("skin.json"), example);
    }
    *DIR.guard() = Some(dir);
}

pub fn dir() -> Option<PathBuf> {
    DIR.guard().clone()
}

fn builtin(id: &str, json: &str) -> Skin {
    let data: Value = serde_json::from_str(json).unwrap_or(Value::Null);
    Skin {
        id: id.to_string(),
        name: data["name"].as_str().unwrap_or(id).to_string(),
        author: data["author"].as_str().unwrap_or("Skip Dat").to_string(),
        builtin: true,
        folder: format!("skins/{id}/"),
        rev: 0,
        data,
        error: None,
    }
}

/// A fingerprint of every file in a folder (names, sizes, times).
fn fingerprint(dir: &Path) -> u64 {
    let mut parts: Vec<String> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for f in rd.flatten() {
            let m = f.metadata().ok();
            let t = m.as_ref().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_millis()).unwrap_or(0);
            parts.push(format!("{}:{}:{}", f.file_name().to_string_lossy(), m.map(|m| m.len()).unwrap_or(0), t));
        }
    }
    parts.sort();
    crate::art::fnv1a(parts.join("|").as_bytes())
}

fn user(folder: &Path) -> Option<Skin> {
    let name = folder.file_name()?.to_string_lossy().to_string();
    let file = folder.join("skin.json");
    if !file.is_file() {
        return None;
    }
    let rev = fingerprint(folder);
    let id = format!("user:{name}");
    Some(match std::fs::read_to_string(&file).map_err(|e| e.to_string()).and_then(|s| serde_json::from_str::<Value>(&s).map_err(|e| e.to_string())) {
        Ok(data) if data.is_object() => Skin {
            id,
            name: data["name"].as_str().unwrap_or(&name).chars().take(40).collect(),
            author: data["author"].as_str().unwrap_or("").chars().take(40).collect(),
            builtin: false,
            folder: format!("{name}/"),
            rev,
            data,
            error: None,
        },
        Ok(_) => Skin { id, name: name.clone(), author: String::new(), builtin: false, folder: format!("{name}/"), rev, data: Value::Null, error: Some("skin.json must be a { … } object".into()) },
        Err(e) => Skin { id, name: name.clone(), author: String::new(), builtin: false, folder: format!("{name}/"), rev, data: Value::Null, error: Some(e) },
    })
}

pub fn list() -> Vec<Skin> {
    let mut out: Vec<Skin> = BUILTIN.iter().map(|(id, json)| builtin(id, json)).collect();
    if let Some(dir) = dir() {
        let mut folders: Vec<PathBuf> = std::fs::read_dir(&dir).map(|rd| rd.flatten().map(|f| f.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
        folders.sort();
        out.extend(folders.iter().filter_map(|f| user(f)));
    }
    out
}

/// The skin with this id, or the default one (also when it can't be read).
pub fn get(id: &str) -> Skin {
    let all = list();
    all.iter().find(|s| s.id == id && s.error.is_none()).cloned().unwrap_or_else(|| builtin(BUILTIN[0].0, BUILTIN[0].1))
}

/// Liquid glass shows the desktop through the cards (it needs a picture of the screen); solid
/// skins don't.
pub fn needs_desktop(s: &Skin) -> bool {
    s.data["base"].as_str().unwrap_or("glass") == "glass"
}

/// A file of a skin in the skins folder, for skipskin://localhost/<folder>/<file>.
pub fn file(path: &str) -> Option<(&'static str, Vec<u8>)> {
    let dir = dir()?;
    let rel = path.trim_start_matches('/');
    let rel = rel.split('?').next().unwrap_or("");
    // the page asks through convertFileSrc, which percent-encodes the whole path ("a%2Fhat.png")
    let decoded = percent_decode(rel)?;
    // only plain names inside the folder: no "..", no drive letters, no absolute paths
    if decoded.is_empty() || decoded.split('/').any(|p| p.is_empty() || p == "." || p == ".." || p.contains(':') || p.contains('\\')) {
        return None;
    }
    let full = dir.join(&decoded);
    let mime = match full.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("json") => "application/json",
        _ => return None,
    };
    let bytes = std::fs::read(full).ok()?;
    (bytes.len() <= 16 * 1024 * 1024).then_some((mime, bytes))
}

/// "%2F" to "/", "%20" to " " and so on; None for a broken escape or text that isn't UTF-8.
fn percent_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            out.push(u8::from_str_radix(s.get(i + 1..i + 3)?, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Watch the skins folder; `changed` runs when anything in it changes.
pub fn watch(changed: impl Fn() + Send + 'static) {
    std::thread::spawn(move || {
        let mut last = 0u64;
        loop {
            let now = dir()
                .map(|d| {
                    let mut f = fingerprint(&d);
                    if let Ok(rd) = std::fs::read_dir(&d) {
                        for sub in rd.flatten().filter(|e| e.path().is_dir()) {
                            f ^= fingerprint(&sub.path()).rotate_left(7);
                        }
                    }
                    f
                })
                .unwrap_or(0);
            if last != 0 && now != last {
                changed();
            }
            last = now;
            std::thread::sleep(Duration::from_millis(900));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both tests point the one skins folder (DIR) somewhere else; run them one at a time.
    static SERIAL: Mutex<()> = Mutex::new(());

    #[test]
    fn built_in_skins_read_and_user_skins_load_with_errors_shown() {
        let _one_at_a_time = SERIAL.guard();
        let dir = std::env::temp_dir().join(format!("skipdat-skins-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone(), "# guide", "{\"name\": \"Starter\"}");
        assert!(dir.join("SKINS.md").is_file());
        assert!(dir.join("my-first-skin").join("skin.json").is_file(), "a skin to start from");
        std::fs::create_dir_all(dir.join("broken")).unwrap();
        std::fs::write(dir.join("broken").join("skin.json"), "{ \"name\": ").unwrap();
        std::fs::create_dir_all(dir.join("not-a-skin")).unwrap();
        let all = list();
        for &(id, _) in BUILTIN {
            let s = all.iter().find(|s| s.id == id).expect(id);
            assert!(s.builtin && s.error.is_none() && s.data.is_object(), "{id} reads");
        }
        assert_eq!(all.iter().find(|s| s.id == "user:my-first-skin").map(|s| s.name.as_str()), Some("Starter"));
        assert!(all.iter().find(|s| s.id == "user:broken").and_then(|s| s.error.clone()).is_some(), "a broken skin.json says so");
        assert!(!all.iter().any(|s| s.id == "user:not-a-skin"), "folders without skin.json are not skins");
        // a broken or missing skin falls back to the default
        assert_eq!(get("user:broken").id, "liquid-glass");
        assert_eq!(get("nope").id, "liquid-glass");
        assert!(needs_desktop(&get("liquid-glass")) && !needs_desktop(&get("pixel-party")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn skin_files_stay_inside_the_skins_folder() {
        let _one_at_a_time = SERIAL.guard();
        let dir = std::env::temp_dir().join(format!("skipdat-skinfiles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone(), "", "{}");
        std::fs::create_dir_all(dir.join("a")).unwrap();
        std::fs::write(dir.join("a").join("hat.png"), b"\x89PNG....").unwrap();
        std::fs::write(dir.join("secret.txt"), b"no").unwrap();
        assert_eq!(file("/a/hat.png").map(|f| f.0), Some("image/png"));
        assert!(file("/a/hat.png?r=12").is_some());
        // what the page really asks for: convertFileSrc percent-encodes the slash
        assert_eq!(file("/a%2Fhat.png?r=12").map(|f| f.0), Some("image/png"));
        assert!(file("/..%2Fsecret.txt").is_none());
        assert!(file("/a%2F%2E%2E%2F%2E%2E%2Fx.png").is_none(), "encoded .. is still refused");
        assert!(file("/a%2Fhat.png%").is_none(), "a broken escape is refused");
        assert!(file("/../secret.txt").is_none());
        assert!(file("/a/../../x.png").is_none());
        assert!(file("/C:/Windows/win.ini").is_none());
        assert!(file("/secret.txt").is_none(), "only pictures and fonts");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
