// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN byte-deterministic JSONL writer for [`NodeSchedEvent`]
//! (PHASE4-N-F-G-J S1, `CN-NODE-04`).
//!
//! Hand-rolled JSON serializer over the closed sched-event enum, mirroring
//! `live_log/writer.rs`. One JSON object per line, flushed after every emit so a
//! SIGINT mid-emit cannot produce a partial line. No serde-derive dep. The
//! per-variant body match is exhaustive: a new [`NodeSchedEvent`] variant is a
//! compile error here until wired (fail-closed-on-unknown, never a silent drop).

use std::io::{self, Write};

use super::sched_event::NodeSchedEvent;

/// JSONL sink for [`NodeSchedEvent`]. Wraps any `Write` impl (an `io::Stderr`
/// on the binary node path, a `Vec<u8>` for tests, etc.).
pub struct NodeSchedLogWriter<W: Write> {
    sink: W,
}

impl<W: Write> NodeSchedLogWriter<W> {
    pub fn new(sink: W) -> Self {
        Self { sink }
    }

    /// Serialize one event to a single line + flush. A SIGINT after the write
    /// returns sees a complete line.
    pub fn emit(&mut self, event: &NodeSchedEvent) -> io::Result<()> {
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

/// The emit-only sink the relay loop holds. A thin trait over
/// [`NodeSchedLogWriter`] so the loop's emitter param is a simple
/// `Option<&mut dyn NodeSchedSink>` — tests pass `None`, the binary passes a
/// `&mut NodeSchedLogWriter<io::Stderr>`. Best-effort: an emit error is swallowed
/// (a diagnostic log must never alter the loop's scheduling / control flow).
pub trait NodeSchedSink {
    fn record(&mut self, event: &NodeSchedEvent);
}

impl<W: Write> NodeSchedSink for NodeSchedLogWriter<W> {
    fn record(&mut self, event: &NodeSchedEvent) {
        let _ = self.emit(event);
    }
}

fn encode_event(event: &NodeSchedEvent, out: &mut String) {
    out.push('{');
    push_key_str(out, "event", event.discriminator());
    match event {
        NodeSchedEvent::FeedUnavailable { reason } => {
            out.push(',');
            push_key_str(out, "reason", reason.as_str());
        }
        NodeSchedEvent::ForgeTickConsidered => {}
        NodeSchedEvent::ForgeTickSkipped { reason } => {
            out.push(',');
            push_key_str(out, "reason", reason.as_str());
        }
        NodeSchedEvent::ForgeAttempted => {}
        NodeSchedEvent::ForgeResult { outcome } => {
            out.push(',');
            push_key_str(out, "outcome", outcome.as_str());
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

// Re-export discriminator strings as constants for the emit-only gate's
// positive grep (mirrors live_log/writer.rs::DISCRIMINATORS).
#[allow(dead_code)]
const SCHED_DISCRIMINATORS: &[&str] = &[
    "feed_unavailable",
    "forge_tick_considered",
    "forge_tick_skipped",
    "forge_attempted",
    "forge_result",
];

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::super::sched_event::{FeedReason, ForgeOutcome};
    use super::*;

    fn emit_to_vec(events: &[NodeSchedEvent]) -> Vec<u8> {
        let buf: Vec<u8> = Vec::new();
        let mut w = NodeSchedLogWriter::new(buf);
        for e in events {
            w.emit(e).expect("emit");
        }
        w.into_inner()
    }

    #[test]
    fn sched_writer_emits_one_object_per_line() {
        let events = vec![
            NodeSchedEvent::FeedUnavailable {
                reason: FeedReason::CleanEmpty,
            },
            NodeSchedEvent::ForgeTickConsidered,
            NodeSchedEvent::ForgeAttempted,
            NodeSchedEvent::ForgeResult {
                outcome: ForgeOutcome::Succeeded,
            },
        ];
        let bytes = emit_to_vec(&events);
        let lines: Vec<&[u8]> = bytes
            .split(|b| *b == b'\n')
            .filter(|l| !l.is_empty())
            .collect();
        assert_eq!(lines.len(), 4);
        for line in &lines {
            let s = std::str::from_utf8(line).expect("utf8");
            assert!(s.starts_with('{'));
            assert!(s.ends_with('}'));
            assert!(s.contains("\"event\":\""), "missing event: {s}");
        }
    }

    #[test]
    fn sched_writer_serializes_feed_unavailable_canonically() {
        let bytes = emit_to_vec(&[NodeSchedEvent::FeedUnavailable {
            reason: FeedReason::UnknownDisconnected,
        }]);
        let s = std::str::from_utf8(&bytes).expect("utf8");
        assert_eq!(
            s,
            "{\"event\":\"feed_unavailable\",\"reason\":\"unknown_disconnected\"}\n"
        );
    }

    #[test]
    fn sched_writer_two_runs_are_byte_identical() {
        let events = vec![
            NodeSchedEvent::ForgeTickSkipped {
                reason: FeedReason::NoBlockAvailable,
            },
            NodeSchedEvent::ForgeResult {
                outcome: ForgeOutcome::NotLeader,
            },
        ];
        let a = emit_to_vec(&events);
        let b = emit_to_vec(&events);
        assert_eq!(a, b);
    }
}
