//! Thin RED glue around the GREEN replayable post-switch branch-continuity
//! reducer (PHASE4-N-AO S10, DC-EVIDENCE-05). Reads a closed convergence-evidence
//! transcript (JSONL) and prints the bounded-window release verdict. The CI gate
//! `ci/ci_check_post_switch_convergence_window.sh` invokes this so the live gate
//! and the hermetic replay test share ONE implementation (no Rust/Python drift).
//!
//! Bounds are FIXED here, not tuned per-run: max 200 slots / 20 admitted blocks.

use ade_node::post_switch_continuity::{evaluate_release_window, EventView, ReleaseVerdict};
use std::io::BufRead;

const MAX_SLOTS: u64 = 200;
const MAX_BLOCKS: u32 = 20;

fn main() {
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: post_switch_continuity <transcript-conv.jsonl>");
            std::process::exit(2);
        }
    };
    let file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("FAIL: cannot open {path}: {e}");
            std::process::exit(2);
        }
    };
    let mut events = Vec::new();
    for line in std::io::BufReader::new(file).lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(t) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(ev) = EventView::from_json(&v) {
            events.push(ev);
        }
    }

    match evaluate_release_window(&events, MAX_SLOTS, MAX_BLOCKS) {
        ReleaseVerdict::Pass {
            continuity,
            terminal,
        } => {
            println!("PASS: post_switch_convergence_window (DC-EVIDENCE-04 / DC-EVIDENCE-05)");
            println!("  bounds: max_slots={MAX_SLOTS} max_admitted_blocks={MAX_BLOCKS}");
            println!("  continuity: {continuity:?}");
            println!("  terminal:   {terminal:?}");
            std::process::exit(0);
        }
        other => {
            eprintln!("FAIL: post_switch_convergence_window ({MAX_SLOTS} slots / {MAX_BLOCKS} blocks)");
            eprintln!("  verdict: {other:?}");
            std::process::exit(1);
        }
    }
}
