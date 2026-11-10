//! How to get from the song playing now to the one that was picked, for each kind of player.
//! Pure decisions (no Windows calls), so every edge case is unit-tested.
//!
//! The rule: go straight to the song when the player has a way to do that and it keeps the queue
//! in the same order. Otherwise press next/previous the right number of times with that player's
//! sound muted, and unmute once the picked song starts. Never press a button the player says it
//! doesn't have.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// a Windows media session (Spotify, Apple Music, other apps, browsers without the companion)
    App,
    /// a browser tab reached through the companion
    Tab,
}

#[derive(Clone, Copy, Debug)]
pub struct Caps {
    pub kind: Kind,
    /// the page can play an entry of its list directly
    pub can_jump: bool,
    /// the picked song has a place in that list
    pub in_list: bool,
    /// a direct jump on this site was seen to reorder the queue (shuffle): don't use it
    pub jump_reorders: bool,
    /// the player accepts "previous" / "next"
    pub can_prev: bool,
    pub can_next: bool,
    /// seconds into the current song (most players restart it on the first "previous" after ~3 s)
    pub position: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Route {
    /// the song picked is the one playing: nothing to move (play it if paused)
    Stay,
    /// the page plays that entry of its list straight away
    Jump,
    /// a tab without a list (one YouTube video): go back to a video it played before
    TabBack { count: u32 },
    /// press next (count > 0) or previous (count < 0) in the tab, with the tab muted meanwhile
    TabSkip { count: i32 },
    /// press next / previous on the app, with the app's sound muted meanwhile
    AppSkip { count: i32 },
    /// the player can't go there (no previous / next button): leave it alone
    Cannot,
}

/// More presses than this are never sent: a list that changed under us can't run away.
pub const MAX_PRESSES: i32 = 40;

pub fn route(rel: i32, c: &Caps) -> Route {
    if rel == 0 {
        return Route::Stay;
    }
    if (rel < 0 && !c.can_prev) || (rel > 0 && !c.can_next) {
        return Route::Cannot;
    }
    match c.kind {
        Kind::Tab => {
            if c.can_jump && c.in_list && !c.jump_reorders {
                Route::Jump
            } else if rel < 0 && !c.in_list {
                Route::TabBack { count: rel.unsigned_abs().min(MAX_PRESSES as u32) }
            } else {
                Route::TabSkip { count: rel.clamp(-MAX_PRESSES, MAX_PRESSES) }
            }
        }
        Kind::App => {
            // the first "previous" a few seconds into a song only takes it back to its start
            let extra = if rel < 0 && c.position > 3.5 { 1 } else { 0 };
            Route::AppSkip { count: (rel - extra).clamp(-MAX_PRESSES, MAX_PRESSES) }
        }
    }
}

fn norm(s: &str) -> String {
    s.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// After a jump: is what comes next still in the order it was before, minus the songs passed?
/// `before`: the songs that came after the picked one; `after`: what comes after it now. Lists
/// that grow (mixes) or were cut short are compared over the part both have.
pub fn order_kept(before: &[String], after: &[String]) -> bool {
    let n = before.len().min(after.len()).min(10);
    before[..n].iter().zip(&after[..n]).all(|(a, b)| norm(a) == norm(b))
}

// ---------- Spotify (with the optional connection): straight to the song, like an ender pearl ----------

/// Can Spotify start this context at a given song? Only playlists and albums take an "offset";
/// Liked Songs, artists, radio and podcasts don't (Spotify could start them from the top instead).
pub fn spotify_offset_ok(context: &str) -> bool {
    context.starts_with("spotify:playlist:") || context.starts_with("spotify:album:")
}

/// What to ask Spotify to play when the song can't be started inside its playlist: the song, then
/// everything listed after it, in the order shown (`items`: (rel, URI) for each song in the list).
/// At most 50 songs.
pub fn spotify_list(items: &[(i32, Option<String>)], target_rel: i32) -> Vec<String> {
    let Some(first) = items.iter().find(|(r, _)| *r == target_rel).and_then(|(_, u)| u.clone()) else { return Vec::new() };
    let mut after: Vec<&(i32, Option<String>)> = items.iter().filter(|(r, u)| *r > target_rel && u.is_some()).collect();
    after.sort_by_key(|(r, _)| *r);
    std::iter::once(first).chain(after.into_iter().filter_map(|(_, u)| u.clone())).take(50).collect()
}

/// Does an audio session's process belong to the media session `app_id`? Desktop apps give
/// their file name ("Spotify.exe"), Store apps their app id ("AppleInc.AppleMusicWin_…!App").
pub fn same_app(app_id: &str, exe: &str, aumid: &str) -> bool {
    let id = app_id.trim().to_lowercase();
    if id.is_empty() {
        return false;
    }
    let exe = exe.to_lowercase();
    let aumid = aumid.to_lowercase();
    if !aumid.is_empty() && aumid == id {
        return true;
    }
    if id.contains('!') {
        // a Store app: its package family, before the "!"
        let family = id.split('!').next().unwrap_or("");
        return !aumid.is_empty() && aumid.split('!').next() == Some(family);
    }
    !exe.is_empty() && (exe == id || exe.trim_end_matches(".exe") == id.trim_end_matches(".exe"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(kind: Kind) -> Caps {
        Caps { kind, can_jump: true, in_list: true, jump_reorders: false, can_prev: true, can_next: true, position: 10.0 }
    }

    #[test]
    fn same_song_stays() {
        assert_eq!(route(0, &caps(Kind::Tab)), Route::Stay);
        assert_eq!(route(0, &Caps { can_prev: false, can_next: false, ..caps(Kind::App) }), Route::Stay);
    }

    #[test]
    fn lists_jump_straight_there() {
        assert_eq!(route(5, &caps(Kind::Tab)), Route::Jump);
        assert_eq!(route(-3, &caps(Kind::Tab)), Route::Jump);
    }

    #[test]
    fn reordering_sites_skip_muted_instead() {
        let c = Caps { jump_reorders: true, ..caps(Kind::Tab) };
        assert_eq!(route(5, &c), Route::TabSkip { count: 5 });
        assert_eq!(route(-2, &c), Route::TabSkip { count: -2 });
    }

    #[test]
    fn a_single_video_goes_back_through_what_it_played() {
        let c = Caps { can_jump: true, in_list: false, ..caps(Kind::Tab) };
        assert_eq!(route(-2, &c), Route::TabBack { count: 2 });
        // its autoplay song ahead is reached with the page's own Next
        assert_eq!(route(1, &Caps { can_jump: false, ..c }), Route::TabSkip { count: 1 });
    }

    #[test]
    fn apps_skip_muted_and_count_the_restart() {
        assert_eq!(route(4, &caps(Kind::App)), Route::AppSkip { count: 4 });
        // 10 s into the song: one extra press, as the first only restarts it
        assert_eq!(route(-1, &caps(Kind::App)), Route::AppSkip { count: -2 });
        assert_eq!(route(-1, &Caps { position: 1.0, ..caps(Kind::App) }), Route::AppSkip { count: -1 });
    }

    #[test]
    fn players_without_the_button_are_left_alone() {
        // the first song of a list, or an app that has no "previous"
        assert_eq!(route(-1, &Caps { can_prev: false, ..caps(Kind::App) }), Route::Cannot);
        assert_eq!(route(-1, &Caps { can_prev: false, ..caps(Kind::Tab) }), Route::Cannot);
        assert_eq!(route(2, &Caps { can_next: false, ..caps(Kind::App) }), Route::Cannot);
        assert_eq!(route(2, &Caps { can_prev: false, ..caps(Kind::App) }), Route::AppSkip { count: 2 });
    }

    #[test]
    fn presses_are_capped() {
        assert_eq!(route(500, &caps(Kind::App)), Route::AppSkip { count: MAX_PRESSES });
        assert_eq!(route(-500, &Caps { in_list: false, ..caps(Kind::Tab) }), Route::TabBack { count: MAX_PRESSES as u32 });
    }

    #[test]
    fn order_check() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        // jumped from A to C in A B C D E F: what follows C is still D E F
        assert!(order_kept(&s(&["D", "E", "F"]), &s(&["D", "E", "F"])));
        // a mix that grew meanwhile
        assert!(order_kept(&s(&["D", "E"]), &s(&["d", "E", "F", "G"])));
        // the site reshuffled what was left
        assert!(!order_kept(&s(&["D", "E", "F"]), &s(&["F", "D", "E"])));
        // nothing to compare: no evidence of reordering
        assert!(order_kept(&s(&[]), &s(&["X"])));
    }

    #[test]
    fn spotify_starts_playlists_and_albums_at_the_song() {
        assert!(spotify_offset_ok("spotify:playlist:37i9dQZF1DXcBWIGoYBM5M"));
        assert!(spotify_offset_ok("spotify:album:4aawyAB9vmqN3uQ7FjRGTy"));
        // these would start from the top: use the list instead
        assert!(!spotify_offset_ok("spotify:user:abc:collection"));
        assert!(!spotify_offset_ok("spotify:collection:tracks"));
        assert!(!spotify_offset_ok("spotify:artist:0TnOYISbd1XYRBk9myaseg"));
        assert!(!spotify_offset_ok(""));
    }

    #[test]
    fn spotify_list_keeps_the_order_shown() {
        let u = |x: &str| Some(format!("spotify:track:{x}"));
        let items = vec![(-2, u("p2")), (-1, u("p1")), (0, u("now")), (1, u("a")), (2, None), (3, u("c")), (4, u("d"))];
        // ahead: the song, then what was listed after it
        assert_eq!(spotify_list(&items, 3), vec![u("c").unwrap(), u("d").unwrap()]);
        // back: the earlier song, then the one that was playing and the queue (songs without a URI left out)
        assert_eq!(spotify_list(&items, -1), vec![u("p1").unwrap(), u("now").unwrap(), u("a").unwrap(), u("c").unwrap(), u("d").unwrap()]);
        // a song without a URI can't be played this way
        assert!(spotify_list(&items, 2).is_empty());
        let long: Vec<(i32, Option<String>)> = (0..80).map(|i| (i, u(&i.to_string()))).collect();
        assert_eq!(spotify_list(&long, 1).len(), 50);
    }

    #[test]
    fn matching_apps_for_muting() {
        assert!(same_app("Spotify.exe", "Spotify.exe", ""));
        assert!(same_app("Spotify.exe", "spotify.exe", ""));
        assert!(same_app("Spotify", "Spotify.exe", ""));
        assert!(same_app("AppleInc.AppleMusicWin_nzyj5cx40ttqa!App", "AppleMusic.exe", "AppleInc.AppleMusicWin_nzyj5cx40ttqa!App"));
        assert!(!same_app("AppleInc.AppleMusicWin_nzyj5cx40ttqa!App", "AppleMusic.exe", ""));
        // a Store app's helper process: its package family, no app id (audio.rs adds the "!")
        assert!(same_app("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify", "Spotify.exe", "SpotifyAB.SpotifyMusic_zpdnekdrzrea0!"));
        assert!(!same_app("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify", "Spotify.exe", "OtherCo.Other_123!"));
        assert!(same_app("Spotify.exe", "Spotify.exe", "OtherCo.Other_123!"), "a desktop app still matches by its exe");
        assert!(!same_app("Spotify.exe", "chrome.exe", ""));
        assert!(!same_app("", "Spotify.exe", ""));
    }
}
