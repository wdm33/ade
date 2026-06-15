#!/usr/bin/env bash
set -uo pipefail

# CN-MEM-01 (MEM-MEASURE-A1): untrusted inbound work is admitted through a
# deterministic bounded policy BEFORE the scarce authoritative resource — the
# BLUE `mempool_ingress` validation — is consumed; the memory-measurement
# substrate pairs every RSS observation with a replay fingerprint + verdict.
#
# Structural gate. The byte-level properties (bounded forwarding, verdict
# preservation, no false-accept, replay stability, RSS-magnitude-ignored
# validation) are proven by the ade_node unit tests under
# crates/ade_node/src/mem_measure/*. This script verifies the substrate SHAPE:
#   1. The four mem_measure modules exist + are registered (lib.rs, mod.rs).
#   2. The GREEN bounded fold defines the closed model (budgets, ShedReason,
#      BoundedOutcome, replay_bounded_ingress_trace) and FRONTS mempool_ingress.
#   3. The GREEN files (bounded_admission.rs, evidence.rs) contain NO
#      nondeterministic / I-O constructs (HashMap/HashSet, clock, RNG, float,
#      async, std::fs, /proc).
#   4. The RED /proc/self/status read is CONFINED to rss_sampler.rs.
#   5. The evidence pairing + validator entry points exist.
#   6. The load-bearing test names exist in their modules.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MM="$REPO_ROOT/crates/ade_node/src/mem_measure"
LIB="$REPO_ROOT/crates/ade_node/src/lib.rs"
MODRS="$MM/mod.rs"
BOUNDED="$MM/bounded_admission.rs"
SAMPLER="$MM/rss_sampler.rs"
EVIDENCE="$MM/evidence.rs"
RUNNER="$MM/runner.rs"

FAIL=0

# 1. files exist
for f in "$MODRS" "$BOUNDED" "$SAMPLER" "$EVIDENCE" "$RUNNER" "$LIB"; do
    [ -f "$f" ] || { echo "FAIL: missing $f"; FAIL=1; }
done
[ "$FAIL" -eq 0 ] || exit 1

# 1a. lib.rs registers the module; mod.rs declares the four submodules.
grep -qE '^pub mod mem_measure;' "$LIB" \
    || { echo "FAIL: lib.rs does not declare 'pub mod mem_measure;'"; FAIL=1; }
for m in bounded_admission evidence rss_sampler runner; do
    grep -qE "^pub mod ${m};" "$MODRS" \
        || { echo "FAIL: mod.rs missing 'pub mod ${m};'"; FAIL=1; }
done

# 2. bounded model symbols + fronts the BLUE mempool_ingress authority.
for sym in \
    'pub const MAX_INBOUND_ADMISSION_COUNT' \
    'pub const MAX_INBOUND_ADMISSION_BYTES' \
    'pub enum ShedReason' \
    'pub enum BoundedOutcome' \
    'pub fn replay_bounded_ingress_trace'; do
    grep -qE "^${sym}" "$BOUNDED" || { echo "FAIL: $BOUNDED missing: ${sym}"; FAIL=1; }
done
grep -qE 'mempool_ingress\(' "$BOUNDED" \
    || { echo "FAIL: the bounded fold does not front mempool_ingress (the BLUE authority)"; FAIL=1; }

# 3. GREEN files: no nondeterministic / I-O constructs (full-line comments
#    excluded so the doc headers may name what they forbid). Here-string on the
#    second grep avoids a pipefail SIGPIPE flake.
for g in "$BOUNDED" "$EVIDENCE"; do
    NONCOMMENT=$(grep -vE '^[[:space:]]*//' "$g" || true)
    HITS=$(grep -nE '\b(HashMap|HashSet|SystemTime|Instant|rand::|thread_rng|f32|f64)\b|std::fs|/proc|tokio::|async[[:space:]]+fn|\.await' <<< "$NONCOMMENT" || true)
    if [ -n "$HITS" ]; then
        echo "FAIL: GREEN file $g uses forbidden nondeterministic/I-O constructs:"
        echo "$HITS"
        FAIL=1
    fi
done

# 4. /proc/self/status read confined to the RED sampler. Doc comments may NAME
#    it (e.g. mod.rs describing the sampler), so only non-comment lines count.
for f in "$BOUNDED" "$EVIDENCE" "$RUNNER" "$MODRS"; do
    CODE=$(grep -vE '^[[:space:]]*//' "$f" || true)
    if grep -qE '/proc/self/status' <<< "$CODE"; then
        echo "FAIL: /proc/self/status read leaked into $f (must be confined to rss_sampler.rs)"
        FAIL=1
    fi
done
grep -qE '/proc/self/status' "$SAMPLER" \
    || { echo "FAIL: rss_sampler.rs does not read /proc/self/status"; FAIL=1; }

# 5. evidence pairing + validator entry points.
for sym in 'pub fn pair_replay' 'pub fn validate_evidence' 'pub fn fingerprint_hex'; do
    grep -qE "^${sym}" "$EVIDENCE" || { echo "FAIL: $EVIDENCE missing: ${sym}"; FAIL=1; }
done

# 6. load-bearing test names exist in their modules.
for pair in \
    "bounded_admission_respects_count_budget:$BOUNDED" \
    "bounded_admission_respects_byte_budget:$BOUNDED" \
    "bounded_admission_is_deterministic:$BOUNDED" \
    "bounded_gate_under_budget_equals_unbounded:$BOUNDED" \
    "bounded_gate_preserves_admit_verdict:$BOUNDED" \
    "bounded_gate_no_false_accept_under_pressure:$BOUNDED" \
    "validator_ignores_rss_magnitude:$EVIDENCE" \
    "diverged_verdict_is_invalid_evidence:$EVIDENCE" \
    "percentile_nearest_rank_is_deterministic:$SAMPLER" \
    "hermetic_measurement_verdict_is_agreed:$RUNNER" \
    "hermetic_measurement_is_replay_stable:$RUNNER"; do
    t="${pair%%:*}"
    f="${pair#*:}"
    grep -qE "fn ${t}\b" "$f" || { echo "FAIL: missing test ${t} in ${f}"; FAIL=1; }
done

if [ "$FAIL" -eq 0 ]; then
    echo "PASS: CN-MEM-01 bounded inbound admission + measurement substrate (model fronts mempool_ingress; GREEN deterministic; RED /proc confined; replay-fingerprint paired)"
fi
exit "$FAIL"
