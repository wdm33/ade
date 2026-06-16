#!/usr/bin/env bash
set -uo pipefail

# MEM-OPT-UTXO-DISK S1.5 (DC-MEM-10): the v2 UTxO fingerprint is a NAMED,
# domain-separated, versioned, golden-vector-pinned commutative set commitment
# (Ristretto255 ECMH). A static assertion that the construction stays a FROZEN
# internal replay contract -- and that S1.5a did NOT flip v1 production.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
SC=crates/ade_crypto/src/utxo_set_commitment.rs
FP=crates/ade_ledger/src/fingerprint.rs

# (1) the named primitive + FROZEN domain-separation tags.
grep -qF 'DOMAIN_ENTRY: &[u8] = b"ade.utxo.fp.v2.entry"' "$SC" || fail "DOMAIN_ENTRY tag changed/missing (frozen)"
grep -qF 'DOMAIN_DIGEST: &[u8] = b"ade/fp/utxo/v2"' "$SC" || fail "DOMAIN_DIGEST tag changed/missing (frozen)"
grep -qE 'struct UtxoSetCommitment' "$SC" || fail "the UtxoSetCommitment primitive is missing"
grep -qE 'from_uniform_bytes' "$SC" || fail "the hash-to-Ristretto (from_uniform_bytes) is missing"

# (2) golden vectors present (FROZEN -- regenerating silently is a contract break).
for g in 70b2faf838d2fe2cdf7d2d54a10491cbb5f572ba61e17768b7ddf8f7fd466ac4 \
         84ddb1dd89b50f55a6443c9086007ad248baace25d12113a79ceeefcecddf151 \
         a72f15a7646926f3c2c135335d463ca47139fbf50f3f07e64564202b65461fbd; do
    grep -qF "$g" "$SC" || fail "golden vector $g missing (frozen)"
done

# (3) explicit versioning + the S1.5b PRODUCTION CUTOVER: fingerprint() is v2.
grep -qE 'FINGERPRINT_VERSION_V1: u32 = 1' "$FP" || fail "FINGERPRINT_VERSION_V1 missing"
grep -qE 'FINGERPRINT_VERSION_V2: u32 = 2' "$FP" || fail "FINGERPRINT_VERSION_V2 missing"
grep -qE 'fn fingerprint_utxo_v2' "$FP" || fail "fingerprint_utxo_v2 (the oracle) missing"
grep -qE 'fn fingerprint_v2' "$FP" || fail "fingerprint_v2 missing"
grep -qE 'pub struct IncrementalUtxoFp' "$FP" || fail "IncrementalUtxoFp (the per-block maintenance) missing"
# v1 preserved as fingerprint_v1 (historical); production fingerprint() delegates to v2.
grep -qE 'pub fn fingerprint_v1' "$FP" || fail "fingerprint_v1 (frozen historical v1) missing"
grep -A3 -F 'pub fn fingerprint(state: &LedgerState) -> LedgerFingerprint {' "$FP" | grep -qF 'fingerprint_v2(state)' \
    || fail "production fingerprint() does not delegate to fingerprint_v2 -- the S1.5b cutover is not in place"

# (4) the fingerprint-version gate: a v1 (or unversioned) store is rejected fail-closed.
PS=crates/ade_runtime/src/chaindb/persistent.rs
grep -qE 'FINGERPRINT_VERSION: u32 = 2' "$PS" || fail "the store FINGERPRINT_VERSION marker is missing"
grep -qE 'FingerprintVersionMismatch' "$PS" || fail "the fingerprint-version fail-closed gate is missing in persistent.rs"

if (( FAILED == 0 )); then
    echo "OK: v2 UTxO set commitment + S1.5b cutover (named/domain-separated/golden-pinned; fingerprint()=v2; fingerprint_v1 historical; old-v1 store fail-closed)"
fi
exit $FAILED
