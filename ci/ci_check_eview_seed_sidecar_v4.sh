#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONTINUITY-ACTIVATION ECA-2-pre (DC-CINPUT-06): the durable consensus profile includes
# genesis_hash + protocol_params_hash, persisted in the v4 SeedEpochConsensusInputs sidecar and
# recovered IDENTICALLY at warm-start. No CLI/config/genesis fallback, no recompute; a pre-v4 store
# fails closed with a TYPED upgrade error (ConsensusInputsSchemaUnsupported), distinct from
# corruption; the fingerprint + manifest seed_hash binding cover both hashes (transitively).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
SCI=crates/ade_ledger/src/seed_consensus_inputs.rs
MERGE=crates/ade_runtime/src/seed_consensus_merge.rs
BOOT=crates/ade_runtime/src/bootstrap.rs
MANIFEST=crates/ade_ledger/src/bootstrap_manifest.rs

# (1) the v4 schema: the two consensus-profile hash fields + the version / field-count bump.
grep -qE 'pub genesis_hash: Hash32' "$SCI" || fail "genesis_hash field missing from SeedEpochConsensusInputs"
grep -qE 'pub protocol_params_hash: Hash32' "$SCI" || fail "protocol_params_hash field missing from SeedEpochConsensusInputs"
grep -qF 'SEED_CINPUT_SCHEMA_VERSION: u32 = 4' "$SCI" || fail "schema version is not bumped to v4"
grep -qF 'FIELDS_OUTER: u64 = 11' "$SCI" || fail "FIELDS_OUTER is not 11 (the two new fields)"

# (2) both hashes are in the canonical encode AND decode (so the bytes -- and every binding over
#     them -- cover the profile).
grep -qF 'write_bytes_canonical(&mut buf, &inputs.genesis_hash.0)' "$SCI" || fail "encode does not write genesis_hash"
grep -qF 'write_bytes_canonical(&mut buf, &inputs.protocol_params_hash.0)' "$SCI" || fail "encode does not write protocol_params_hash"
grep -qF 'let genesis_hash = read_hash32(bytes, &mut o)?' "$SCI" || fail "decode does not read genesis_hash"
grep -qF 'let protocol_params_hash = read_hash32(bytes, &mut o)?' "$SCI" || fail "decode does not read protocol_params_hash"

# (3) the merge populates both from the canonical import bundle (the durable persist authority) --
#     copied verbatim, NOT recomputed, NOT CLI-supplied.
grep -qF 'genesis_hash: canonical.genesis_hash' "$MERGE" || fail "merge does not carry genesis_hash from the bundle"
grep -qF 'protocol_params_hash: canonical.protocol_params_hash' "$MERGE" || fail "merge does not carry protocol_params_hash from the bundle"

# (4) the TYPED upgrade error -- a pre-v4 sidecar is a reimport requirement, DISTINCT from
#     corruption; the bootstrap authority maps UnknownVersion to it (never the generic decode error).
grep -qF 'ConsensusInputsSchemaUnsupported' "$BOOT" || fail "the typed schema-upgrade error is missing"
grep -qF 'SeedConsensusInputsError::UnknownVersion { expected, found }' "$BOOT" || fail "the bootstrap authority does not distinguish a version mismatch from corruption"

# (4b) the LIVE warm-start decode (node_lifecycle warm_start_recovery -- the FIRST decode of the
#      sidecar, for geometry) ALSO surfaces the typed error, never a generic decode string, so the
#      live path matches the bootstrap authority's auditable diagnostics (both decode sites covered).
NL=crates/ade_node/src/node_lifecycle.rs
grep -qF 'NodeLifecycleError::ConsensusInputsSchemaUnsupported' "$NL" || fail "the live warm-start decode does not map a pre-v4 sidecar to the typed schema-upgrade error"
grep -qE 'fn warm_start_pre_v4_sidecar_is_typed_schema_upgrade_not_corruption' "$NL" || fail "the live-path typed-upgrade proof is missing"

# (5) the manifest seed_hash binds the sidecar bytes -> the two new hashes are covered TRANSITIVELY
#     (no manifest format change needed).
grep -qF 'blake2b_256(seed_bytes) != manifest.seed_hash' "$MANIFEST" || fail "the manifest seed_hash does not bind the sidecar bytes"

# (6) NO recompute: the merge is a pure mapping that persists the IMPORTED hashes verbatim -- it must
#     compute no hash (recomputing protocol_params_hash from reserialized params is restart-dependent
#     drift). The recovery sources the hashes ONLY from the decoded sidecar (no CLI/genesis re-supply).
if grep -nE 'blake2b' "$MERGE" >/dev/null 2>&1; then
    fail "the merge must not compute any hash -- the consensus-profile hashes are the IMPORTED values, persisted verbatim"
fi

# (7) the proofs.
grep -qE 'fn merge_persists_consensus_profile_hashes' "$MERGE" || fail "the merge-populates proof is missing"
grep -qE 'fn warm_start_pre_v4_sidecar_is_typed_schema_upgrade_not_corruption' "$BOOT" || fail "the typed-upgrade-error proof is missing"
grep -qE 'fn seed_cinput_canonical_bytes_cover_the_consensus_profile_hashes' "$SCI" || fail "the canonical-bytes-cover-the-hashes proof is missing"

if (( FAILED == 0 )); then
    echo "OK: SeedEpochConsensusInputs v4 (DC-CINPUT-06; genesis_hash + protocol_params_hash durable in the sidecar, recovered from the store, typed pre-v4 upgrade error, fingerprint+manifest binding covers them, no CLI/recompute fallback)"
fi
exit $FAILED
