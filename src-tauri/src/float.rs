//! The Settings window floats: no frame and no background, only its glass panels show. Windows
//! still hands a see-through window every click on it, so while it is open a light timer lets
//! clicks through wherever the pointer isn't over a panel (the page reports where its panels are).

use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Manager, WebviewWindow};

use crate::util::Lock;

/// The page's panels, in CSS pixels from the window's top-left corner: [x, y, width, height].
static PANELS: Mutex<Vec<[f64; 4]>> = Mutex::new(Vec::new());
/// A band this wide (CSS px) along the window's edges always takes the pointer, for resizing.
const EDGE: f64 = 6.0;

pub fn set_panels(rects: Vec<[f64; 4]>) {
    *PANELS.guard() = rects.into_iter().filter(|r| r.iter().all(|v| v.is_finite()) && r[2] > 0.0 && r[3] > 0.0).take(64).collect();
}

/// Whether the window takes the pointer at (x, y), in CSS px inside a window w × h.
fn takes(x: f64, y: f64, w: f64, h: f64, panels: &[[f64; 4]]) -> bool {
    if x < 0.0 || y < 0.0 || x >= w || y >= h {
        return false;
    }
    // until the page has said where its panels are, the whole window takes clicks
    panels.is_empty()
        || x < EDGE
        || y < EDGE
        || x >= w - EDGE
        || y >= h - EDGE
        || panels.iter().any(|r| x >= r[0] && x < r[0] + r[2] && y >= r[1] && y < r[1] + r[3])
}

fn pointer_on_panel(w: &WebviewWindow) -> bool {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut c = POINT::default();
    let (Ok(()), Ok(p), Ok(s), Ok(k)) = (unsafe { GetCursorPos(&mut c) }, w.inner_position(), w.inner_size(), w.scale_factor()) else {
        return true;
    };
    let (x, y) = ((c.x - p.x) as f64 / k, (c.y - p.y) as f64 / k);
    takes(x, y, s.width as f64 / k, s.height as f64 / k, &PANELS.guard())
}

/// Lets clicks through the empty parts of the Settings window until it closes.
pub fn watch(app: AppHandle) {
    PANELS.guard().clear();
    std::thread::spawn(move || {
        let mut through = false;
        loop {
            std::thread::sleep(Duration::from_millis(30));
            let Some(w) = app.get_webview_window("settings") else { return };
            let want = !pointer_on_panel(&w);
            if want != through && w.set_ignore_cursor_events(want).is_ok() {
                through = want;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicks_pass_through_only_between_panels() {
        let panels = [[20.0, 20.0, 560.0, 100.0], [20.0, 140.0, 560.0, 300.0]];
        assert!(takes(300.0, 60.0, 600.0, 800.0, &panels), "on a panel");
        assert!(!takes(300.0, 130.0, 600.0, 800.0, &panels), "in the gap between panels");
        assert!(!takes(300.0, 600.0, 600.0, 800.0, &panels), "below the last panel");
        assert!(takes(2.0, 600.0, 600.0, 800.0, &panels), "the edge band, for resizing");
        assert!(takes(300.0, 797.0, 600.0, 800.0, &panels), "the bottom edge band");
        assert!(!takes(-5.0, 60.0, 600.0, 800.0, &panels), "outside the window");
        assert!(takes(300.0, 600.0, 600.0, 800.0, &[]), "before the page reports, it all takes clicks");
    }
}
