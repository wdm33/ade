#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-J S7 — snapshot encoder single-authority closure.
#
# Mechanical guards for CE-N-J-7 + CN-STORE-08 + DC-STORE-08 + DC-STORE-09:
#
#   1. The only `pub fn` pair encoding/decoding the combined
#      `(LedgerState, PraosChainDepState)` snapshot bytes lives at
#      crates/ade_ledger/src/snapshot/framing.rs
#      (`encode_snapshot` / `decode_snapshot`). Any other `pub fn`
#      named `encode_snapshot` or `decode_snapshot` outside that
#      file is a single-authority regression (CN-STORE-08).
#   2. The only `pub fn` pair encoding/decoding `LedgerState` bytes
#      lives at crates/ade_ledger/src/snapshot/ledger.rs.
#   3. The only `pub fn` pair encoding/decoding `PraosChainDepState`
#      bytes lives at crates/ade_ledger/src/snapshot/chain_dep.rs.
#   4. The `SCHEMA_VERSION` constant lives only in framing.rs —
#      no parallel snapshot version constants (DC-STORE-09).
#   5. The fingerprint-mismatch + unknown-version variants of
#      `SnapshotDecodeError` are referenced from framing.rs — proves
#      the cross-check path exists at the framing layer
#      (DC-STORE-08 + DC-STORE-09).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FRAMING_SITE="crates/ade_ledger/src/snapshot/framing.rs"
LEDGER_SITE="crates/ade_ledger/src/snapshot/ledger.rs"
CHAIN_DEP_SITE="crates/ade_ledger/src/snapshot/chain_dep.rs"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

for site in "$FRAMING_SITE" "$LEDGER_SITE" "$CHAIN_DEP_SITE"; do
    if [[ ! -f "$REPO_ROOT/$site" ]]; then
        print_fail "canonical site missing: $site"
    fi
done

if [[ "$FAILED" -ne 0 ]]; then
    exit "$FAILED"
fi

# 1. Combined snapshot encoder/decoder authority: exactly one pair.
for fn in encode_snapshot decode_snapshot; do
    hits=$(grep -rln --include='*.rs' "pub fn ${fn}\b" "$REPO_ROOT/crates" | grep -v "^$REPO_ROOT/$FRAMING_SITE$" || true)
    if [[ -n "$hits" ]]; then
        print_fail "CN-STORE-08: stray pub fn ${fn} outside ${FRAMING_SITE}:"
        echo "$hits" | sed 's/^/    /'
    fi
done

# 2. LedgerState encoder/decoder authority.
for fn in encode_ledger_state decode_ledger_state; do
    hits=$(grep -rln --include='*.rs' "pub fn ${fn}\b" "$REPO_ROOT/crates" | grep -v "^$REPO_ROOT/$LEDGER_SITE$" || true)
    if [[ -n "$hits" ]]; then
        print_fail "CN-STORE-08: stray pub fn ${fn} outside ${LEDGER_SITE}:"
        echo "$hits" | sed 's/^/    /'
    fi
done

# 3. PraosChainDepState encoder/decoder authority.
for fn in encode_chain_dep decode_chain_dep; do
    hits=$(grep -rln --include='*.rs' "pub fn ${fn}\b" "$REPO_ROOT/crates" | grep -v "^$REPO_ROOT/$CHAIN_DEP_SITE$" || true)
    if [[ -n "$hits" ]]; then
        print_fail "CN-STORE-08: stray pub fn ${fn} outside ${CHAIN_DEP_SITE}:"
        echo "$hits" | sed 's/^/    /'
    fi
done

# 4. SCHEMA_VERSION authority.
schema_hits=$(grep -rln --include='*.rs' 'pub const SCHEMA_VERSION' "$REPO_ROOT/crates" | grep -v "^$REPO_ROOT/$FRAMING_SITE$" || true)
if [[ -n "$schema_hits" ]]; then
    print_fail "DC-STORE-09: stray pub const SCHEMA_VERSION outside ${FRAMING_SITE}:"
    echo "$schema_hits" | sed 's/^/    /'
fi

# 5. Cross-check paths present in framing.rs.
if ! grep -q 'FingerprintMismatch' "$REPO_ROOT/$FRAMING_SITE"; then
    print_fail "DC-STORE-08: framing.rs does not reference FingerprintMismatch"
fi
if ! grep -q 'UnknownVersion' "$REPO_ROOT/$FRAMING_SITE"; then
    print_fail "DC-STORE-09: framing.rs does not reference UnknownVersion"
fi

if [[ "$FAILED" -eq 0 ]]; then
    echo "OK: snapshot encoder closure (CN-STORE-08 + DC-STORE-08 + DC-STORE-09)"
fi
exit "$FAILED"
