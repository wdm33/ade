# Cluster PHASE4-N-J — Persistent ledger snapshot encoder (closes DC-CONS-21)

> **Status:** Planning artifact (non-normative). Strengthens
> `T-DET-01`, `T-ENC-01` (canonical persisted/replay byte path —
> not Cardano hash-critical wire byte), `DC-CONS-20`, `DC-CONS-22`,
> `CN-STORE-07`. Introduces `DC-STORE-08`, `DC-STORE-09`,
> `CN-STORE-08` as enforced. Closes `DC-CONS-21` (removes
> `open_obligation`).

## Primary invariant

> Snapshot bytes are canonical (BTreeMap iteration only); version-
> tagged (u32 == 1); fingerprint-embedded; round-trip-preserving;
> single-authority. `PersistentSnapshotCache` and
> `InMemorySnapshotCache` produce snapshot triples with equal
> `LedgerFingerprint.combined` for the same admitted state.

## Scope (Path A discipline)

- **Conway-only encoder.** Pre-Conway era → `EraNotSupported` on
  both encode and decode. Pre-Conway support is a future cluster.
- **Schema version = 1.** Bytes start with `u32` version tag.
- **No eviction.** Operational; out of scope.
- **8 slices** with `UTxOState` as its own dedicated slice (S2).

## Grounding (verified at HEAD `e509886`)

- **`ade_ledger::fingerprint::fingerprint`** — already walks every
  field of `LedgerState` deterministically. Encoder mirrors this
  walk; field order identical.
- **`ade_codec::cbor::*`** — primitives ready
  (`write_uint_canonical`, `write_bytes_canonical`,
  `write_array_header`, `write_map_header`, `write_text_canonical`,
  `write_null`, plus `read_*` counterparts; `ContainerEncoding::Definite`).
- **`ade_runtime::chaindb::SnapshotStore`** — N-D trait + impls
  ready (`put_snapshot`, `get_snapshot`, `latest_snapshot`,
  `list_snapshot_slots`, `delete_snapshot`).
- **`ade_runtime::rollback::InMemorySnapshotCache`** — N-I in-
  memory `SnapshotReader` impl; the persistent variant must be
  observationally equivalent.

## Slice index

| Slice | Scope | TCB |
|----|----|----|
| S1 | `PraosChainDepState` encoder/decoder + closed error sums | BLUE |
| S2 | `UTxOState` encoder/decoder (BTreeMap traversal) | BLUE |
| S3 | `CertState` encoder/decoder | BLUE |
| S4 | `EpochState` + `SnapshotState` encoder/decoder | BLUE |
| S5 | `ProtocolParameters` + `ConwayGovState` + `ConwayOnlyDepositParams` | BLUE |
| S6 | `encode_ledger_state` + `decode_ledger_state` assemble | BLUE |
| S7 | `encode_snapshot` + `decode_snapshot` framing | BLUE |
| S8 | `PersistentSnapshotCache` + cross-impl equivalence | GREEN + test |

## Forbidden during this cluster

- HashMap/HashSet/wall-clock/tokio/rand/floats in any
  `ade_ledger::snapshot::*` module.
- Any `pub fn` returning `Vec<u8>` from `&LedgerState` or
  `&PraosChainDepState` outside the canonical encoder site
  (CN-STORE-08).
- Sub-state slices merging "encoder mostly done, decoder later" —
  every sub-state slice must prove `decode(encode(s)) ==
  s-equivalent`.
- Pre-Conway encode/decode without `EraNotSupported`.

## Replay obligations introduced by this cluster

- New canonical replay surface: snapshot bytes.
- `T-DET-01.strengthened_in += "PHASE4-N-J"` (snapshot bytes are
  a new authoritative-deterministic surface).
- `T-ENC-01.strengthened_in += "PHASE4-N-J"` (canonical
  persisted/replay byte path for internal snapshot evidence —
  **not** Cardano wire-byte hash-critical).
- `DC-CONS-20.strengthened_in += "PHASE4-N-J"` (rollback atomicity
  now restart-safe).
- `DC-CONS-22.strengthened_in += "PHASE4-N-J"` (replay-forward
  over persistent snapshots equals direct-apply).
- `CN-STORE-07.strengthened_in += "PHASE4-N-J"` (single
  materialize authority's input source now persistent).
