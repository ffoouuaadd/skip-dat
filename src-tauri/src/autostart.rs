//! "Start with Windows": a value under HKCU\...\Run, the same place Task Manager's Startup tab reads.

use windows::core::{w, PCWSTR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegGetValueW, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE,
    REG_SZ, RRF_RT_REG_SZ,
};

const RUN: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const NAME: PCWSTR = w!("Skip Dat");
/// the name used before the app was called Skip Dat
const OLD_NAME: PCWSTR = w!("Liquid Queue");

pub fn enabled() -> bool {
    unsafe { RegGetValueW(HKEY_CURRENT_USER, RUN, NAME, RRF_RT_REG_SZ, None, None, None).is_ok() }
}

pub fn set(on: bool) -> Result<(), String> {
    unsafe {
        let mut key = HKEY::default();
        RegOpenKeyExW(HKEY_CURRENT_USER, RUN, Some(0), KEY_SET_VALUE, &mut key).ok().map_err(|e| e.to_string())?;
        let res = if on {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let wide: Vec<u16> = format!("\"{}\"", exe.display()).encode_utf16().chain(Some(0)).collect();
            let bytes = std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2);
            RegSetValueExW(key, NAME, Some(0), REG_SZ, Some(bytes))
        } else {
            RegDeleteValueW(key, NAME)
        };
        let _ = RegDeleteValueW(key, OLD_NAME);
        let _ = RegCloseKey(key);
        if on { res.ok().map_err(|e| e.to_string()) } else { Ok(()) }
    }
}
