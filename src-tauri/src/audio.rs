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
use windows::Win32::Storage::Packaging::Appx::GetApplicationUserModelId;
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
        let aumid = if GetApplicationUserModelId(h, &mut alen, Some(PWSTR(abuf.as_mut_ptr()))).is_ok() {
            String::from_utf16_lossy(&abuf[..(alen as usize).saturating_sub(1)])
        } else {
            String::new()
        };
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
