//! Muting one app for a moment, through Windows' per-app volume (the Volume mixer), so pressing
//! "next" several times to reach a song further down the queue makes no sound in between.
//!
//! Only sessions that were audible get muted, and exactly those are unmuted again: a mute the
//! person set themselves is left alone. The app ids being muted are written to a small file
//! first, so if Skip Dat were closed in the middle, it unmutes them the next time it starts.

use std::path::PathBuf;
use std::sync::Mutex;

use windows::core::{Interface, PWSTR};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Media::Audio::{
    eRender, IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
};
use windows::Win32::Storage::Packaging::Appx::{GetApplicationUserModelId, GetPackageFamilyName};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};

use crate::util::Lock;

static MARKER: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Where the "currently muted by Skip Dat" note lives; unmutes anything left from last time.
pub fn init(dir: PathBuf) {
    let path = dir.join("muted.json");
    if let Ok(s) = std::fs::read_to_string(&path) {
        if let Ok(ids) = serde_json::from_str::<Vec<String>>(&s) {
            for id in ids {
                for v in sessions_of(&id) {
                    unsafe {
                        let _ = v.SetMute(false, std::ptr::null());
                    }
                }
            }
        }
        let _ = std::fs::remove_file(&path);
    }
    *MARKER.guard() = Some(path);
}

fn process_names(pid: u32) -> (String, String) {
    unsafe {
        let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return (String::new(), String::new()) };
        let mut buf = [0u16; 520];
        let mut len = buf.len() as u32;
        let exe = if QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok() {
            let full = String::from_utf16_lossy(&buf[..len as usize]);
            full.rsplit(['\\', '/']).next().unwrap_or("").to_string()
        } else {
            String::new()
        };
        let mut abuf = [0u16; 260];
        let mut alen = abuf.len() as u32;
        let mut aumid = if GetApplicationUserModelId(h, &mut alen, Some(PWSTR(abuf.as_mut_ptr()))).is_ok() {
            String::from_utf16_lossy(&abuf[..(alen as usize).saturating_sub(1)])
        } else {
            String::new()
        };
        if aumid.is_empty() {
            // a Store app's helper process (often the one making the sound) has its package but
            // no app id: "<family>!" still matches the app's package family in plan::same_app
            let mut fbuf = [0u16; 260];
            let mut flen = fbuf.len() as u32;
            if GetPackageFamilyName(h, &mut flen, Some(PWSTR(fbuf.as_mut_ptr()))).is_ok() {
                aumid = format!("{}!", String::from_utf16_lossy(&fbuf[..(flen as usize).saturating_sub(1)]));
            }
        }
        let _ = CloseHandle(h);
        (exe, aumid)
    }
}

/// The volume controls of every playback session (on every output device) of this app.
fn sessions_of(app_id: &str) -> Vec<ISimpleAudioVolume> {
    let mut out = Vec::new();
    unsafe {
        let Ok(en) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) else { return out };
        let Ok(devices) = en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) else { return out };
        for i in 0..devices.GetCount().unwrap_or(0) {
            let Ok(dev) = devices.Item(i) else { continue };
            let Ok(mgr) = dev.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) else { continue };
            let Ok(list) = mgr.GetSessionEnumerator() else { continue };
            for j in 0..list.GetCount().unwrap_or(0) {
                let Ok(ctl) = list.GetSession(j) else { continue };
                let Ok(ctl2) = ctl.cast::<IAudioSessionControl2>() else { continue };
                let pid = ctl2.GetProcessId().unwrap_or(0);
                if pid == 0 {
                    continue;
                }
                let (exe, aumid) = process_names(pid);
                if crate::plan::same_app(app_id, &exe, &aumid) {
                    if let Ok(v) = ctl.cast::<ISimpleAudioVolume>() {
                        out.push(v);
                    }
                }
            }
        }
    }
    out
}

/// Sets this app's own volume in the Volume mixer (each of its sessions) to `to(current)`.
/// The new level, or None when the app has no audio session (it hasn't made a sound yet).
pub fn nudge(app_id: &str, to: impl Fn(f32) -> f32) -> Option<f32> {
    let mut out = None;
    for v in sessions_of(app_id) {
        unsafe {
            let Ok(cur) = v.GetMasterVolume() else { continue };
            let n = to(cur);
            if v.SetMasterVolume(n, std::ptr::null()).is_ok() {
                out.get_or_insert(n);
            }
        }
    }
    out
}

