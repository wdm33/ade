# Cluster/Slice Plan — PHASE4-N-J Persistent ledger snapshot encoder

**Status**: cluster-plan phase complete; awaiting `/cluster-doc`
**HEAD pin**: `e509886`
**Date**: 2026-05-26
**Source**: `docs/planning/persistent-snapshot-encoder-invariants.md`

## Cluster Index (Dependency Order)

1. **PHASE4-N-J — Persistent ledger snapshot encoder** — primary
   invariant: `(LedgerState, PraosChainDepState)` round-trips
   through a canonical, deterministic, version-tagged + fingerprint-
   embedded byte encoder; `PersistentSnapshotCache` is observationally
   equivalent to `InMemorySnapshotCache` for the same admitted state.

Closes PHASE4-N-I's deferred `DC-CONS-21` open_obligation. Restart-
safe rollback becomes possible.

## PHASE4-N-J — Persistent encoder

- **Primary invariant**: snapshot bytes are canonical (BTreeMap
  iteration only); version-tagged (u32 == 1); fingerprint-embedded;
  round-trip-preserving (`fingerprint(decode(encode(s))) ==
  fingerprint(s)`); single-authority (one encoder/decoder pair per
  sub-state and per combined snapshot).

- **TCB partition**:
  - **BLUE (new)**: `ade_ledger::snapshot::{error, chain_dep,
    utxo_state, cert_state, epoch_state, gov_state, assemble,
    combined}` — sub-state encoders/decoders + combined framing.
  - **GREEN (new)**: `ade_runtime::rollback::persistent_cache` —
    `PersistentSnapshotCache` impl of `SnapshotReader`. Extended
    `snapshot_writer.rs` with `maybe_capture_snapshot_persistent`.
  - **RED (unchanged)**: `ade_runtime::chaindb::SnapshotStore`
    trait + impls (existing N-D).

- **Cluster Exit Criteria**:

  - **CE-N-J-1** — `PraosChainDepState` encode/decode round-trip
    (BLUE). `encode_chain_dep` + `decode_chain_dep` pair; per-field
    round-trip equality. Foundation for the framing pattern.

  - **CE-N-J-2** — `UTxOState` encode/decode (BLUE). Deterministic
    BTreeMap traversal of TxIn → TxOut entries. Round-trip equality.

  - **CE-N-J-3** — `CertState` encode/decode (BLUE). Delegation +
    pool + DRep round-trip equality.

  - **CE-N-J-4** — `EpochState` encode/decode (BLUE). Epoch + slot
    + reserves + treasury + block_production + epoch_fees +
    snapshots round-trip.

  - **CE-N-J-5** — `ConwayGovState` + `ProtocolParams` +
    `ConwayOnlyDepositParams` encode/decode (BLUE). Option-typed
    fields encoded as `null | bytes`; Rational as `[num, den]`.

  - **CE-N-J-6** — `encode_ledger_state` + `decode_ledger_state`
    (BLUE assemble). Composes the sub-state encoders. Field order
    matches `ade_ledger::fingerprint::fingerprint`'s walk.

  - **CE-N-J-7** — `encode_snapshot` + `decode_snapshot` (BLUE
    combined). Version tag (u32 == 1) + embedded fingerprint +
    cross-check. Flips `DC-STORE-08` + `DC-STORE-09` + `CN-STORE-08`
    to enforced.

  - **CE-N-J-8** — `PersistentSnapshotCache` + cross-impl
    equivalence (GREEN + integration). Closes `DC-CONS-21`.

- **Slices**:

  - **S1** — `PraosChainDepState` encode/decode +
    `SnapshotEncodeError` / `SnapshotDecodeError` closed sums.
    Smallest sub-state; sets the encoder pattern.
  - **S2** — `UTxOState` encode/decode. BTreeMap<TxIn, TxOut>
    traversal.
  - **S3** — `CertState` encode/decode. Delegation +
    PoolState + DRepState.
  - **S4** — `EpochState` encode/decode. Includes `SnapshotState`
    (mark/set/go).
  - **S5** — `ConwayGovState` + `ProtocolParameters` +
    `ConwayOnlyDepositParams` encode/decode.
  - **S6** — `encode_ledger_state` + `decode_ledger_state` assemble.
  - **S7** — `encode_snapshot` + `decode_snapshot` with version
    + fingerprint framing. CI gate
    `ci/ci_check_snapshot_encoder_closure.sh`.
  - **S8** — `PersistentSnapshotCache` + cross-impl equivalence
    test + close `DC-CONS-21`.

- **CE coverage matrix**:

| CE | Slice | Registry IDs flipped to `enforced` on close |
|----|----|----|
| CE-N-J-1 | S1 | *(foundation)* |
| CE-N-J-2 | S2 | *(per-sub-state round-trip)* |
| CE-N-J-3 | S3 | *(per-sub-state round-trip)* |
| CE-N-J-4 | S4 | *(per-sub-state round-trip)* |
| CE-N-J-5 | S5 | *(per-sub-state round-trip)* |
| CE-N-J-6 | S6 | *(assemble round-trip)* |
| CE-N-J-7 | S7 | DC-STORE-08, DC-STORE-09, CN-STORE-08 |
| CE-N-J-8 | S8 | DC-CONS-21 (removes open_obligation) |

3 new entries enforce at S7; DC-CONS-21 closes at S8. 5 existing
rules strengthened on close (T-DET-01, T-ENC-01, DC-CONS-20,
DC-CONS-22, CN-STORE-07).
