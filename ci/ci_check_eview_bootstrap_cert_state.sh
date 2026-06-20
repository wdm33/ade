#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW S3f-2-pre (DC-EVIEW-09): the manifest-bound bootstrap cert-state
# import. The seed (per-POOL active view) and the cert state (per-CREDENTIAL ledger
# continuation state) are SEPARATE authority surfaces -- the closed seed record is NOT
# widened. They bind only through a canonical BootstrapManifest (network, era, source
# point, both artifact hashes, source commitment). The cert-state artifact is the
# COMPLETE canonical CertState via the EXISTING codec (decode_cert_state, verbatim -- no
# hand-reconstructed maps). Fail-closed on missing one side / hash / network / era /
# malformed, BEFORE any bootstrap state durables. No live producer behaviour change.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
M=crates/ade_ledger/src/bootstrap_manifest.rs
B=crates/ade_node/src/admission/bootstrap.rs
S=crates/ade_node/src/admission/seed_to_snapshot.rs
SEED=crates/ade_ledger/src/seed_consensus_inputs.rs

test -f "$M" || fail "the bootstrap manifest ($M) is missing"

# (1) the manifest binds all six fields.
grep -qE 'pub struct BootstrapManifest' "$M" || fail "BootstrapManifest missing"
for f in network_magic era source_point seed_hash cert_state_hash source_commitment; do
    grep -qE "pub $f" "$M" || fail "BootstrapManifest is missing the $f binding"
done

# (2) the cert state is imported via the EXISTING canonical codec, verbatim (NOT
#     hand-reconstructed delegation/reward maps).
grep -qF 'use crate::snapshot::cert_state::decode_cert_state' "$M" \
    || fail "the manifest import does not reuse the canonical decode_cert_state codec"
grep -qF 'decode_cert_state(cert_state_bytes)' "$M" \
    || fail "verify_and_import_cert_state does not decode the COMPLETE CertState via the codec"

# (3) fail-closed: hash / network / era / malformed / decode all reject.
for e in SeedHashMismatch CertStateHashMismatch NetworkMismatch EraMismatch MalformedManifest CertStateDecode; do
    grep -qE "$e" "$M" || fail "BootstrapManifestError is missing the $e fail-closed variant"
done

# (4) the SEPARATE-SURFACE rule: the closed seed record is NOT widened with a delegation
#     map / rewards / cert state.
if grep -qE 'pub delegations|pub rewards|pub cert_state|pub delegation' "$SEED"; then
    fail "SeedEpochConsensusInputs was widened with cert-state fields -- keep the surfaces separate"
fi

# (5) the bootstrap populates cert_state fail-closed (partial package -> error) and
#     build_seed_ledger / seed_to_snapshot carry it.
grep -qE 'fn import_bootstrap_cert_state' "$B" || fail "bootstrap does not import the manifest-bound cert state"
grep -qF 'verify_and_import_cert_state(' "$B" || fail "bootstrap does not verify the manifest binding"
grep -qE 'BootstrapCertState' "$B" || fail "no fail-closed BootstrapCertState error in the bootstrap"
grep -qE 'cert_state: ade_ledger::delegation::CertState' "$S" \
    || fail "seed_to_snapshot does not take the bootstrap cert state"
grep -qF 'ledger.cert_state = cert_state' "$S" || fail "build_seed_ledger does not populate cert_state"

# (6) the load-bearing fail-closed proofs.
for t in verify_and_import_happy_path seed_hash_mismatch_fails_closed cert_state_hash_mismatch_fails_closed \
         network_and_era_mismatch_fail_closed malformed_manifest_fails_closed malformed_cert_state_fails_closed_after_hash_ok; do
    grep -qE "fn $t" "$M" || fail "the $t proof is missing"
done

# (7) no live producer change: the manifest/import MECHANISM does not feed live leader
#     election (the EpochConsensusView / leader-schedule wiring is the DC-EVIEW-08
#     activation, not here). (bootstrap.rs legitimately sets up the live PoolDistrView for
#     the runner; the constraint is on the cert-state import mechanism module.)
if grep -qE 'query_leader_schedule|EpochConsensusView|PoolDistrView' "$M"; then
    fail "the bootstrap cert-state import mechanism reaches into live leader election -- out of scope"
fi

if (( FAILED == 0 )); then
    echo "OK: bootstrap cert-state import (DC-EVIEW-09; separate manifest-bound surface, canonical codec verbatim, fail-closed, no seed widening, no live producer change)"
fi
exit $FAILED
