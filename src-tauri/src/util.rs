//! Small shared helpers.

use std::sync::{Mutex, MutexGuard, PoisonError};

/// Locking that survives an earlier panic. A panic on some background thread while it held a
/// lock would otherwise poison it, and the next lock on the app's main thread (a command from a
/// window) would panic inside Windows' callback, which ends the whole process. The panic itself
/// is still written to the crash log (see `crash_log` in lib.rs); this only stops one failure from
/// taking everything else down with it.
pub trait Lock<T> {
    fn guard(&self) -> MutexGuard<'_, T>;
}

impl<T> Lock<T> for Mutex<T> {
    fn guard(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

use std::io::Write;
use std::path::PathBuf;

static LOG: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Where the log goes (%APPDATA%\app.skipdat.desktop\skipdat.log).
pub fn set_log_dir(dir: PathBuf) {
    *LOG.guard() = Some(dir.join("skipdat.log"));
}

/// One line in the log: panics, and the few decisions worth knowing about later. Kept small.
pub fn log(msg: &str) {
    let Some(path) = LOG.guard().clone() else { return };
    if std::fs::metadata(&path).map(|m| m.len() > 256 * 1024).unwrap_or(false) {
        let _ = std::fs::rename(&path, path.with_extension("old.log"));
    }
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "[{secs}] {msg}");
    }
}

/// Every panic, on any thread, is written to the log with where it happened.
pub fn log_panics() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current().name().unwrap_or("unnamed").to_string();
        let what = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        let at = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        log(&format!("PANIC in thread '{thread}' at {at}: {what}"));
        default(info);
    }));
}
