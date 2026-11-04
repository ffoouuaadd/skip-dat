//! Which media sources are music. Pure logic (no Windows calls) so it is unit-tested on any OS.
//!
//! Rules, in order:
//! 1. Dedicated music apps (and their installed web apps) are music.
//! 2. Browsers are music only when the page is a music site we can recognise: the Skip Dat
//!    browser companion says so exactly (site + YouTube's own "Music" category); without it we
//!    use what the page publishes to Windows (an album, "- Topic"/VEVO channels, music-video
//!    titles). Social and video sites are never music.
//! 3. Everything else (chat and AI apps, meeting apps, games, video players) is not music,
//!    unless the person chose "Show anyway" in Settings.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppKind {
    Music(&'static str),
    Browser(&'static str),
    Other,
}

/// Web apps installed from Chrome/Edge carry the store id of the site in their app id.
const PWAS: &[(&str, &str)] = &[
    ("cinhimbnkkaeohfgghhklpknlkffjgod", "YouTube Music"),
    ("pjibgclleladliembfgfagdaldikeohf", "Spotify"),
    ("music.youtube.com", "YouTube Music"),
    ("soundcloud.com", "SoundCloud"),
    ("open.spotify.com", "Spotify"),
    ("music.apple.com", "Apple Music"),
];

const MUSIC_APPS: &[(&str, &str)] = &[
    ("spotify", "Spotify"),
    ("applemusic", "Apple Music"),
    ("apple music", "Apple Music"),
    ("itunes", "iTunes"),
    ("youtube-music", "YouTube Music"),
    ("youtubemusic", "YouTube Music"),
    ("youtube music", "YouTube Music"),
    ("soundcloud", "SoundCloud"),
    ("tidal", "TIDAL"),
    ("deezer", "Deezer"),
    ("amazonmusic", "Amazon Music"),
    ("amazon music", "Amazon Music"),
    ("zunemusic", "Media Player"),
    ("foobar2000", "foobar2000"),
    ("musicbee", "MusicBee"),
    ("aimp", "AIMP"),
    ("winamp", "Winamp"),
    ("qobuz", "Qobuz"),
    ("plexamp", "Plexamp"),
    ("sh.cider", "Cider"),
    ("cider.exe", "Cider"),
    ("pandora", "Pandora"),
    ("audirvana", "Audirvana"),
    ("anghami", "Anghami"),
];

const BROWSERS: &[(&str, &str)] = &[
    ("msedge", "Edge"),
    ("chrome", "Chrome"),
    ("firefox", "Firefox"),
    ("brave", "Brave"),
    ("opera", "Opera"),
    ("vivaldi", "Vivaldi"),
    ("thebrowsercompany", "Arc"),
    ("zen", "Zen"),
];

pub fn app_kind(app_id: &str) -> AppKind {
    let l = app_id.to_ascii_lowercase();
    for (k, v) in PWAS {
        if l.contains(k) {
            return AppKind::Music(v);
        }
    }
    for (k, v) in MUSIC_APPS {
        if l.contains(k) {
            return AppKind::Music(v);
        }
    }
    for (k, v) in BROWSERS {
        if l.contains(k) {
            return AppKind::Browser(v);
        }
    }
    // Firefox registers with a 16-hex-digit hash of its install folder instead of a name
    if app_id.len() == 16 && app_id.chars().all(|c| c.is_ascii_hexdigit()) {
        return AppKind::Browser("Firefox");
    }
    AppKind::Other
}

/// A readable name for any app id (used for hidden sources in Settings).
pub fn app_name(app_id: &str) -> String {
    match app_kind(app_id) {
        AppKind::Music(n) | AppKind::Browser(n) => n.to_string(),
        AppKind::Other => {
            let base = app_id.rsplit(['\\', '!', '/']).next().unwrap_or(app_id);
            let base = base.trim_end_matches(".exe").trim_end_matches(".EXE");
            let base = base.split('_').next().unwrap_or(base);
            let base = base.rsplit('.').next().unwrap_or(base);
            let mut c = base.chars();
            match c.next() {
                Some(f) => f.to_uppercase().chain(c).collect(),
                None => "Media".into(),
            }
        }
    }
}

pub struct Meta<'a> {
    pub title: &'a str,
    pub artist: &'a str,
    pub album: &'a str,
    pub duration: f64,
}

const NOT_MUSIC: &[&str] = &[
    "tiktok", "instagram", "reels", "facebook", "twitter", "netflix", "prime video", "disney+", "twitch",
    "claude", "chatgpt", "gemini", "copilot", "whatsapp", "telegram", "discord", "zoom", "teams", "google meet",
];

const MUSIC_TITLE_MARKS: &[&str] = &[
    "official music video", "official video", "official audio", "official lyric", "lyric video", "lyrics video",
    "(lyrics)", "[lyrics]", "visualizer", "visualiser", "(audio)", "[audio]", "music video", "remix)", "remix]",
    " ft. ", " feat. ", "(feat.", "(ft.", "(prod.", "prod. by", "(live at", "(acoustic", "(instrumental", "slowed + reverb",
    "slowed & reverb", "sped up", "nightcore", "8d audio", "full album", "lofi hip hop", "lo-fi beats",
];

