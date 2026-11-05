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
    // ---- look (1.4) ----
    /// carousel style: "wheel" (default), "stack", "coverflow", "orbit", "compact"
    #[serde(default = "default_style")]
    pub style: String,
    /// "small", "medium" (default), "large"
    #[serde(default = "default_size")]
    pub size: String,
    /// how blurry the songs away from the middle get, 0..1
    #[serde(default = "default_blur")]
    pub depth_blur: f64,
    /// how much they fade, 0..1
    #[serde(default = "default_fade")]
    pub depth_fade: f64,
    /// performance mode: no blur, glass, shadows, animations or album art
    #[serde(default)]
    pub perf: bool,
    /// skin id: a built-in one or a folder in the skins folder
    #[serde(default = "default_skin")]
    pub skin: String,
    /// sites whose direct "play this queue item" was seen to reorder the queue (shuffle):
    /// they get muted skips instead
    #[serde(default)]
    pub jump_reorders: Vec<String>,
}

fn default_style() -> String {
    "wheel".into()
}

fn default_size() -> String {
    "medium".into()
}

fn default_blur() -> f64 {
    0.55
}

fn default_fade() -> f64 {
    0.45
}

fn default_skin() -> String {
    "liquid-glass".into()
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
            style: default_style(),
            size: default_size(),
            depth_blur: default_blur(),
            depth_fade: default_fade(),
            perf: false,
            skin: default_skin(),
            jump_reorders: Vec::new(),
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
