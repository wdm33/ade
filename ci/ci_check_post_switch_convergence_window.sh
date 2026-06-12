#!/usr/bin/env bash
# ci_check_post_switch_convergence_window.sh -- PHASE4-N-AO CE-AO-6
# (DC-EVIDENCE-04 refined + DC-EVIDENCE-05).
#
# RELEASE/EVIDENCE-tier transcript checker (NOT a BLUE consensus rule). Thin
# wrapper around the GREEN replayable post-switch branch-continuity reducer
# (crates/ade_node/src/post_switch_continuity.rs) -- the SAME implementation the
# hermetic replay test exercises, so the live gate and the audited invariant
# cannot drift. The reducer proves Ade STAYED on the adopted valid branch and did
# not diverge while catching up -- a real correctness property, not a lucky
# exact-tip moment and not a frozen venue.
#
# Hard proof (must precede the window): block_received from BOTH peers ->
# fork_choice_selected{win} -> branch_fetch_completed -> branch_prevalidated ->
# fork_switch_applied{ForkChoiceWin} at X -> block_admitted X.
# Bounded window after X (bounds FIXED in the bin, not tuned post-hoc):
#   max_slots = 200, max_admitted_blocks = 20
#   - PostSwitchContinuity::ContinuesSelectedBranch: unbroken prev_hash lineage
#     from X across every post-X block_admitted, no diverged, every win terminal
#   - terminal: agreement_verdict{agreed, our==peer} at X-or-descendant, OR a
#     validated prefix of peer (continuity holds + >=1 followed descendant +
#     peer observed ahead). The peer tip is an OBSERVED comparison only.
set -euo pipefail
C="${1:?usage: $0 <transcript-conv.jsonl>}"
[ -f "$C" ] || { echo "FAIL: transcript $C not found" >&2; exit 1; }

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/debug/post_switch_continuity"

# Build the reducer bin if absent / stale. One implementation, shared with the
# replay test `post_switch_continuity_replays_byte_identical`.
if [ ! -x "$BIN" ] || [ "$ROOT/crates/ade_node/src/post_switch_continuity.rs" -nt "$BIN" ]; then
  echo "building post_switch_continuity reducer bin..." >&2
  ( cd "$ROOT" && cargo build -q -p ade_node --bin post_switch_continuity ) >&2
fi

exec "$BIN" "$C"
