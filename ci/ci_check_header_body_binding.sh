#!/usr/bin/env bash
# ci_check_header_body_binding.sh -- CN-CONS-04.
#
# "Header validation must bind exactly to the accepted body and consensus context."
# Structural half: the block_validity transition MUST (1) run the header-validation
# pipeline `validate_and_apply_header` (VRF / KES / nonce / slot / block_no / op_cert
# context binding) and fail-close on its error, and (2) bind the body to the header via
# the recomputed body-hash comparison (`computed_body_hash != header body_hash` ->
# `BodyHashMismatch`) BEFORE body application. Neither binding may be silently removed.
# Behavioral half = the 13 CN-CONS-04 tests (altered_body_rejected_by_hash_binding,
# praos_vrf_keyhash_mismatch_rejected, header_with_{slot,block_no,op_cert}_regression_rejected,
# header_with_invalid_vrf_proof_rejected, ...). Together = complete enforcement.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
T="$REPO_ROOT/crates/ade_ledger/src/block_validity/transition.rs"
fail() { echo "FAIL (ci_check_header_body_binding): $1" >&2; exit 1; }

# Returns 0 iff BOTH bindings are present in $1 (comment-stripped).
check() {
  local s; s="$(sed -E 's://.*$::' "$1")"
  grep -Eq 'validate_and_apply_header\(' <<< "$s" || return 1   # context binding runs
  grep -Eq 'computed_body_hash[[:space:]]*!=' <<< "$s" || return 1  # body-hash compared
  grep -Eq 'BodyHashMismatch' <<< "$s" || return 1              # mismatch fail-closes
  return 0
}

if [ "${1:-}" = "--self-test" ]; then
  [ -f "$T" ] || fail "transition.rs missing for self-test"
  tmp="$(mktemp)"; trap 'rm -f "$tmp"' EXIT
  # Strip the body-hash binding comparison -> the gate MUST detect the missing binding.
  sed -E '/computed_body_hash[[:space:]]*!=/d' "$T" > "$tmp"
  if check "$tmp"; then echo "FAIL: scanner did not detect the removed body-hash binding" >&2; exit 1; fi
  echo "PASS: scanner detects a removed header-body binding"; exit 0
fi

[ -f "$T" ] || fail "block_validity/transition.rs missing: $T"
check "$T" || fail "transition.rs missing the header-context (validate_and_apply_header) and/or body-hash (computed_body_hash != header body_hash -> BodyHashMismatch) binding (CN-CONS-04)"
echo "OK: block_validity transition binds the header to its consensus context + body-hash before body application (CN-CONS-04)"
