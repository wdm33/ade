#!/usr/bin/env bash
# ci_check_ledgerdb_state_decode.sh -- MITHRIL-VERIFIED-ANCHOR-IMPORT Stage 1.
#
# The native cardano-node V2 (utxohd-mem) LedgerDB `state` decoder is a deterministic,
# fail-closed, NON-EMITTING probe: it decodes the Conway NewEpochState into Ade's canonical
# CertState + pool distribution + Praos nonces, with EXPLICIT era-tagged telescope navigation
# (never "take the latest element"), a MANDATORY real VRF per active pool (a zero VRF is
# terminal), a PoolDistr<->pools VRF cross-check, a canonical encode/decode round-trip
# self-check, and a deterministic commitment. It emits NO LedgerState / UTxO / admission
# artifact -- the tables/UTxO reader + admission anchor are Stage 2.
set -euo pipefail

MOD="crates/ade_ledger/src/ledgerdb_state.rs"
HERM="crates/ade_ledger/tests/ledgerdb_state_hermetic.rs"
fail() { echo "FAIL (ci_check_ledgerdb_state_decode): $1" >&2; exit 1; }
for f in "$MOD" "$HERM"; do [ -f "$f" ] || fail "file $f missing"; done

# (A) Fail-closed: every terminal variant present (a decode failure halts deterministically,
# never a best-effort partial decode).
for v in MalformedCbor UnsupportedEra ZeroVrf PoolDistrVrfMismatch EpochMismatch RoundTripMismatch; do
  grep -Eq "$v" "$MOD" || fail "fail-closed variant $v missing"
done

# (B) EXPLICIT + era-tagged telescope navigation: the current era must equal Conway, and a
# non-Conway current era is terminal -- never a silent fallback to the latest element.
grep -Eq "CONWAY_TELESCOPE_INDEX" "$MOD" || fail "Conway era index gate missing"
grep -Eq "UnsupportedEra \{ current_index" "$MOD" || fail "a non-Conway current era must be terminal"

# (C) Mandatory real VRF + the PoolDistr<->pools VRF cross-check.
grep -Eq "ZeroVrf\(pool_id\)" "$MOD" || fail "a zero/default VRF must be terminal (ZeroVrf)"
grep -Eq "PoolDistrVrfMismatch" "$MOD" || fail "the PoolDistr<->pools VRF cross-check is missing"

# (D) Canonical round-trip self-check (the decoded CertState must encode + decode identically).
grep -Eq "decode_cert_state" "$MOD" || fail "the canonical round-trip self-check is missing"

# (E) NON-EMITTING: the entry point returns only the probe report; no UTxO/LedgerState/admission
# CONSTRUCTION (Stage 2 owns those). The probe is the SOLE public output.
grep -Eq "pub fn probe_ledgerdb_state\(" "$MOD" || fail "the probe entry point is missing"
if grep -Eq "UTxOState \{|LedgerState \{|AdmissionLog|admission::" "$MOD"; then
  fail "Stage 1 must NOT construct UTxO/LedgerState/admission artifacts (that is Stage 2)"
fi

# (F) BLUE determinism: no wall-clock / rand / env / HashMap USAGE in the authoritative decoder
# (the core-contract header comment naming the prohibitions is not a usage).
if grep -Eq "std::env::|SystemTime::|Instant::now|rand::|HashMap::|HashMap<|HashSet::|HashSet<" "$MOD"; then
  fail "the BLUE decoder must be deterministic (no env / clock / rand / HashMap)"
fi

# (G) The hermetic fail-closed + round-trip + determinism tests pass.
cargo test -p ade_ledger --test ledgerdb_state_hermetic --quiet >/dev/null 2>&1 \
  || fail "hermetic decoder tests failed (run: cargo test -p ade_ledger --test ledgerdb_state_hermetic)"

echo "OK: V2 LedgerDB state decoder is deterministic + fail-closed + era-tagged + VRF-mandatory + non-emitting (MITHRIL-VERIFIED-ANCHOR-IMPORT Stage 1)"
