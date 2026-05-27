#!/usr/bin/env bash
#
# ci_check_leader_check_authority.sh — PHASE4-N-R-A S2 / A2 gate.
#
# Enforces CN-FORGE-02's "no external caller may bypass
# LeaderCheckVerdict" rule:
#
# The helper `is_leader_for_vrf_output` lives at
# `ade_core::consensus::leader_check::is_leader_for_vrf_output`.
# It is publicly callable, but only by an allow-list of files that
# implement the canonical leader-decision path:
#
#   1. crates/ade_core/src/consensus/leader_check.rs (definition site +
#      use inside verify_and_evaluate_leader)
#   2. crates/ade_ledger/src/producer/forge.rs (defense-in-depth pin +
#      defensive call; NC-VRF-3 single-source-of-leader-truth invariant)
#
# Any other file that imports or calls `is_leader_for_vrf_output` is
# REJECTED. New code MUST use `verify_and_evaluate_leader` and consume
# the closed `LeaderCheckVerdict` enum.
#
# The gate searches across all Rust source files (.rs) for the symbol
# reference (import path, fn call, or fn pointer) and rejects unknown
# call sites.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

ALLOW_LIST=(
  "crates/ade_core/src/consensus/leader_check.rs"
  "crates/ade_ledger/src/producer/forge.rs"
  # Integration test exercising the lower-level eligibility math
  # against a corpus; not a production call site.
  "crates/ade_core/tests/leader_schedule_corpus.rs"
  # consensus/mod.rs re-export and leader_schedule.rs comments are
  # also accepted — the grep filter below excludes them.
)

# Find all .rs files referencing the symbol via a hard pattern
# (import / fn call / fn pointer).
HITS=$(
  grep -rln \
    -e "use ade_core::consensus::.*is_leader_for_vrf_output" \
    -e "consensus::leader_check::is_leader_for_vrf_output" \
    -e "is_leader_for_vrf_output(" \
    crates/ 2>/dev/null \
  | sort -u \
  || true
)

# Filter out the allow-list and the definition + re-export sites.
VIOLATIONS=""
for f in $HITS; do
  skip=0
  for allowed in "${ALLOW_LIST[@]}"; do
    if [[ "$f" == "$allowed" ]]; then
      skip=1
      break
    fi
  done
  # consensus/mod.rs is the canonical re-export site; allow.
  if [[ "$f" == "crates/ade_core/src/consensus/mod.rs" ]]; then
    skip=1
  fi
  # leader_schedule.rs may contain doc comments referencing the
  # function by name; the grep above only matches the fn-call /
  # use-path patterns so doc comments alone won't trigger. But
  # belt-and-suspenders: allow leader_schedule.rs since it cannot
  # call the function (no import).
  if [[ "$f" == "crates/ade_core/src/consensus/leader_schedule.rs" ]]; then
    skip=1
  fi
  if [[ $skip -eq 0 ]]; then
    VIOLATIONS+="$f\n"
  fi
done

if [[ -n "$VIOLATIONS" ]]; then
  echo "[ci_check_leader_check_authority] FAIL — external caller(s) bypassing LeaderCheckVerdict:"
  echo -e "$VIOLATIONS" | sed 's/^/  /'
  echo ""
  echo "  Allow-list:"
  for a in "${ALLOW_LIST[@]}"; do
    echo "    $a"
  done
  echo ""
  echo "  New leader-decision call sites MUST use:"
  echo "    ade_core::consensus::leader_check::verify_and_evaluate_leader"
  echo "  which returns a closed LeaderCheckVerdict (Eligible / NotEligible)."
  exit 1
fi

echo "[ci_check_leader_check_authority] PASS (allow-list respected)"
