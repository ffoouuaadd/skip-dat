//! Settings saved in %APPDATA%\app.skipdat.desktop\config.json

use std::path::PathBuf;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// what you hold while scrolling: alt, ctrl, shift, win, ctrl+alt, ctrl+shift, mouse-right, mouse-back, mouse-forward
    #[serde(default = "default_trigger")]
    pub trigger: String,
    /// "vertical" (default) or "horizontal"
    #[serde(default = "default_layout")]
    pub layout: String,
    /// pause the other sources when you pick a song in one
    #[serde(default = "yes")]
    pub pause_others: bool,
    /// app ids shown even though they are not recognised as music ("Show anyway" in Settings)
    #[serde(default)]
    pub allow: Vec<String>,
    // optional Spotify Web API (full upcoming Spotify queue)
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub expires_at: u64,
}

fn yes() -> bool {
    true
}

fn default_trigger() -> String {
    "alt".into()
}

fn default_layout() -> String {
    "vertical".into()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            trigger: default_trigger(),
            layout: default_layout(),
            pause_others: true,
            allow: Vec::new(),
            client_id: String::new(),
            refresh_token: None,
            access_token: None,
            expires_at: 0,
        }
    }
}

pub fn load(path: &PathBuf) -> Config {
    std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save(path: &PathBuf, cfg: &Config) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(s) = serde_json::to_string_pretty(cfg) {
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, s).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}
