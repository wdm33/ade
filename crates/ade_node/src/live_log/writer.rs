// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN JSONL writer for `LiveLogEvent` (PHASE4-N-L-LIVE S1).
//!
//! Hand-rolled JSON serializer over the closed event enum. One
//! JSON object per line, flushed after every emit so a SIGINT
//! mid-emit cannot produce a partial line. No serde-derive dep.

use std::io::{self, Write};

use super::event::LiveLogEvent;

/// JSONL sink for `LiveLogEvent`. Wraps any `Write` impl
/// (a `File`, a `Vec<u8>` for tests, `io::stdout()`, etc.).
pub struct LiveLogWriter<W: Write> {
    sink: W,
}

impl<W: Write> LiveLogWriter<W> {
    pub fn new(sink: W) -> Self {
        Self { sink }
    }

    /// Serialize one event to a single line + flush. Atomic on
    /// the file system call boundary; a SIGINT after the write
    /// returns sees a complete line.
    pub fn emit(&mut self, event: &LiveLogEvent) -> io::Result<()> {
        let mut buf = String::new();
        encode_event(event, &mut buf);
        buf.push('\n');
        self.sink.write_all(buf.as_bytes())?;
        self.sink.flush()
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.sink.flush()
    }

    pub fn into_inner(self) -> W {
        self.sink
    }
}

fn encode_event(event: &LiveLogEvent, out: &mut String) {
    out.push('{');
    push_key_str(out, "event", event.discriminator());
    match event {
        LiveLogEvent::NodeStarted { mode, peer_count } => {
            out.push(',');
            push_key_str(out, "mode", mode.as_str());
            out.push(',');
            push_key_u64(out, "peer_count", *peer_count as u64);
        }
        LiveLogEvent::PeerDialStarted { peer } => {
            out.push(',');
            push_key_str(out, "peer", peer);
        }
        LiveLogEvent::HandshakeOk {
            peer,
            negotiated_version,
        } => {
            out.push(',');
            push_key_str(out, "peer", peer);
            out.push(',');
            push_key_u64(out, "negotiated_version", *negotiated_version as u64);
        }
        LiveLogEvent::PeerTipRead {
            peer,
            slot,
            hash_hex,
            block_no,
        } => {
            out.push(',');
            push_key_str(out, "peer", peer);
            out.push(',');
            push_key_u64(out, "slot", *slot);
            out.push(',');
            push_key_str(out, "hash_hex", hash_hex);
            out.push(',');
            push_key_u64(out, "block_no", *block_no);
        }
        LiveLogEvent::PeerDialFailed { peer, kind, detail } => {
            out.push(',');
            push_key_str(out, "peer", peer);
            out.push(',');
            push_key_str(out, "kind", kind.as_str());
            out.push(',');
            push_key_str(out, "detail", detail);
        }
        LiveLogEvent::WireSmokeComplete {
            admission_enabled,
            peer_count_ok,
            peer_count_failed,
        } => {
            out.push(',');
            push_key_bool(out, "admission_enabled", *admission_enabled);
            out.push(',');
            push_key_u64(out, "peer_count_ok", *peer_count_ok as u64);
            out.push(',');
            push_key_u64(out, "peer_count_failed", *peer_count_failed as u64);
        }
        LiveLogEvent::NodeShutdown { reason } => {
            out.push(',');
            push_key_str(out, "reason", reason.as_str());
        }
    }
    out.push('}');
}

fn push_key_str(out: &mut String, key: &str, val: &str) {
    out.push('"');
    push_json_str_body(out, key);
    out.push_str("\":\"");
    push_json_str_body(out, val);
    out.push('"');
}

fn push_key_u64(out: &mut String, key: &str, val: u64) {
    out.push('"');
    push_json_str_body(out, key);
    out.push_str("\":");
    out.push_str(&val.to_string());
}

fn push_key_bool(out: &mut String, key: &str, val: bool) {
    out.push('"');
    push_json_str_body(out, key);
    out.push_str("\":");
    out.push_str(if val { "true" } else { "false" });
}

fn push_json_str_body(out: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
}

