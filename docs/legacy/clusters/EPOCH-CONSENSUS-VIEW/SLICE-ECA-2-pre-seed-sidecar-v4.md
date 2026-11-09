# SLICE ECA-2-pre — SeedEpochConsensusInputs v4 (durable consensus-profile hashes)

Part of EPOCH-CONTINUITY-ACTIVATION. Prerequisite for ECA-2 (deterministic activation inputs). The
ECA-2 investigation found that 8 of the 10 `EviewActivationInputs` fields are already recoverable
from canonical durable state, but **`genesis_hash` and `protocol_params_hash` are not persisted** —
they exist only in the admission-phase `LiveConsensusInputsCanonical` bundle, never in the recovered
`SeedEpochConsensusInputs` sidecar. Both are needed at runtime for the ECA-0b consensus-profile
commitment `blake2b(domain ‖ genesis_hash ‖ protocol_params_hash ‖ asc)` that `derive_candidate`
binds and `to_pool_distr_view` re-verifies.

## User directive (2026-06-21)

These hashes are NOT optional EVIEW metadata — they are part of the durable consensus profile a
continuous producer must recover IDENTICALLY after restart. They go in the SAME sidecar as
eta0/asc/venue-geometry/pool-distribution (the established durable consensus-inputs authority,
DC-CINPUT-05), bumped v3→v4. Rejected: a separate EVIEW-only manifest authority (splits one runtime
consensus profile across two recovered surfaces → a mismatch class + extra plumbing); recomputing
`protocol_params_hash` from parsed/reserialized JSON (byte-sensitive → restart-dependent semantic
drift; the hash is over the IMPORTED protocol-params JSON). See
`feedback_durable_state_is_replay_authority`.

**v4 rules (verbatim):** old v1/v2/v3 fail closed for ECA-capable startup; import writes v4
canonical bytes BEFORE state is usable; fingerprint + manifest binding cover both new hashes;
warm-start reads the hashes ONLY from the v4 durable sidecar (no CLI/config/genesis fallback);
wrong/missing/mismatched → structured terminal error. NUANCE: a pre-v4 store must report a TYPED
upgrade/reimport requirement (`ConsensusInputsSchemaUnsupported { found_version, required_version }`),
NOT generic corruption — fail-closed but recoverable/auditable.

## Design

1. **`SeedEpochConsensusInputs` gains two fields** (`ade_ledger/src/seed_consensus_inputs.rs`):
   `genesis_hash: Hash32`, `protocol_params_hash: Hash32` — the durable consensus profile, grouped
   with `epoch_nonce` (the commitment inputs nonce/genesis/pp/asc).

2. **Codec v3→v4**: `SEED_CINPUT_SCHEMA_VERSION` 3→4; `FIELDS_OUTER` 9→11; encode/decode the two
   `bytes(32)` fields after `epoch_nonce`, before the asc array; refresh the (stale) wire-shape doc.
   The byte-canonical round-trip check (`re-encode != input → MalformedCbor`) extends automatically.

3. **Populate at import** (`ade_runtime/src/seed_consensus_merge.rs`): `merge_seed_epoch_consensus_inputs`
   copies `canonical.genesis_hash` + `canonical.protocol_params_hash` from the bundle (which already
   carries both). This is the SINGLE real constructor; the persist path writes v4 bytes before state
   is usable (`persist_seed_epoch_consensus_inputs` → put + WAL provenance, unchanged).

4. **Typed upgrade error** (`ade_runtime/src/bootstrap.rs`): the codec keeps `UnknownVersion {expected, found}`
   (now expected=4) — the precise version-mismatch fact, distinct from `MalformedCbor`/`Structural`
   (corruption). The bootstrap authority (the warm-start sidecar decode, `bootstrap.rs:379`) maps
   `UnknownVersion` to a NEW `BootstrapError::ConsensusInputsSchemaUnsupported { found_version,
   required_version }` — a typed terminal "reimport required", separate from `SeedConsensusSidecarDecode`
   (corruption). The node-lifecycle `warm_start_recovery` decode — the FIRST decode of the sidecar
   on the live warm-start path (for geometry), which would otherwise shadow the bootstrap one —
   surfaces the SAME typed `NodeLifecycleError::ConsensusInputsSchemaUnsupported` and prints an
   operator-facing reimport message in `report()`, so BOTH decode sites match. Global v4 bump (the
   DC-CINPUT-05 precedent): every store must be v4; existing v3 stores fail closed with the typed
   error.

5. **Bindings cover the new hashes — automatically:**
   - **Fingerprint/provenance**: `append_seed_epoch_provenance` commits the full sidecar bytes
     (`sidecar_hash`); the bootstrap A3b re-hash check (`bootstrap.rs:370`) is over the full bytes.
     The v4 bytes include the two hashes → covered.
   - **Manifest** (DC-EVIEW-09): `BootstrapManifest.seed_hash = blake2b_256(SeedEpochConsensusInputs bytes)`
     — extending the canonical encoding extends what `seed_hash` commits to; NO manifest format change.
     A v4 seed with a different `genesis_hash` hashes differently → `SeedHashMismatch`.

6. **No runtime fallback** (this slice persists; ECA-2 consumes): ECA-2 reads the two hashes ONLY
   from the recovered sidecar — never `cli.*`, config, genesis file, or a recompute.

## Invariant

- **DC-CINPUT-06 (new):** the durable consensus profile includes `genesis_hash` + `protocol_params_hash`,
  persisted canonically in the v4 `SeedEpochConsensusInputs` sidecar and recovered identically at
  warm-start. No CLI/config/genesis fallback and no recomputation supplies them; a pre-v4 store fails
  closed with a TYPED upgrade/reimport error (`ConsensusInputsSchemaUnsupported {found_version,
  required_version}`), distinct from corruption; the fingerprint/provenance and the manifest `seed_hash`
  binding cover both hashes (transitively, via the extended canonical bytes).
- DC-CINPUT-05 strengthened (the durable consensus-inputs authority now carries the consensus-profile
  hashes; schema 3→4).

## Tests + CI

- v4 round-trips with the two hashes; the canonical-hash / byte-canonical check is sensitive to a
  `genesis_hash` change + a `protocol_params_hash` change.
- a v3 (and v1/v2) buffer fails closed → the typed `ConsensusInputsSchemaUnsupported {found, required}`
  (NOT `MalformedCbor`), distinct from a genuinely corrupt buffer (→ corruption error).
- `merge_seed_epoch_consensus_inputs` populates both hashes from the canonical bundle.
- the manifest `seed_hash` over the v4 seed bytes covers the new hashes (a changed `genesis_hash` →
  different `seed_hash` → `SeedHashMismatch` at `verify_and_import_cert_state`).
- `ci/ci_check_eview_seed_sidecar_v4.sh`.

## Out of scope

ECA-2 (construct `EviewActivationInputs` deterministically, consuming the recovered hashes), ECA-3
(the atomic authority swap), ECA-4 (warm-start recovery of the promoted authority), ECA-5 (live
proof). This slice only makes the consensus-profile hashes durable + recoverable, fail-closed.
