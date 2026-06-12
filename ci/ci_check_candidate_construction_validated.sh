#!/usr/bin/env bash
# ci_check_candidate_construction_validated.sh -- PHASE4-N-AO S2 (DC-NODE-35).
#
# The candidate aggregator is BLUE-safe + PURE: fragments come ONLY from
# validate_and_apply_header output (no minting), it performs no store reads /
# materialize / selection / WAL / IO, it is deterministic, and it fails closed.
set -euo pipefail

M="crates/ade_node/src/candidate_aggregator.rs"
fail() { echo "FAIL (ci_check_candidate_construction_validated): $1" >&2; exit 1; }
[ -f "$M" ] || fail "module $M missing"

# (A) VALIDATED-ONLY: the build path validates via validate_and_apply_header and
# collects ONLY its `.summary`; it mints no ValidatedHeaderSummary and never uses
# the follow.rs peer-trusted minting path.
grep -Eq 'validate_and_apply_header\(' "$M" \
  || fail "candidates must be validated via validate_and_apply_header"
grep -Eq 'applied\.summary' "$M" \
  || fail "fragment headers must be validate_and_apply_header output (applied.summary)"
if grep -nE 'ValidatedHeaderSummary[[:space:]]*\{' "$M"; then
  fail "must NOT construct (mint) a ValidatedHeaderSummary -- only validated output"
fi
if grep -nE 'ade_core_interop::follow|project_header_from|fn as_summary|\.as_summary\(' "$M"; then
  fail "must NOT use the follow.rs minting path"
fi

# (B) PURE: no store reads / materialize / selection / WAL / socket / clock / async.
# Strip standalone comment lines first -- the module's docstrings legitimately
# NAME select_best_chain / materialize_rolled_back_state to describe the S3
# boundary; the prohibition is on CODE references, not prose.
if grep -vE '^[[:space:]]*//' "$M" | grep -qE '\bChainDb\b|SnapshotReader|materialize_rolled_back_state|\bWalStore\b|\bWalEntry\b|select_best_chain|TcpStream|SystemTime|Instant|tokio::'; then
  fail "S2 must be PURE -- no ChainDb/materialize/select_best_chain/WAL/socket/clock/async (code references)"
fi

# (C) DETERMINISTIC: no HashMap/HashSet (code references; the Core Contract banner
# legitimately names them as a prohibition).
if grep -vE '^[[:space:]]*//' "$M" | grep -qE '\bHashMap\b|\bHashSet\b'; then
  fail "no HashMap/HashSet -- candidate construction must be deterministic"
fi

# (D) FAIL-CLOSED: a header validation error / empty headers yield CandidateBuildError,
# never a fragment.
grep -Eq 'CandidateBuildError::HeaderInvalid' "$M" \
  || fail "a header validation failure must yield CandidateBuildError::HeaderInvalid"
grep -Eq 'CandidateBuildError::EmptyHeaders' "$M" \
  || fail "zero headers must fail closed (CandidateBuildError::EmptyHeaders)"
grep -Eq 'return Err\(CandidateBuildError::EmptyHeaders\)' "$M" \
  || fail "empty headers must early-return the fail-closed error"

echo "OK: candidate aggregator is validated-only + pure + deterministic + fail-closed (DC-NODE-35)"