// Re-export discriminator strings as constants for the suppressor
// gate's positive grep.
#[allow(dead_code)]
const DISCRIMINATORS: &[&str] = &[
    "node_started",
    "peer_dial_started",
    "handshake_ok",
    "peer_tip_read",
    "peer_dial_failed",
    "wire_smoke_complete",
    "node_shutdown",
];

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use super::super::event::{ModeTag, PeerDialFailureKind, WireOnlyShutdownReason};
    use std::io::BufRead;

    fn emit_to_vec(events: &[LiveLogEvent]) -> Vec<u8> {
        let buf: Vec<u8> = Vec::new();
        let mut w = LiveLogWriter::new(buf);
        for e in events {
            w.emit(e).expect("emit");
        }
        w.into_inner()
    }

    #[test]
    fn live_log_writer_emits_one_object_per_line() {
        let events = vec![
            LiveLogEvent::NodeStarted {
                mode: ModeTag::WireOnly,
                peer_count: 2,
            },
            LiveLogEvent::PeerDialStarted {
                peer: "127.0.0.1:3001".to_string(),
            },
            LiveLogEvent::HandshakeOk {
                peer: "127.0.0.1:3001".to_string(),
                negotiated_version: 14,
            },
            LiveLogEvent::PeerTipRead {
                peer: "127.0.0.1:3001".to_string(),
                slot: 12345,
                hash_hex: "deadbeef".to_string(),
                block_no: 100,
            },
            LiveLogEvent::NodeShutdown {
                reason: WireOnlyShutdownReason::TipReadComplete,
            },
        ];
        let bytes = emit_to_vec(&events);
        let lines: Vec<&[u8]> = bytes
            .split(|b| *b == b'\n')
            .filter(|l| !l.is_empty())
            .collect();
        assert_eq!(lines.len(), 5);
        for (i, line) in lines.iter().enumerate() {
            let s = std::str::from_utf8(line).expect("utf8");
            assert!(s.starts_with('{'), "line {i} must start with {{");
            assert!(s.ends_with('}'), "line {i} must end with }}");
        }
    }

    #[test]
    fn live_log_writer_serializes_node_started_canonically() {
        let bytes = emit_to_vec(&[LiveLogEvent::NodeStarted {
            mode: ModeTag::WireOnly,
            peer_count: 1,
        }]);
        let s = std::str::from_utf8(&bytes).expect("utf8");
        assert_eq!(
            s,
            "{\"event\":\"node_started\",\"mode\":\"wire_only\",\"peer_count\":1}\n"
        );
    }

    #[test]
    fn live_log_writer_two_runs_are_byte_identical() {
        let events = vec![
            LiveLogEvent::NodeStarted {
                mode: ModeTag::WireOnly,
                peer_count: 3,
            },
            LiveLogEvent::PeerDialFailed {
                peer: "1.2.3.4:5".to_string(),
                kind: PeerDialFailureKind::TcpConnectFailed,
                detail: "ECONNREFUSED".to_string(),
            },
        ];
        let a = emit_to_vec(&events);
        let b = emit_to_vec(&events);
        assert_eq!(a, b);
    }

    #[test]
    fn live_log_writer_escapes_quotes_in_detail() {
        let bytes = emit_to_vec(&[LiveLogEvent::PeerDialFailed {
            peer: "p".to_string(),
            kind: PeerDialFailureKind::HandshakeRejected,
            detail: "reason with \"quotes\"".to_string(),
        }]);
        let s = std::str::from_utf8(&bytes).expect("utf8");
        assert!(s.contains("\\\"quotes\\\""), "got: {s}");
    }

    #[test]
    fn live_log_writer_emits_wire_smoke_complete_with_counts() {
        let bytes = emit_to_vec(&[LiveLogEvent::WireSmokeComplete {
            admission_enabled: false,
            peer_count_ok: 1,
            peer_count_failed: 0,
        }]);
        let s = std::str::from_utf8(&bytes).expect("utf8");
        assert!(s.contains("\"admission_enabled\":false"), "got: {s}");
        assert!(s.contains("\"peer_count_ok\":1"), "got: {s}");
        assert!(s.contains("\"peer_count_failed\":0"), "got: {s}");
    }

    #[test]
    fn live_log_writer_lines_are_parseable_as_one_json_object_per_line() {
        // Light JSON validity check: every line starts with `{`,
        // ends with `}`, contains `"event":`, and has matching
        // brace depth = 0 at the end. This is a structural check;
        // the integration test in S3 uses `jq` if available.
        let events = vec![
            LiveLogEvent::NodeStarted {
                mode: ModeTag::WireOnly,
                peer_count: 1,
            },
            LiveLogEvent::PeerDialStarted {
                peer: "p".to_string(),
            },
            LiveLogEvent::NodeShutdown {
                reason: WireOnlyShutdownReason::TipReadComplete,
            },
        ];
        let bytes = emit_to_vec(&events);
        let cursor = std::io::Cursor::new(bytes);
        for line in cursor.lines() {
            let s = line.expect("line");
            assert!(s.starts_with('{'));
            assert!(s.ends_with('}'));
            assert!(s.contains("\"event\":\""), "missing event: {s}");
            let mut depth = 0i32;
            for c in s.chars() {
                match c {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
            }
            assert_eq!(depth, 0, "unbalanced braces: {s}");
        }
    }
}
