#!/usr/bin/env bash
# ci_check_fork_choice_evidence_closed.sh -- PHASE4-N-AO S9 (DC-EVIDENCE-04).
#
# The live SELECT path emits a CLOSED, observe-only convergence-evidence sequence
# (needs_fork_choice -> lca_discovered -> candidate_fragment_built ->
# fork_choice_selected -> branch_fetch_* -> branch_prevalidated ->
# fork_switch_applied | fork_switch_failed). Closed discriminants == the emit-only
# allow-list; no free-form error strings (failure_code is a closed enum;
# fork_switch_id is a bounded blake2b of the canonical tuple); the evidence is
# OBSERVE-ONLY -- never read by the BLUE selector / LCA walk / candidate builder.
set -euo pipefail

EV="crates/ade_node/src/admission_log/event.rs"
WR="crates/ade_node/src/admission_log/writer.rs"
CE="crates/ade_node/src/convergence_evidence.rs"
LCA="crates/ade_node/src/lca_walk.rs"
AGG="crates/ade_node/src/candidate_aggregator.rs"
fail() { echo "FAIL (ci_check_fork_choice_evidence_closed): $1" >&2; exit 1; }
for f in "$EV" "$WR" "$CE" "$LCA" "$AGG"; do [ -f "$f" ] || fail "module $f missing"; done

EVENTS="needs_fork_choice lca_discovered candidate_fragment_built fork_choice_selected branch_fetch_started branch_fetch_completed branch_prevalidated fork_switch_applied fork_switch_failed fork_switch_superseded"

# (A) Vocabulary == allow-list: each of the 9 closed events has a discriminator
# (event.rs) AND is in the writer's DISCRIMINATORS allow-list (the gate's positive
# grep + the closed-set test).
for e in $EVENTS; do
  grep -Eq "=> \"$e\"" "$EV" || fail "missing discriminator for '$e' in $EV"
  grep -Eq "\"$e\"" "$WR"   || fail "'$e' missing from the writer DISCRIMINATORS allow-list"
done

# (B) No free-form error string: failure_code is the closed ForkChoiceEvidenceFailure
# enum (closed as_str), NOT a String. fork_choice failures must not carry a String.
grep -Eq "pub enum ForkChoiceEvidenceFailure" "$EV" \
  || fail "ForkChoiceEvidenceFailure closed enum missing"
if grep -Eq "failure_code: String" "$EV"; then
  fail "failure_code must be the closed enum, never a free-form String"
fi
# fork_choice_selected.result is the closed ForkChoiceResult enum (win/loss).
grep -Eq "pub enum ForkChoiceResult" "$EV" || fail "ForkChoiceResult closed enum missing"

# (C) fork_switch_id is a BOUNDED DETERMINISTIC id derived from the canonical tuple
# (winning_peer + fork_anchor + winner_tip) via blake2b -- never free-form text.
FSID_FN="$(awk '/pub fn fork_switch_id\(/{f=1} f{print} f&&/^}/{exit}' "$CE")"
[ -n "$FSID_FN" ] || fail "fork_switch_id helper missing in $CE"
echo "$FSID_FN" | grep -Eq "blake2b" \
  || fail "fork_switch_id must be a blake2b of the canonical tuple (bounded deterministic)"
echo "$FSID_FN" | grep -Eq "winning_peer|anchor_hash" \
  || fail "fork_switch_id must derive from the canonical peer+anchor+winner_tip tuple"

# (D) Observe-only: the BLUE LCA walk + the candidate builder must NOT reference the
# evidence sink / emit methods -- evidence taps live in the RED dispatch/apply only,
# and no evidence event is ever consumed by selection/walk/build.
for f in "$LCA" "$AGG"; do
  if grep -Eq "ConvergenceEvidence|emit_needs_fork_choice|emit_fork_choice_selected|emit_fork_switch_applied|AdmissionLogEvent" "$f"; then
    fail "observe-only violated: $f (BLUE walk/builder) references evidence -- it must never read/emit it"
  fi
done
# The walk + the selector decision must not branch on an emit result (fire-and-forget).
if grep -Eq "if .*emit_(needs_fork_choice|lca_discovered|fork_choice_selected|fork_switch_applied)" "crates/ade_node/src/node_lifecycle.rs"; then
  fail "an evidence emit result must never gate control flow (observe-only)"
fi

# (E) Every win resolves to EXACTLY ONE terminal -- applied / failed / superseded.
# applied|failed at the apply site; superseded in the dispatch when a newer win
# overwrites an older provisional pending (so the relay loop applying only the FINAL
# pending leaves no dangling wins). The hermetic test proves the 1:1 fork_switch_id
# pairing.
grep -Eq "emit_fork_switch_applied\(" "crates/ade_node/src/node_lifecycle.rs" \
  || fail "the Adopted apply path must emit fork_switch_applied"
grep -Eq "emit_fork_switch_failed\(" "crates/ade_node/src/node_lifecycle.rs" \
  || fail "the ProofFailed apply path must emit fork_switch_failed"
grep -Eq "emit_fork_switch_superseded\(" "crates/ade_node/src/node_lifecycle.rs" \
  || fail "an overwritten provisional win must emit fork_switch_superseded (no dangling wins)"

echo "OK: fork-choice evidence is closed-vocab + allow-listed + no-free-form-strings + bounded fork_switch_id + observe-only (DC-EVIDENCE-04)"