/// Without the browser companion: is this browser media music? Returns the label to show.
pub fn browser_is_music(m: &Meta) -> Option<&'static str> {
    let t = m.title.trim().to_lowercase();
    let a = m.artist.trim().to_lowercase();
    if t.is_empty() {
        return None;
    }
    if NOT_MUSIC.iter().any(|n| t.contains(n) || a.contains(n)) {
        return None;
    }
    // YouTube Music publishes the album; video sites never do
    if !m.album.trim().is_empty() && !a.is_empty() {
        return Some("YouTube Music");
    }
    // auto-generated artist channels and label channels
    if a.ends_with(" - topic") || a.contains("vevo") {
        return Some("YouTube");
    }
    if MUSIC_TITLE_MARKS.iter().any(|k| t.contains(k)) {
        return Some("YouTube");
    }
    // "Artist - Song" from the artist's own channel, song length
    if !a.is_empty() && t.starts_with(&a) && t[a.len()..].trim_start().starts_with(['-', '–', '—']) && (60.0..=600.0).contains(&m.duration) {
        return Some("YouTube");
    }
    None
}

/// With the browser companion: the page tells us the site and (on YouTube) the video category.
pub fn web_is_music(site: &str, category: &str) -> bool {
    match site {
        "ytmusic" | "soundcloud" | "spotify" | "applemusic" => true,
        "youtube" => category.eq_ignore_ascii_case("music"),
        _ => false,
    }
}

pub fn web_site_name(site: &str) -> &'static str {
    match site {
        "ytmusic" => "YouTube Music",
        "youtube" => "YouTube",
        "soundcloud" => "SoundCloud",
        "spotify" => "Spotify Web",
        "applemusic" => "Apple Music Web",
        _ => "Web",
    }
}

/// Titles compare loosely: case, spacing and the "(Official Video)"-style tails browsers add.
pub fn same_title(a: &str, b: &str) -> bool {
    fn norm(s: &str) -> String {
        s.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
    }
    let (a, b) = (norm(a), norm(b));
    !a.is_empty() && (a == b || a.starts_with(&b) || b.starts_with(&a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apps() {
        assert_eq!(app_kind("Spotify.exe"), AppKind::Music("Spotify"));
        assert_eq!(app_kind("AppleInc.AppleMusicWin_nzyj5cx40ttqa!App"), AppKind::Music("Apple Music"));
        assert_eq!(app_kind("Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic"), AppKind::Music("Media Player"));
        assert_eq!(app_kind("Chrome._crx_cinhimbnkkaeohfgghhklpknlkffjgod"), AppKind::Music("YouTube Music"));
        assert_eq!(app_kind("com.github.th-ch.youtube-music"), AppKind::Music("YouTube Music"));
        assert_eq!(app_kind("SoundCloud.exe"), AppKind::Music("SoundCloud"));
        assert_eq!(app_kind("Chrome"), AppKind::Browser("Chrome"));
        assert_eq!(app_kind("MSEdge"), AppKind::Browser("Edge"));
        assert_eq!(app_kind("308046B0AF4A39CB"), AppKind::Browser("Firefox"));
        assert_eq!(app_kind("Claude.exe"), AppKind::Other);
        assert_eq!(app_kind("com.squirrel.claude.claude"), AppKind::Other);
        assert_eq!(app_kind("ChatGPT.exe"), AppKind::Other);
        assert_eq!(app_kind("Discord.exe"), AppKind::Other);
        assert_eq!(app_kind("vlc.exe"), AppKind::Other);
        assert_eq!(app_kind("BytedancePte.Ltd.TikTok_6yccndn6064se!App"), AppKind::Other);
        assert_eq!(app_name("com.squirrel.claude.claude"), "Claude");
        assert_eq!(app_name("C:\\Apps\\vlc.exe"), "Vlc");
    }

    #[test]
    fn browser_media() {
        let m = |title, artist, album, duration| Meta { title, artist, album, duration };
        assert_eq!(browser_is_music(&m("Reminder", "The Weeknd", "Starboy", 220.0)), Some("YouTube Music"));
        assert_eq!(browser_is_music(&m("Blinding Lights", "The Weeknd - Topic", "", 200.0)), Some("YouTube"));
        assert_eq!(browser_is_music(&m("The Weeknd - Blinding Lights (Official Video)", "TheWeekndVEVO", "", 260.0)), Some("YouTube"));
        assert_eq!(browser_is_music(&m("Starboy (Official Audio)", "The Weeknd", "", 230.0)), Some("YouTube"));
        assert_eq!(browser_is_music(&m("The Weeknd - Reminder", "The Weeknd", "", 220.0)), Some("YouTube"));
        assert_eq!(browser_is_music(&m("How I edit my videos in 2026", "Some Creator", "", 900.0)), None);
        assert_eq!(browser_is_music(&m("Day in my life - vlog", "Day in my life", "", 2400.0)), None);
        assert_eq!(browser_is_music(&m("TikTok - Make Your Day", "", "", 30.0)), None);
        assert_eq!(browser_is_music(&m("Instagram", "", "", 15.0)), None);
        assert_eq!(browser_is_music(&m("", "", "", 0.0)), None);
        assert_eq!(browser_is_music(&m("Claude", "Voice mode", "", 0.0)), None);
    }

    #[test]
    fn web() {
        assert!(web_is_music("ytmusic", ""));
        assert!(web_is_music("soundcloud", ""));
        assert!(web_is_music("youtube", "Music"));
        assert!(!web_is_music("youtube", "Gaming"));
        assert!(!web_is_music("youtube", ""));
        assert!(!web_is_music("tiktok", ""));
    }

    #[test]
    fn titles() {
        assert!(same_title("Reminder", "reminder"));
        assert!(same_title("The Weeknd - Starboy (Official Video)", "The Weeknd - Starboy"));
        assert!(!same_title("Reminder", "Starboy"));
        assert!(!same_title("", ""));
    }
}