/// Muted for as long as this lives; dropping it brings the sound back.
pub struct Muted {
    vols: Vec<ISimpleAudioVolume>,
}

pub fn mute(app_id: &str) -> Muted {
    if let Some(p) = MARKER.guard().as_ref() {
        let _ = std::fs::write(p, serde_json::json!([app_id]).to_string());
    }
    let mut vols = Vec::new();
    for v in sessions_of(app_id) {
        unsafe {
            // leave a mute the person set alone
            if v.GetMute().map(|b| b.as_bool()).unwrap_or(true) {
                continue;
            }
            if v.SetMute(true, std::ptr::null()).is_ok() {
                vols.push(v);
            }
        }
    }
    Muted { vols }
}

impl Drop for Muted {
    fn drop(&mut self) {
        for v in self.vols.drain(..) {
            unsafe {
                let _ = v.SetMute(false, std::ptr::null());
            }
        }
        if let Some(p) = MARKER.guard().as_ref() {
            let _ = std::fs::remove_file(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Live: every audio session on this PC with its process and app id, and whether `same_app`
    /// matches it to SKIPDAT_LIVE_APP (not run by default).
    #[test]
    #[ignore]
    fn live_list_sessions() {
        unsafe {
            let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);
            let want = std::env::var("SKIPDAT_LIVE_APP").unwrap_or_else(|_| "SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify".into());
            let en = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL).unwrap();
            let devices = en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE).unwrap();
            for i in 0..devices.GetCount().unwrap() {
                let mgr = devices.Item(i).unwrap().Activate::<IAudioSessionManager2>(CLSCTX_ALL, None).unwrap();
                let list = mgr.GetSessionEnumerator().unwrap();
                for j in 0..list.GetCount().unwrap() {
                    let ctl = list.GetSession(j).unwrap();
                    let pid = ctl.cast::<IAudioSessionControl2>().unwrap().GetProcessId().unwrap_or(0);
                    let (exe, aumid) = process_names(pid);
                    println!("device {i} session {j}: pid {pid} exe {exe:?} aumid {aumid:?} matches {want}: {}", crate::plan::same_app(&want, &exe, &aumid));
                }
            }
        }
    }

    /// Live, against this PC's real Volume mixer (not run by default):
    ///   cargo test --lib live_ -- --ignored --nocapture
    /// Uses the Spotify desktop app (or SKIPDAT_LIVE_APP) and puts its volume back afterwards.
    #[test]
    #[ignore]
    fn live_nudge_moves_that_app_five_percent_and_restores() {
        unsafe {
            let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);
        }
        let id = std::env::var("SKIPDAT_LIVE_APP").unwrap_or_else(|_| "SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify".into());
        let before: Vec<f32> = sessions_of(&id).iter().map(|v| unsafe { v.GetMasterVolume().unwrap() }).collect();
        assert!(!before.is_empty(), "{id} has no audio session: play something in it first");
        // start from a level with room both ways
        nudge(&id, |_| 0.5).unwrap();
        let up = nudge(&id, |v| crate::extras::volume_after(v, 1)).unwrap();
        let down = nudge(&id, |v| crate::extras::volume_after(v, -2)).unwrap();
        // the mixer itself reports the new level, not just our return value
        let seen: Vec<f32> = sessions_of(&id).iter().map(|v| unsafe { v.GetMasterVolume().unwrap() }).collect();
        for (v, b) in sessions_of(&id).iter().zip(&before) {
            unsafe { v.SetMasterVolume(*b, std::ptr::null()).unwrap() };
        }
        let restored: Vec<f32> = sessions_of(&id).iter().map(|v| unsafe { v.GetMasterVolume().unwrap() }).collect();
        println!("{id}: before {before:?}, +1 notch {up}, -2 notches {down}, mixer saw {seen:?}, restored {restored:?}");
        assert!((up - 0.55).abs() < 0.001 && (down - 0.45).abs() < 0.001);
        assert!(seen.iter().all(|s| (s - 0.45).abs() < 0.001), "every session of the app moved");
        assert!(restored.iter().zip(&before).all(|(r, b)| (r - b).abs() < 0.001), "volume put back");
    }
}
