// Core Contract:
// - RED shell: write-only operational telemetry (I/O). NEVER influences any authoritative
//   state, transition, replay, or consensus decision.
// - Must never panic or block the node: every failure (open / lock / write) is silently dropped.

//! Operational follow-progress log — a KNOWN, default file destination for the human-readable
//! follow / epoch-boundary progress lines, so an operator (or a test) can SEE the node working
//! instead of depending on how the process's stdout/stderr happens to be redirected at launch
//! (which is fragile and was the source of a "logs went to the wrong file" tangle).
//!
//! Defaults to `<data-dir>/node.log`. Lines ALSO go to stderr, so the console still shows them;
//! the file just guarantees a stable place to read them from. This is deliberately separate from
//! the structured [`crate::live_log::LiveLogWriter`] (a closed-vocabulary event stream) — this is
//! the free-text "what is the node doing right now" narrative (tip advancing, behind-by-N,
//! boundary crossings).

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

static OPS_LOG: OnceLock<Mutex<File>> = OnceLock::new();

/// Point the operational log at `path` (typically `<data-dir>/node.log`). Creates the parent
/// directory if needed. Idempotent — the first successful call wins; later calls are ignored. A
/// failure to open the file is NON-FATAL: the node keeps logging to stderr (telemetry must never
/// block operation), so this is best-effort.
pub fn init_ops_log(path: &Path) {
    if OPS_LOG.get().is_some() {
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(f) = File::options().create(true).append(true).open(path) {
        let _ = OPS_LOG.set(Mutex::new(f));
    }
}

/// Append one already-formatted line to the node log file, if one is configured. Never panics: a
/// poisoned lock or a write error is silently dropped (telemetry must never take the node down).
pub fn ops_log_line(line: &str) {
    if let Some(m) = OPS_LOG.get() {
        if let Ok(mut f) = m.lock() {
            let _ = writeln!(f, "{line}");
            let _ = f.flush();
        }
    }
}

/// Emit a human-readable operational line to BOTH stderr (console) and `<data-dir>/node.log` (if
/// configured via [`init_ops_log`]). Use this for the follow / boundary progress narrative so the
/// output lands in a known place regardless of how the process's stderr is redirected.
#[macro_export]
macro_rules! node_log {
    ($($arg:tt)*) => {{
        let __ade_ops_line = format!($($arg)*);
        eprintln!("{}", __ade_ops_line);
        $crate::ops_log::ops_log_line(&__ade_ops_line);
    }};
}
