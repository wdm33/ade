# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, **55 CI checks** at HEAD (`f15102f`).
> Reads CODEMAP for the module list and TCB colors; reads the invariant
> registry (`docs/ade-invariant-registry.toml` — **209 entries**) for
> rule IDs; reads the Phase 4 cluster plan
> (`docs/active/phase_4_cluster_plan.md`), the closed N-D / N-A / N-B /
> N-E / N-C / N-G / N-H / N-I / B1 / B2 / B3 / B4 / B5 cluster docs,
> the OQ5 / COMMITTEE / DREP / ENACTMENT-COMMITTEE-FIDELITY /
> ENACTMENT-COMMITTEE-WRITEBACK / PROPOSAL-PROCEDURES-DECODE cluster
> docs, and the **just-closed PHASE4-N-J cluster doc + S1..S8 slice
> docs** (`docs/clusters/PHASE4-N-J/cluster.md` + `N-J-S{1..8}.md`).
>
> **This is the PHASE4-N-J FULL CLOSE refresh (HEAD `f15102f`).** The
> previous SEAMS (HEAD `75f75da`) pinned the PHASE4-N-I full-close
> state and surfaced **persistent ledger snapshot encoding** as the
> highest-priority candidate seam (`DC-CONS-21` declared,
> `open_obligation = persistent_ledger_snapshot_encoding_follow_on_cluster`).
> Eight N-J slices have landed between that revision and this one and
> close that obligation:
>
> 1. **N-J-S1** ships the BLUE `PraosChainDepState` encoder/decoder
>    `ade_ledger::snapshot::chain_dep::{encode_chain_dep,
>    decode_chain_dep}` + closed `SnapshotEncodeError` /
>    `SnapshotDecodeError` / `StructuralReason` sums in
>    `ade_ledger::snapshot::error`. Canonical CBOR; BTreeMap
>    iteration only; definite-length containers.
> 2. **N-J-S2** ships the BLUE `UTxOState` encoder/decoder
>    `ade_ledger::snapshot::utxo_state::{encode_utxo_state,
>    decode_utxo_state}` — `BTreeMap<TxIn, TxOut>` traversal.
> 3. **N-J-S3** ships the BLUE `CertState` encoder/decoder
>    `ade_ledger::snapshot::cert_state::{encode_cert_state,
>    decode_cert_state}`.
> 4. **N-J-S4** ships the BLUE `EpochState` (+ `SnapshotState`)
>    encoder/decoder
>    `ade_ledger::snapshot::epoch_state::{encode_epoch_state,
>    decode_epoch_state}`.
> 5. **N-J-S5** ships the BLUE `ConwayGovState` +
>    `ProtocolParameters` + `ConwayOnlyDepositParams`
>    encoder/decoder triple in
>    `ade_ledger::snapshot::gov_state::{encode_gov_state,
>    decode_gov_state, encode_pparams, decode_pparams,
>    encode_conway_deposit_params, decode_conway_deposit_params}`.
> 6. **N-J-S6** ships the BLUE composite `LedgerState`
>    encoder/decoder
>    `ade_ledger::snapshot::ledger::{encode_ledger_state,
>    decode_ledger_state}` — assembles S2–S5 sub-state encoders in
>    canonical field order matching `ade_ledger::fingerprint`'s
>    deterministic field walk.
> 7. **N-J-S7** ships the BLUE combined-snapshot framing layer
>    `ade_ledger::snapshot::framing::{encode_snapshot,
>    decode_snapshot, SCHEMA_VERSION}` plus CI gate
>    `ci/ci_check_snapshot_encoder_closure.sh`. Wire layout:
>    `array(4)[u32 version (== 1), bytes(32) source_fingerprint,
>    bytes ledger_state_bytes, bytes chain_dep_bytes]`. Decoder
>    verifies the version tag BEFORE any payload work (DC-STORE-09)
>    and recomputes + verifies the fingerprint AFTER decode
>    (DC-STORE-08). Registry rules `DC-STORE-08`, `DC-STORE-09`,
>    `CN-STORE-08` flip to `enforced`.
> 8. **N-J-S8** ships the GREEN
>    `ade_runtime::rollback::persistent_cache::PersistentSnapshotCache<'a,
>    S: SnapshotStore + ?Sized>` — a pure adapter that implements
>    the BLUE `SnapshotReader` trait (from N-I-S1) over the existing
>    `SnapshotStore` trait (from N-D). Provides:
>     - `capture(slot, &ledger, &chain_dep)` — encodes via
>       `framing::encode_snapshot` and `put_snapshot`s the bytes;
>     - `impl SnapshotReader::nearest_le(target_slot)` — walks
>       `SnapshotStore::list_snapshot_slots()`, picks the largest
>       ≤ target, decodes via `framing::decode_snapshot`, surfaces
>       `None` on decode failure (decode error is treated as "no
>       usable snapshot here", not as a panic);
>     - `PERSISTENT_CACHE_SCHEMA_VERSION` re-export of
>       `framing::SCHEMA_VERSION` so out-of-crate consumers can
>       assert the wire version without depending on
>       `ade_ledger::snapshot::framing`;
>     - closed `PersistentCacheError { Encode | Decode | Store }`
>       sum.
>    Cross-impl equivalence proven via
>    `persistent_cache_matches_in_memory_cache_semantics` — for the
>    same admit sequence, persistent and in-memory caches return
>    identical `(slot, LedgerState, PraosChainDepState)` triples
>    across every probe. Registry rule `DC-CONS-21` **flips from
>    `declared` to `enforced`** with the `open_obligation` removed;
>    `strengthened_in` gains `PHASE4-N-J`.
>
> **THE KEY FULL-CLOSE DELTAS.** The prior SEAMS revision flagged
> "Persistent ledger snapshot encoding" as **the highest-priority
> remaining candidate seam** (closure of `DC-CONS-21` open obligation
> on the persistent-encoder follow-on cluster). PHASE4-N-J closes it.
> One §1 / §3 candidate row flips from "next-cluster seam (HIGHEST
> PRIORITY)" to "wired & closed":
>
> - **Persistent ledger snapshot encoding —
>   `(LedgerState, PraosChainDepState) <-> bytes`** → wired via
>   `ade_ledger::snapshot::framing::{encode_snapshot,
>   decode_snapshot}` (BLUE single-authority chokepoint pair) +
>   `ade_runtime::rollback::PersistentSnapshotCache` (GREEN
>   `SnapshotReader` impl over `SnapshotStore`), defended by
>   `ci_check_snapshot_encoder_closure.sh`. `materialize_rolled_back_state`
>   is unchanged — `PersistentSnapshotCache` drops in alongside
>   `InMemorySnapshotCache` as the second production
>   `SnapshotReader` impl.
>
> Counts at this refresh: **+1 CI script** (54 → 55:
> `ci_check_snapshot_encoder_closure.sh`); **+3 registry rules**
> introduced (`DC-STORE-08` `enforced`, `DC-STORE-09` `enforced`,
> `CN-STORE-08` `enforced`); **1 carried rule strengthened + closed**
> (`DC-CONS-21` gains `strengthened_in += PHASE4-N-J` and
> `open_obligation` removed — status flips from `declared` to
> `enforced`); **+1 new BLUE submodule tree** under
> `ade_ledger::snapshot::{chain_dep, utxo_state, cert_state,
> epoch_state, gov_state, ledger, framing, error}` (8 files), with
> `framing.rs` hosting the SOLE `pub fn` pair for combined-snapshot
> bytes (CN-STORE-08); **+1 new GREEN submodule** in
> `ade_runtime::rollback::persistent_cache`
> (`PersistentSnapshotCache`); **+1 new versioned schema seam**
> (`framing::SCHEMA_VERSION: u32 = 1` — the future-migration anchor
> for snapshot wire-format evolution); **0 new operator-action probe
> binaries** at this HEAD — snapshot encoding is wholly internal
> authority with no Tier-1 wire-format counterpart. Total invariant
> registry: **209 entries** (206 → 209). **No new explicit
> carried-forward open obligation surfaced by N-J** — snapshot
> eviction (OQ-5 from N-I) and the carried operator-action live
> obligations (CE-N-C-8 / CE-N-G-8 / CE-N-H-6) remain the named
> follow-on candidates.

Ade is a Cardano block-producing node. Its closure surface is dominated
by two facts:

1. The Cardano protocol fixes wire bytes and hashes for hash-critical
   paths (Tier 1 — must-conform). New work that touches those bytes
   has essentially no degrees of freedom.
2. Everything operator-facing — storage layout, query API, telemetry,
   packaging — is Tier 5: deliberate divergence "in our own image"
   (per `docs/active/CE-79_tier5_addendum.md`).

This document names where the system opens and where it stays closed.

**PHASE4-N-J is fully closed at this HEAD.** Restart-safe rollback is
now possible: the BLUE encoder/decoder pair
`ade_ledger::snapshot::framing::{encode_snapshot, decode_snapshot}` is
the single canonical authority for `(LedgerState,
PraosChainDepState) <-> bytes`, version-tagged + fingerprint-embedded,
and the GREEN `PersistentSnapshotCache` bridges it to the existing
`SnapshotStore` trait (N-D). `materialize_rolled_back_state` (N-I) now
has two production `SnapshotReader` impls — in-memory (warm cache)
and persistent (restart-safe). Scope: **Conway-only** at the encoder
boundary; pre-Conway → `SnapshotEncodeError::EraNotSupported` /
`SnapshotDecodeError::EraNotSupported` (same Path A discipline as N-I).

**PHASE4-N-I remains fully closed** (carried; `DC-CONS-21`
`open_obligation` now removed). **PHASE4-N-H remains fully closed**
(carried). **PHASE4-N-G remains fully closed** (carried).
**PHASE4-N-C remains fully closed** (carried).
**PHASE4-N-E remains fully closed** (carried).
**PROPOSAL-PROCEDURES-DECODE remains fully closed** (carried).
**PHASE4-B3..B5, OQ5 / COMMITTEE / DREP /
ENACTMENT-COMMITTEE-WRITEBACK** all remain closed (carried).

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative
> pipelines. At HEAD there remain **eight** fully-wired *external*
> ingress surfaces (block bytes, Plutus script bytes, snapshot bytes,
> Ouroboros mux frames, genesis JSON bundles, chain-selector stream
> inputs, the N-E wire-level mempool ingress, and the N-H receive-side
> N2N peer ingress). **PHASE4-N-J adds no new external ingress
> surface** — persistent snapshot encoding is wholly internal: it
> reduces the in-memory pair `(LedgerState, PraosChainDepState)` to
> canonical bytes for the existing `SnapshotStore` trait and reads
> back from it. The bytes never leave the local node; no peer or
> client produces or consumes them.
>
> **N-J does, however, formalize an INTERNAL canonical-byte surface**:
> `[u32 version][bytes(32) fingerprint][bytes ledger][bytes chain_dep]`
> per `ade_ledger::snapshot::framing`. This is the project's first
> persisted non-Cardano-wire canonical byte format, gated by a
> versioned schema (`SCHEMA_VERSION: u32 = 1`) and the
> single-authority CI check `ci_check_snapshot_encoder_closure.sh`.

### Surface: Receive-side N2N peer ingress (carried from N-H + N-I; **rollback half now restart-safe via N-J's persistent encoder**)

```
Surface: A peer-originated chain-sync ForkChoiceSignal
         (RollForward { header_bytes, tip }
         | RollBackward { target_point, tip }
         | Intersected | NoIntersection)
         OR a peer-originated block-fetch BatchDeliveryEvent
         (BatchStarted | BlockDelivered { block_bytes }
         | NoBlocks | BatchCompleted)
         delivered by a real cardano-node peer over N2N mux
Reduces to: ReceiveEffect — closed 4-variant sum
            { Admitted { slot, hash } | Cached { slot, hash }
            | RolledBack { to_slot } | NoOp { ... } }
            — OR ReceiveError — closed 4-variant sum
            { HeaderBodyMismatch | Validity(BlockValidityError)
            | RollbackOutOfScope { target_point }
            | ChainDb(ChainWriteError) }
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. RED transport (ade_network::mux::transport) decodes mux frame
  2. BLUE chain-sync / block-fetch codec (N-A) — decode_*_message
  3. RED dispatcher (ade_runtime::receive::orchestrator)
  4. RED-internal translation: peer-agency message →
     ForkChoiceSignal / BatchDeliveryEvent
  5. GREEN lift (ade_runtime::receive::events_to_state) → ReceiveEvent
  6. BLUE reducer — receive_apply(state, event, chain_write,
     era_schedule, ledger_view, rollback_ctx: Option<&RollbackContext>)
       - RollBackward + Some(ctx): roll_backward composes
         materialize_rolled_back_state(target, ctx.snapshot_reader,
         ctx.block_source, ...) → commit_rollback.
         **N-J effect on this arm:** `ctx.snapshot_reader` may now be
         backed by `PersistentSnapshotCache` (decoding bytes via
         `framing::decode_snapshot`) instead of (or in addition to)
         `InMemorySnapshotCache`. The reducer is agnostic — it sees
         `&dyn SnapshotReader`; the choice of backing impl is the
         orchestrator's. Restart-safe rollback is now possible: a
         process restart loses the in-memory cache but the persistent
         cache survives.
  7. GREEN ChainDb write — ChainDbWriter::write_admitted /
     rollback_to_slot.
  8. RED snapshot-write hook (ade_runtime::rollback::snapshot_writer
     — maybe_capture_snapshot): on each ReceiveEffect::Admitted
     consults cadence policy + captures (ledger, chain_dep) into
     InMemorySnapshotCache when due. **N-J note:** the analogous
     persistent-capture call site is NOT wired into the
     orchestrator at this HEAD — the persistent capture is exposed
     as `PersistentSnapshotCache::capture(slot, ledger, chain_dep)`
     and is called explicitly by tests; the orchestrator-side
     persistent-capture wiring is a forward-looking thin RED call
     site (see candidate below).
Cross-surface state sharing: per-peer state stays fully independent
  (N-H invariant carried). N-J introduces the persistent
  `SnapshotStore` as a NEW shared resource alongside the shared
  ChainDb — but the persistent store's bytes are written/read
  exclusively through `PersistentSnapshotCache` (no parallel path).
```

**Rule (carried + extended in N-J).** `receive_apply` (with
`receive_apply_sequence` as its deterministic driver) remains the
**single receive-side composition root**. `RollbackContext` (N-I) is
the **single receive-side rollback entry point** and its
`snapshot_reader: &'a dyn SnapshotReader` field is **the seam through
which N-J's persistent reader attaches** — without changing the
reducer, the materialize chokepoint, or the commit chokepoint. New
rollback features attach by implementing `SnapshotReader` (e.g. the
N-J `PersistentSnapshotCache`); the chokepoint **never moves**:
`materialize_rolled_back_state` is the SOLE `pub fn` returning
`(LedgerState, PraosChainDepState)` in the rollback module tree
(CN-STORE-07 — CI-defended), and `encode_snapshot` /
`decode_snapshot` is the SOLE `pub fn` pair encoding/decoding that
tuple to/from bytes (CN-STORE-08 — CI-defended, NEW in N-J).

### Surfaces carried unchanged from prior revisions

- **Producer-side chain-sync server-role ingress** (N-G): carried.
- **Producer-side block-fetch server-role ingress** (N-G): carried.
- **Forge-block transition** (N-C): carried.
- **Self-accept broadcast gate** (N-C): carried. `AcceptedBlock` /
  `AdmittedBlock` matched pair unchanged in N-J.
- **Scheduler input ingress** (N-C): carried.
- **Mempool ingress** (Tier-1 wire-level — N-E): carried.
- **Conway tx-body `proposal_procedures` sub-grammar** (PP): carried.
- **Single-tx validity** (B2): carried.
- **Mempool admission** (Tier-1 gate — B2): carried.
- **Full block validity** (B1): carried. **N-J usage:** unchanged
  from N-I (`materialize_rolled_back_state` composes `block_validity`
  as the per-block step in the replay-forward fold; the
  fold-input snapshot may now come from the persistent reader).
- **Block bytes, Plutus script bytes, Snapshot bytes (N-D layer
  unchanged), Consensus-input extraction, Ouroboros mux frames,
  Genesis JSON bundles, Chain-selector stream inputs**: all carried.

### Receive-side rollback authority (carried from N-I; **persistent reader added in N-J**)

The receive-side rollback authority surface from N-I is unchanged at
the chokepoint level: `materialize_rolled_back_state` +
`commit_rollback` + `RollbackContext` + `ChainDbWrite::rollback_to_slot`.
**What N-J adds is a second production `SnapshotReader` impl**
(`PersistentSnapshotCache`), making rollback restart-safe. The
trait was deliberately written in N-I as `&dyn SnapshotReader` so the
persistent impl drops in without touching BLUE code — that
extension point is now exercised.

### Candidates — surfaces not yet wired

- **N-J-S1..S8 WIRED AND CLOSED the prior revision's "Persistent
  ledger snapshot encoding" candidate** — removed (now
  `encode_snapshot` + `decode_snapshot` + `PersistentSnapshotCache`
  + 3 registry rules + 1 CI gate).
- **NEW CANDIDATE (flagged by N-J close): orchestrator-side
  persistent-capture wiring.** N-J exposes
  `PersistentSnapshotCache::capture(slot, ledger, chain_dep)` but
  does NOT wire it into the per-peer post-admission hook
  (`maybe_capture_snapshot` still talks only to
  `InMemorySnapshotCache`). The next cluster wires the persistent
  capture path — either by extending `maybe_capture_snapshot` to
  accept a `&PersistentSnapshotCache` alongside the in-memory cache,
  or by introducing a sibling RED hook (`maybe_capture_persistent_snapshot`).
  Tier-5; the BLUE invariants do not change.
- **NEW CANDIDATE (flagged by N-J close): N-J followup — snapshot
  eviction policy.** Carried from N-I (OQ-5). With persistent
  snapshots now landed, the eviction concern doubles: the
  in-memory cache and the persistent `SnapshotStore` both grow
  monotonically at this HEAD. The persistent store already has a
  `SnapshotStore::delete_snapshot(slot)` method (N-D), but no
  policy decides when to call it. Tier-5 operational concern.
- **NEW CANDIDATE (flagged by N-J close): pre-Conway snapshot
  encoder.** N-J ships Conway-only at the encoder boundary
  (pre-Conway → `EraNotSupported` structurally). A future cluster
  could widen this to Babbage and earlier eras; today the carve-out
  is documented at the type level (no `open_obligation` registry
  entry because the operational use case for pre-Conway snapshots
  is unclear — rollback target windows are bounded and Conway is
  the live era).
- **NEW CANDIDATE (flagged by N-J close): snapshot schema migration
  v1 → v2.** `framing::SCHEMA_VERSION: u32 = 1` is the explicit
  future-migration anchor. The first new field appended to the
  framing wire format (e.g. an explicit schema-version field for
  the inner `ledger_state_bytes` payload, or a snapshot-of-snapshot
  pointer for incremental diffs) will bump this to 2; existing v1
  bytes remain readable until a future cluster ratifies dropping
  legacy support. Today there is no v2 — the candidate exists only
  to name the seam.
- **CANDIDATE (carried from N-I — OQ-4 lock; now further enabled by
  N-J's restart-safe rollback): multi-peer fork choice.** Now
  doubly-unblocked: N-I gave us rollback within a session; N-J
  gives us rollback across restarts. The Praos longest-chain
  selection across competing `PerPeerReceiveState[]` follows the
  same shape sketched in the N-I revision; no new attachment
  surface specifically introduced by N-J.
- **CANDIDATE (carried from N-H/N-I): N2C local-chain-sync receive
  surface.** Unchanged at N-J.
- **CE-N-H-6 live-evidence — still
  `blocked_until_operator_peer_available`** (carried).
- **CE-N-G-8 / CE-N-C-8 live-evidence — still
  `blocked_until_operator_*_available`** (carried).
- **PROPOSAL-PROCEDURES-DECODE remains closed** (carried). The four
  PP open obligations remain separable candidate seams (carried).
- **PHASE4-N-E remains closed** (carried).

| Cluster | Surface | Expected reduction target | Expected chokepoint | Confidence |
|---------|---------|---------------------------|---------------------|------------|
| **PHASE4-N-J** *(FULLY CLOSED at this HEAD — mechanical close; Conway-only encoder scope; pre-Conway → `EraNotSupported` structurally)* | **Persistent ledger snapshot encoding: `(LedgerState, PraosChainDepState) <-> bytes` via the canonical `encode_snapshot` / `decode_snapshot` pair + a `PersistentSnapshotCache` impl of `SnapshotReader` over `SnapshotStore`** | `(LedgerState, PraosChainDepState)` → `Vec<u8>` (encoder); `&[u8]` → `(LedgerState, PraosChainDepState)` (decoder); `(SlotNo, &dyn SnapshotStore)` → `Option<(SlotNo, LedgerState, PraosChainDepState)>` (persistent reader) | **DONE:** `ade_ledger::snapshot::{chain_dep, utxo_state, cert_state, epoch_state, gov_state, ledger, framing, error}` (BLUE; 8 files); `ade_ledger::snapshot::framing::{encode_snapshot, decode_snapshot, SCHEMA_VERSION}` (the SOLE `pub fn` pair encoding/decoding `(LedgerState, PraosChainDepState)` bytes per CN-STORE-08); `ade_runtime::rollback::persistent_cache::PersistentSnapshotCache<'a, S: SnapshotStore + ?Sized>` (GREEN single production impl of the second `SnapshotReader` backend); `PersistentCacheError` (closed `Encode | Decode | Store` sum); `PERSISTENT_CACHE_SCHEMA_VERSION` (re-export). CI gate `ci_check_snapshot_encoder_closure.sh`. Registry rules `DC-STORE-08`, `DC-STORE-09`, `CN-STORE-08` (`enforced`); `DC-CONS-21` flipped from `declared` to `enforced` with `strengthened_in += PHASE4-N-J` and `open_obligation` removed. Cross-impl equivalence test `persistent_cache_matches_in_memory_cache_semantics`. | **wired & closed in PHASE4-N-J (mechanical half — wholly internal authority, no Tier-1 wire-format counterpart; the bytes are local-only persisted state, gated by version tag + fingerprint cross-check)** |
| **NEW CANDIDATE — Orchestrator-side persistent-capture wiring** *(flagged by N-J close)* | **Per-peer post-admission hook that also writes to `PersistentSnapshotCache`** | Extension of `maybe_capture_snapshot` to accept `&PersistentSnapshotCache` alongside `&mut InMemorySnapshotCache`, or sibling hook `maybe_capture_persistent_snapshot` | Likely a Tier-5 operational/RED layer change in `ade_runtime::rollback::snapshot_writer`. No BLUE chokepoint moves. | **candidate (next-cluster seam; surface; sequenced naturally before any restart-mode test scenario)** |
| **NEW CANDIDATE — Snapshot eviction policy** *(carried from N-I; **now doubled by N-J — applies to both caches**)* | **Bounded cache size for in-memory + bounded retention for persistent `SnapshotStore`** | `evict_older_than(slot)` on both `InMemorySnapshotCache` and `PersistentSnapshotCache` (latter forwards to `SnapshotStore::delete_snapshot`) | Tier-5 operator-tunable; must remain replay-deterministic. | **candidate (next-cluster seam; surface)** |
| **NEW CANDIDATE — Pre-Conway snapshot encoder** *(flagged by N-J close — explicit Conway-only scope at N-J)* | **Widen encoder/decoder to Babbage and earlier eras** | Same canonical layout, era-tagged dispatch in `encode_ledger_state` / `decode_ledger_state` | Tier-5; no current operational need; flagged as a follow-on if rollback target windows ever exceed era boundaries. | **candidate (low-priority next-cluster seam; surface)** |
| **NEW CANDIDATE — Snapshot schema migration v1 → v2** *(flagged by N-J close — `framing::SCHEMA_VERSION` is the explicit anchor)* | **Future evolution of the snapshot wire format** | New `SCHEMA_VERSION: u32 = 2`; decoder dispatches on tag; v1 readers remain valid until a future cluster ratifies drop | Tier-5; closed-version-gated. No present need; the seam is documented to set the migration discipline now. | **candidate (no current cluster; the seam is documented so future evolution does not improvise)** |
| **CANDIDATE — Multi-peer fork choice (Praos longest-chain across competing peers)** *(carried from N-I; now doubly-enabled by N-J — rollback is restart-safe)* | Carried. | Carried. | **candidate (next-cluster seam; surface)** |
| **CANDIDATE — N2C local-chain-sync receive surface** *(carried from N-I)* | Carried. | Carried. | **candidate (next-cluster seam; surface)** |
| **CE-N-H-6 (cross-cluster obligation carried)** | **Live N2N follow-mode admission** | Carried. | Carried. | **carried (`blocked_until_operator_peer_available`)** |
| **CE-N-G-8 (cross-cluster obligation carried)** | **Live N2N block-fetch acceptance (Ade serving)** | Carried. | Carried. | **carried (`blocked_until_operator_peer_available`)** |
| **CE-N-C-8 (cross-cluster obligation carried)** | **Live N2N block-fetch acceptance (Ade forging)** | Carried. | Carried. | **carried (`blocked_until_operator_stake_available`)** |
| **N-C+ (declared non-goal in N-C cluster doc; OQ-4 lock)** | **TPraos producer (Shelley..Alonzo full-block production)** | Carried. | Carried. | candidate (declared non-goal) |
| **CE-NODE-N2C-LTX (cross-cluster obligation carried from N-E)** | **Live N2C UDS server + N2N bulk-tx inbound listener** | Carried. | Carried. | **deferred cross-cluster obligation (NOT an open seam in N-E)** |
| **PP OQ-1..OQ-4 (separable seams)** | various | Carried. | Carried. | candidate (carried) |
| B+ (full tx UTxO scope) | Full-scope single-tx validity over real resolved UTxO | Carried. | Carried. | candidate |
| B+ (Conway body witness depth) | Conway block-body vkey-witness closure | Carried. | Carried. | candidate (B2-carried) |
| B+ (pre-Conway tx) | Pre-Conway single-tx validity | Carried. | Carried. | candidate |
| B1+ (pre-Babbage block) | TPraos full-block validity (Shelley..Alonzo) | Carried. | Carried. | candidate |
| N-F | LSQ semantic dispatch (LocalStateQuery payloads) | Internal Query enum | Single dispatch fn over opaque-bytes payloads | candidate |
| N-F | LocalTxMonitor semantic dispatch | Mempool-snapshot Query/Reply enums | Single dispatch fn over opaque-bytes payloads | candidate |
| N-B+ | Live cardano-node session driver | `StreamInput` translated from `ChainSyncMessage` + `BlockFetchMessage` | Composition layer in `ade_core_interop` | candidate |

### Operator-action evidence (live-wire artifacts — not BLUE seams)

The Ade workspace closes Tier-1 wire-level seams in two halves: a
mechanical / GREEN half (code + harness + CI gates that the workspace
itself can certify on every push) and a **live-wire operator-action
half** (a real peer / client at the other end of a real socket
producing bytes Ade has never seen).

**At this HEAD two live-evidence logs remain committed**, three
cross-cluster obligations remain `blocked_until_operator_*_available`,
and one cross-cluster obligation is carried from N-E. **N-J added
no new operator-action obligation** — persistent snapshot encoding is
wholly internal (no Tier-1 wire-format counterpart that requires a
real peer to certify).

| Procedure | Evidence-log artifact | Status at HEAD | What it asserts | TCB |
|-----------|----------------------|----------------|------------------|-----|
| `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_PROCEDURE.md` | `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_<date>.log` | **CAPTURED** (carried) | Real cardano-node N-B follow-mode tip agreement | RED operator action |
| `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_PROCEDURE.md` | `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_2026-05-25.log` | **CAPTURED** (carried) | Outbound-client probe against a real preprod N2N relay | RED operator action |
| `docs/clusters/completed/PHASE4-N-E/CE-N-E-7_PROCEDURE.md` | (deferred) `CE-NODE-N2C-LTX_<date>.log` | **DEFERRED to CE-NODE-N2C-LTX** | Real `cardano-cli transaction submit` to Ade over the N2C UDS | RED operator action (deferred) |
| `docs/clusters/completed/PHASE4-N-C/CE-N-C-8_PROCEDURE.md` | (pending) `CE-N-C-LIVE_<date>.log` | **`blocked_until_operator_stake_available`** (carried) | Cardano-node accepts an Ade-forged block as the next chain head | RED operator action |
| `docs/clusters/completed/PHASE4-N-G/CE-N-G-8_PROCEDURE.md` | (pending) `CE-N-G-LIVE_<date>.log` | **`blocked_until_operator_peer_available`** (carried) | A real cardano-node peer issuing `RequestRange` accepts Ade-served bytes | RED operator action |
| `docs/clusters/completed/PHASE4-N-H/CE-N-H-6_PROCEDURE.md` | (pending) `CE-N-H-LIVE_<date>.log` | **`blocked_until_operator_peer_available`** (carried) | Ade follower fed RollForward + BlockDelivered from a real cardano-node peer produces a matching ChainDb tip | RED operator action |

**Operator-action probe binaries (RED — `ade_core_interop::bin::*`).**
At this HEAD there are still **five** such binaries (no N-J addition):

| Binary | Slice | Live-evidence target | Status |
|--------|-------|----------------------|--------|
| `live_consensus_session` (PHASE4-N-B) | N-B | CE-N-B-6 | captured |
| `live_tx_submission_session` (PHASE4-N-E S6) | N-E S6 | CE-N-E-6 | captured |
| `live_block_production_session` (PHASE4-N-C S7) | N-C S7 | CE-N-C-8 | blocked_until_operator_stake_available |
| `live_block_fetch_session` (PHASE4-N-G S7) | N-G S7 | CE-N-G-8 | blocked_until_operator_peer_available |
| `live_block_follow_session` (PHASE4-N-H S6) | N-H S6 | CE-N-H-6 | blocked_until_operator_peer_available |

**Pattern carried.** Hermetic default + `--connect <peer>` live pass.
**N-J has no new entry in this family** — persistent snapshot encoding
is exercised by the inline test set in `ade_ledger::snapshot::*` plus
the cross-impl equivalence test
`persistent_cache_matches_in_memory_cache_semantics` in
`ade_runtime::rollback::persistent_cache`. Restart-mode evidence
would require either an operator-action procedure (kill-and-restart
on a real node) or a stress harness (`KillStrategy<D>`-style); both
are forward-looking and not surfaced by N-J as new obligations.

**These are evidence-log patterns, not BLUE seams.**

User confirmation needed for each candidate at cluster entry. **The
most load-bearing remaining candidates for the bounty** are
**CE-N-C-8** (live cardano-node forge acceptance), **CE-N-G-8** (live
cardano-node block-fetch acceptance — Ade-serving counterpart),
**CE-N-H-6** (live cardano-node follow-mode admission — Ade-receiving
counterpart), **multi-peer fork choice** (now doubly-enabled by N-I
+ N-J's restart-safe rollback), **CE-NODE-N2C-LTX** (the deferred
live N2C UDS server + N2N bulk-tx inbound listener), and the four
**PROPOSAL-PROCEDURES-DECODE open obligations**.

---

## 2. Data-Only vs. Authoritative Layers

Ade has **nineteen** authoritative domains. **PHASE4-N-J added one
new domain — persistent ledger snapshot encoding authority** — a new
BLUE encoder/decoder chokepoint pair
(`encode_snapshot`/`decode_snapshot`) producing canonical bytes from
`(LedgerState, PraosChainDepState)` and back, plus a GREEN
`SnapshotReader` impl (`PersistentSnapshotCache`) that bridges the
BLUE chokepoint to the existing `SnapshotStore` trait. Prior cluster
narratives are preserved unchanged below.

### Persistent ledger snapshot encoding authority (NEW in PHASE4-N-J)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **BLUE sub-state encoder/decoder pairs (S1–S5)** | `ade_ledger::snapshot::{chain_dep, utxo_state, cert_state, epoch_state, gov_state}` | BLUE | One encoder/decoder pair per sub-state, each the SOLE `pub fn` pair for its sub-state's bytes (in-module single-authority). Canonical CBOR (BTreeMap iteration only, definite-length containers, no `HashMap`/`HashSet`/wall-clock/`tokio`/`rand`/float). Each sub-state's `decode(encode(s))` is byte-identical and fingerprint-equal to the source. |
| **BLUE composite LedgerState encoder/decoder (S6)** | `ade_ledger::snapshot::ledger::{encode_ledger_state, decode_ledger_state}` | BLUE | The **SOLE `pub fn` pair encoding/decoding `LedgerState` bytes in the workspace** (CN-STORE-08 — CI-defended). Assembles S2–S5 sub-state encoders in canonical field order matching `ade_ledger::fingerprint`'s deterministic field walk. |
| **BLUE composite chain-dep encoder/decoder (S1)** | `ade_ledger::snapshot::chain_dep::{encode_chain_dep, decode_chain_dep}` | BLUE | The **SOLE `pub fn` pair encoding/decoding `PraosChainDepState` bytes in the workspace** (CN-STORE-08 — CI-defended). Canonical 9-field array (5 nonces + 3 optional uints + op-cert-counter array). |
| **BLUE combined-snapshot framing (S7)** | `ade_ledger::snapshot::framing::{encode_snapshot, decode_snapshot, SCHEMA_VERSION}` | BLUE | The **SOLE `pub fn` pair encoding/decoding the combined `(LedgerState, PraosChainDepState)` snapshot bytes** (CN-STORE-08 — CI-defended). Wire shape: `array(4)[u32 version (== SCHEMA_VERSION), bytes(32) source_fingerprint, bytes ledger_state_bytes, bytes chain_dep_bytes]`. Decoder verifies version BEFORE payload (DC-STORE-09) and recomputes + verifies fingerprint AFTER decode (DC-STORE-08). Conway-only at encoder; pre-Conway → `EraNotSupported`. |
| **BLUE closed snapshot error sums (S1)** | `ade_ledger::snapshot::error::{SnapshotEncodeError, SnapshotDecodeError, StructuralReason}` | BLUE | `SnapshotEncodeError` has 1 variant (`EraNotSupported { era }`). `SnapshotDecodeError` has 5 variants (`Cbor(CodecError)`, `UnknownVersion { expected, found }`, `FingerprintMismatch { expected, actual }`, `EraNotSupported { era }`, `Structural { reason }`). `StructuralReason` has 9 variants (`ArrayLengthMismatch`, `MapLengthExceeded`, `UnexpectedNull`, `UnexpectedNonNull`, `NonceLengthMismatch`, `PoolIdLengthMismatch`, `Hash32LengthMismatch`, `Hash28LengthMismatch`, `EraTagOutOfRange`). All closed; no `#[non_exhaustive]`; no `String`. |
| **BLUE versioned schema seam (S7)** | `ade_ledger::snapshot::framing::SCHEMA_VERSION: u32 = 1` | BLUE | The **future-migration anchor** for snapshot wire-format evolution. Today `== 1`; a future cluster ratifying a v2 layout bumps to `2`. CI gate enforces this is the SOLE `pub const SCHEMA_VERSION` in `crates/`. |
| **GREEN persistent snapshot cache (S8)** | `ade_runtime::rollback::persistent_cache::PersistentSnapshotCache<'a, S: SnapshotStore + ?Sized>` | GREEN | **The single production persistent impl of `SnapshotReader`.** Pure adapter over any `SnapshotStore`. Borrows the store (lifetime `'a`); holds no in-memory state. Methods: `new(store)`, `capture(slot, &ledger, &chain_dep)` (encode + put), `impl SnapshotReader::nearest_le` (list slots, pick largest ≤ target, get bytes, decode). Decode failures map to `None` (treats corrupt persisted bytes as "no usable snapshot here" — the reader is monotonic-best-effort, not crash-and-burn). |
| **GREEN closed persistent-cache error sum (S8)** | `ade_runtime::rollback::persistent_cache::PersistentCacheError` | GREEN | Closed `Encode(SnapshotEncodeError) | Decode(SnapshotDecodeError) | Store(ChainDbError)` sum. No `String`. Carries upstream errors verbatim so the caller (orchestrator) decides whether to halt or skip. |
| **GREEN persistent schema re-export (S8)** | `ade_runtime::rollback::persistent_cache::PERSISTENT_CACHE_SCHEMA_VERSION: u32` | GREEN | Re-exports `framing::SCHEMA_VERSION` so out-of-crate consumers can assert the cache's wire version without depending on `ade_ledger::snapshot::framing`. Test `persistent_cache_schema_version_mirrors_framing` pins the equality. |
| **CI gate (S7)** | `ci/ci_check_snapshot_encoder_closure.sh` | CI | 1 mechanical gate defending the snapshot encoder authority surface. Enforces: (1) the only `pub fn encode_snapshot` / `decode_snapshot` pair lives at `framing.rs` (CN-STORE-08); (2) the only `pub fn encode_ledger_state` / `decode_ledger_state` pair lives at `ledger.rs` (CN-STORE-08); (3) the only `pub fn encode_chain_dep` / `decode_chain_dep` pair lives at `chain_dep.rs` (CN-STORE-08); (4) the only `pub const SCHEMA_VERSION` lives at `framing.rs` (DC-STORE-09); (5) framing.rs references `FingerprintMismatch` (DC-STORE-08) and `UnknownVersion` (DC-STORE-09) — positive presence of the cross-check paths. Total CI count: 54 → 55. |

**Rule.** This domain has:
- **One BLUE combined-snapshot encoder/decoder chokepoint pair**
  (`encode_snapshot`/`decode_snapshot` — CN-STORE-08 single-authority;
  the SOLE `pub fn` pair encoding/decoding the combined tuple bytes).
- **Two BLUE composite encoder/decoder chokepoint pairs**
  (`encode_ledger_state`/`decode_ledger_state`,
  `encode_chain_dep`/`decode_chain_dep` — each the SOLE `pub fn` pair
  for its half of the tuple).
- **Five BLUE sub-state encoder/decoder pairs** (one per S1–S5
  sub-state — each the SOLE pair for its sub-state).
- **Three BLUE closed error sums** (`SnapshotEncodeError`,
  `SnapshotDecodeError`, `StructuralReason`).
- **One BLUE versioned schema anchor** (`SCHEMA_VERSION: u32 = 1`).
- **One GREEN production persistent reader** (`PersistentSnapshotCache`)
  — single production impl of `SnapshotReader` over `SnapshotStore`.
- **One GREEN closed cache error sum** (`PersistentCacheError`).
- **One GREEN schema re-export** (`PERSISTENT_CACHE_SCHEMA_VERSION`).
- **One CI gate** (`ci_check_snapshot_encoder_closure.sh`).

**THE KEY SEAMS:**

1. **`encode_snapshot` / `decode_snapshot` is the SOLE `pub fn` pair
   encoding/decoding the combined `(LedgerState, PraosChainDepState)`
   tuple bytes** in the workspace (CN-STORE-08). Mirrors
   `materialize_rolled_back_state`'s single-authority discipline
   (CN-STORE-07) at the bytes layer. CI-defended via repo-wide grep
   in `ci_check_snapshot_encoder_closure.sh`.
2. **`encode_ledger_state` / `decode_ledger_state` is the SOLE pair
   for the half-tuple `LedgerState` bytes** in the workspace
   (CN-STORE-08). Composed by `encode_snapshot` /
   `decode_snapshot`. Field order matches `ade_ledger::fingerprint`'s
   deterministic walk.
3. **`encode_chain_dep` / `decode_chain_dep` is the SOLE pair for
   the half-tuple `PraosChainDepState` bytes** in the workspace
   (CN-STORE-08). Composed by `encode_snapshot` /
   `decode_snapshot`.
4. **`SCHEMA_VERSION: u32 = 1` is the SOLE schema-version anchor**
   in the workspace (DC-STORE-09). Decoder rejects unknown versions
   BEFORE payload work — bytes from a future v2 layout fail closed
   on a v1 reader.
5. **Fingerprint cross-check is structurally enforced.** Encoder
   embeds `fingerprint(ledger).combined`; decoder recomputes after
   decode and rejects on mismatch (`SnapshotDecodeError::FingerprintMismatch`).
   Corruption / schema drift / tampering all fail closed
   (DC-STORE-08).
6. **`PersistentSnapshotCache` is a CLOSED extension point** — single
   production impl of the BLUE `SnapshotReader` trait for the
   persistent backend. Pure adapter; borrows the `SnapshotStore`;
   no in-memory state. New persistent backends attach by
   implementing `SnapshotStore` (not by re-implementing
   `SnapshotReader`).
7. **Decode failure maps to `None` at the reader.** Corrupt persisted
   bytes do not panic the rollback path; the reader returns `None`
   (treated by `materialize_rolled_back_state` as "no snapshot
   available" → `RollbackTooDeep`). The decoder still surfaces the
   structured error to direct callers (`PersistentSnapshotCache::capture`
   propagates `PersistentCacheError::Decode` to the operator
   surface).
8. **Conway-only at the encoder boundary** (matches N-I's
   `MaterializeError::EraNotSupported` scope edge). Pre-Conway →
   `EraNotSupported` structurally on both encode and decode. Today
   the live era is Conway; a future cluster widens this if rollback
   target windows ever exceed era boundaries.

**New work** that adds a snapshot encoding feature attaches by:
- Adding a new `SnapshotStore` impl (e.g. a different on-disk
  backend) — `PersistentSnapshotCache` is generic over `S:
  SnapshotStore + ?Sized` and works unchanged.
- Extending the closed `SnapshotEncodeError` / `SnapshotDecodeError`
  / `StructuralReason` arms inside their enum bodies (closed-sum
  extension, version-gated).
- Bumping `SCHEMA_VERSION` to introduce a v2 layout (decoder
  dispatches on tag; v1 readers reject v2 cleanly until they
  upgrade).
- Adding a sibling encoder/decoder for a different state type
  (e.g. mempool snapshot, peer-table snapshot) inside
  `ade_ledger::snapshot::*` — each sibling carries its own
  single-authority CI gate.

— **not** by adding a parallel `(LedgerState, PraosChainDepState)
<-> bytes` encoder anywhere outside `framing.rs`, **not** by
adding a parallel `LedgerState <-> bytes` encoder outside
`ledger.rs`, **not** by adding a parallel `PraosChainDepState <->
bytes` encoder outside `chain_dep.rs`, **not** by declaring a
parallel `SCHEMA_VERSION` constant, **not** by skipping the
version check or fingerprint check in any new decoder.

**Declared non-goals carried from the cluster doc:**
- Pre-Conway snapshot encoding (S7 scope — surfaces structurally
  as `EraNotSupported`; flagged as a low-priority candidate seam).
- Snapshot eviction (OQ-5 from N-I — out of scope; both the
  in-memory cache and the persistent store grow monotonically at
  this HEAD; eviction is the named follow-on candidate).
- Orchestrator-side persistent-capture wiring — the persistent
  cache's `capture` method is exposed and tested but the
  post-admission hook still talks only to `InMemorySnapshotCache`;
  wiring is flagged as a Tier-5 follow-on.
- Schema migration v1 → v2 — `SCHEMA_VERSION = 1` today; the seam
  is documented to set the migration discipline now, but no v2
  bump is planned at this HEAD.

### Receive-side rollback authority (carried unchanged from PHASE4-N-I; **persistent reader impl added in N-J**)

Carried. **N-J note:** the BLUE chokepoint set
(`materialize_rolled_back_state`, `commit_rollback`,
`RollbackContext`, `ChainDbWrite::rollback_to_slot`) is structurally
unchanged at this HEAD. What N-J adds is a **second production
`SnapshotReader` impl** (`PersistentSnapshotCache`) that fits the
existing `&dyn SnapshotReader` extension point in `RollbackContext`.
The N-I in-memory impl (`InMemorySnapshotCache`) and the N-J
persistent impl (`PersistentSnapshotCache`) are observationally
equivalent for the same admit sequence (proven by
`persistent_cache_matches_in_memory_cache_semantics`).

### Receive-side admission authority (carried unchanged from PHASE4-N-H)

Carried. **N-J note:** unchanged; the admit-side reducer arm is
not touched by N-J.

### Producer-side server response authority (carried unchanged from N-G)

Carried.

### Block production authority (carried unchanged from N-C)

Carried.

### Mempool ingress (carried unchanged from N-E)

Carried.

### Conway tx-body `proposal_procedures` sub-grammar authority (carried unchanged from PROPOSAL-PROCEDURES-DECODE)

Carried.

### Conway value-conservation accounting / Conway certificate-state accumulation / Credential discriminant fidelity / Conway governance-cert accumulation / Single-tx validity / Mempool admission / Full block validity / Ledger application / Stake-snapshot projection for consensus / Plutus phase-2 evaluation / Governance ratification & enactment / Mini-protocol wire conformance / Praos consensus runtime

All carried unchanged from the prior revision. **N-J-specific
strengthening:** the `LedgerFingerprint` fold authority (B3/B5) is
now also the **input to the framing layer's fingerprint cross-check**
— `encode_snapshot` reads `fingerprint(ledger).combined` and embeds
it; `decode_snapshot` recomputes the same and rejects mismatches.
Snapshot bytes carry no semantic authority beyond what the
fingerprint already authorizes. `DC-CONS-21.strengthened_in +=
PHASE4-N-J`.

### Where the boundary is enforced

- `ci_check_dependency_boundary.sh` — no BLUE crate may depend on
  RED. N-J added one new edge — `ade_runtime::rollback::persistent_cache`
  imports `ade_ledger::snapshot::framing::{encode_snapshot,
  decode_snapshot, SCHEMA_VERSION}` + `ade_ledger::snapshot::{SnapshotDecodeError,
  SnapshotEncodeError}` + `ade_ledger::rollback::SnapshotReader` +
  `ade_ledger::state::LedgerState`. Same direction (RED/GREEN →
  BLUE) as existing N-C / N-G / N-H / N-I edges; allowed.
- `ci_check_no_async_in_blue.sh` — async forbidden in BLUE. The new
  `ade_ledger::snapshot::*` modules are BLUE; no async.
- **`ci_check_snapshot_encoder_closure.sh`** *(N-J-S7 — CN-STORE-08,
  DC-STORE-08, DC-STORE-09 enforcement)* — repo-wide grep gates:
  (1) `pub fn encode_snapshot` / `decode_snapshot` outside
  `crates/ade_ledger/src/snapshot/framing.rs` → FAIL;
  (2) `pub fn encode_ledger_state` / `decode_ledger_state` outside
  `crates/ade_ledger/src/snapshot/ledger.rs` → FAIL;
  (3) `pub fn encode_chain_dep` / `decode_chain_dep` outside
  `crates/ade_ledger/src/snapshot/chain_dep.rs` → FAIL;
  (4) `pub const SCHEMA_VERSION` outside `framing.rs` → FAIL;
  (5) framing.rs must reference `FingerprintMismatch` and
  `UnknownVersion` — positive presence of the cross-check paths.
- *N-I carried CI gates:* `ci_check_rollback_materialize_closure.sh`,
  `ci_check_snapshot_cadence_purity.sh`.
- *N-H carried CI gates:* `ci_check_admitted_block_closure.sh`,
  `ci_check_receive_reducer_closure.sh`,
  `ci_check_receive_replay_purity.sh`,
  `ci_check_receive_orchestrator_no_producer_dep.sh`,
  `ci_check_receive_paths_corpus_present.sh`.
- *N-G carried CI gates:* `ci_check_no_parallel_header_splitter.sh`,
  `ci_check_served_chain_closure.sh`,
  `ci_check_chain_sync_server_closure.sh`,
  `ci_check_block_fetch_server_closure.sh`,
  `ci_check_broadcast_to_served_purity.sh`,
  `ci_check_n2n_server_no_signing_dep.sh`,
  `ci_check_server_paths_corpus_present.sh`.
- *N-C carried CI gates:* `ci_check_private_key_custody.sh`,
  `ci_check_opcert_closed.sh`, `ci_check_forge_purity.sh`,
  `ci_check_no_producer_body_encoder.sh`,
  `ci_check_self_accept_gate.sh`, `ci_check_scheduler_closure.sh`,
  `ci_check_producer_corpus_present.sh`.
- `ci_check_constitution_coverage.sh` — carried.
- `ci_check_proposal_procedures_closed.sh` *(PP — DC-LEDGER-11)* — carried.
- `ci_check_mempool_ingress_closure.sh` /
  `ci_check_mempool_ingress_replay.sh` *(N-E)* — carried.
- `ci_check_credential_discriminant_closed.sh` *(OQ5 / COMMITTEE /
  DREP / ENACTMENT)* — carried.
- `ci_check_gov_cert_accumulation_closed.sh` *(B5)* — carried.
- `ci_check_deposit_param_authority.sh` *(B3)* — carried.
- `ci_check_conway_cert_classification_closed.sh` *(B3F)* — carried.
- `ci_check_no_chaindb_in_consensus_blue.sh` /
  `ci_check_no_float_in_consensus.sh` /
  `ci_check_no_density_in_fork_choice.sh` /
  `ci_check_consensus_closed_enums.sh` — carried.
- `ci_check_pallas_quarantine.sh`, `ci_check_no_signing_in_blue.sh`,
  `ci_check_ingress_chokepoints.sh`, `ci_check_ce_n_a_5_proof.sh` —
  carried.

---

## 3. Closed vs. Extensible Registries

Ade's authority surface is **almost entirely closed.** **PHASE4-N-J
added eleven closed surfaces** — the snapshot module tree itself
(`ade_ledger::snapshot::{chain_dep, utxo_state, cert_state,
epoch_state, gov_state, ledger, framing, error}`); the combined
encoder/decoder pair (`encode_snapshot` / `decode_snapshot` — SOLE
`pub fn` pair for combined-snapshot bytes per CN-STORE-08); the
composite encoder/decoder pairs (`encode_ledger_state` /
`decode_ledger_state`, `encode_chain_dep` / `decode_chain_dep`); the
versioned schema seam (`SCHEMA_VERSION: u32 = 1`); the closed error
sums (`SnapshotEncodeError` 1 variant, `SnapshotDecodeError` 5
variants, `StructuralReason` 9 variants); and the persistent reader
type (`PersistentSnapshotCache`, single production impl of
`SnapshotReader`) with its closed error sum
(`PersistentCacheError`). Plus **one CI gate** (CI count 54 → 55)
and **three newly-introduced + one strengthening + closure** registry
rules (`DC-CONS-21` flipped from `declared` to `enforced`,
`open_obligation` removed; registry total 206 → 209).

### Closed (frozen — version-gated changes only)

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `CardanoEra` | `ade_types::era` | 8 variants | New variant = new hard fork. |
| `Certificate` | `ade_types::shelley::cert` | 7 variants | Shelley-era frozen. |
| `StakeCredential` *(OQ5)* | `ade_types::shelley::cert` | 2 variants | DC-LEDGER-10. |
| Credential-decode chokepoints *(OQ5 + PP)* | `ade_codec::{shelley,conway}::cert::decode_stake_credential` + `ade_codec::conway::governance::decode_stake_credential` | 3 functions | Closed 2-variant mapping. |
| `ConwayCert` *(B3/B4)* | `ade_types::conway::cert` | 19 variants | DC-LEDGER-08. |
| `GovAction` *(PP/ENACTMENT)* | `ade_types::conway::governance` | 7 variants | DC-LEDGER-11. |
| `ProposalProcedure` *(PP)* | `ade_types::conway::governance` | closed 4-field struct | DC-LEDGER-11. |
| `decode_proposal_procedures` / `encode_proposal_procedures` *(PP)* | `ade_codec::conway::governance` | 2 functions | DC-LEDGER-11. |
| `MIRPot` | `ade_types::shelley::cert` | 2 variants | Frozen. |
| `DRep` | `ade_types::conway::cert` | 4 variants | CIP-1694 fixed. |
| `CertDisposition` / `DepositEffect` / `CoinSource` *(B3)* | `ade_types::conway::cert` | 3 / 2 / 3 variants | Closed. |
| `ConwayCertAction` *(B4)* | `ade_ledger::delegation` | closed | No `Neutral`. |
| `GovernanceCertEffect` / `OwnerTaggedEffect` / etc. *(B4)* | `ade_ledger::delegation` | closed | B4 plumbing. |
| `GovCertEnv` *(B5)* | `ade_ledger::state` | closed struct | Fail-fast. |
| `apply_conway_gov_cert` dispatch *(B5)* | `ade_ledger::gov_cert` | 1 function | DC-LEDGER-09. |
| `apply_committee_enactment` *(ENACTMENT)* | `ade_ledger::governance` | 1 pure transition | Closed. |
| `IngressSource` *(N-E)* | `ade_ledger::mempool::ingress` | 2 variants | Closed source discriminant. |
| `IngressEvent` *(N-E)* | `ade_ledger::mempool::ingress` | closed struct | Closed flat-data envelope. |
| `mempool_ingress` chokepoint *(N-E)* | `ade_ledger::mempool::ingress` | 1 function | DC-MEM-03. |
| `ProducerTick` *(N-C-S3)* | `ade_ledger::producer::state` | closed 14-field struct | Carried. |
| `forge_block` chokepoint *(N-C-S3)* | `ade_ledger::producer::forge` | 1 function | Carried. |
| `ForgeError` / `ForgeEffects` / `ForgedBlock` *(N-C-S3)* | `ade_ledger::producer::forge` | 7 / 1 / closed struct | Carried. |
| `encode_opcert` / `decode_opcert` chokepoint pair *(N-C-S2)* | `ade_codec::shelley::opcert` | 2 functions | Carried. |
| `OpCertCodecError` *(N-C-S2)* | `ade_codec::shelley::opcert` | 7 variants | Carried. |
| `opcert_validate` chokepoint *(N-C-S2)* | `ade_core::consensus::opcert_validate` | 1 function | Carried. |
| `OpCertError` *(N-C-S2)* | `ade_core::consensus::opcert_validate` | closed | Carried. |
| `block_body_hash_from_buckets` chokepoint *(N-C-S4)* | `ade_ledger::block_body_hash` | 1 function | Carried. |
| `AcceptedBlock` token *(N-C-S5)* | `ade_ledger::producer::self_accept` | 1 newtype | Carried. |
| `self_accept` chokepoint *(N-C-S5)* | `ade_ledger::producer::self_accept` | 1 function | Carried. |
| `SelfAcceptError` *(N-C-S5)* | `ade_ledger::producer::self_accept` | 1 variant | Carried. |
| `SchedulerInput` / `SchedulerEffect` / `SchedulerHaltReason` / `SchedulerState` *(N-C-S6)* | `ade_runtime::producer::scheduler` | closed sums | Carried. |
| `TickInputs` / `TickAssemblyError` / `assemble_tick` *(N-C-S6)* | `ade_runtime::producer::tick_assembler` | closed | Carried. |
| `BroadcastError` *(N-C-S6)* | `ade_runtime::producer::broadcast` | 2 variants | Carried. |
| RED signing primitives + key types *(N-C-S1)* | `ade_runtime::producer::signing::*` | closed | Carried. |
| RED key loader *(N-C-S1)* | `ade_runtime::producer::keys` | closed | Carried. |
| `accepted_block_header_bytes` accessor *(N-G-S1)* | `ade_ledger::block_validity::header_input` | 1 function | Carried. |
| `ServerReply` (chain-sync + block-fetch) *(N-G-S1)* | `ade_network::{chain_sync, block_fetch}::server` | 2 closed wrappers | Carried. |
| `HeaderProjection` *(N-G-S3)* | `ade_network::chain_sync::server` | closed struct | Carried. |
| `ServedHeaderLookup` / `ServedRangeLookup` traits *(N-G-S3/S4)* | `ade_network::{chain_sync, block_fetch}::server` | 2 closed traits | Carried. |
| `producer_chain_sync_serve` / `producer_chain_sync_advance_tip` *(N-G-S3)* | `ade_network::chain_sync::server` | 2 functions | Carried. |
| `producer_block_fetch_serve` *(N-G-S4)* | `ade_network::block_fetch::server` | 1 function | Carried. |
| `Producer*ServerState` / `ProducerServerError` / `ServerStep` / etc. *(N-G-S3/S4)* | `ade_network::{chain_sync, block_fetch}::server` | closed | Carried. |
| `ServedChainSnapshot` / `served_chain_admit` *(N-G-S2)* | `ade_ledger::producer::served_chain` | closed | Carried. |
| `PerPeerN2nServerState` / `DispatchError` *(N-G-S6)* | `ade_runtime::network::n2n_server` | closed | Carried. |
| `AdmittedBlock` token *(N-H-S1)* | `ade_ledger::receive::admitted` | closed struct | Carried. |
| `AdmittedOutcome` *(N-H-S1)* | `ade_ledger::receive::admitted` | closed struct | Carried. |
| `admit_via_block_validity` chokepoint *(N-H-S1)* | `ade_ledger::receive::admitted` | 1 function | Carried. |
| `ReceiveEvent` *(N-H-S1)* | `ade_ledger::receive::events` | 3 variants | Carried. |
| `ReceiveEffect` *(N-H-S1)* | `ade_ledger::receive::events` | 4 variants | Carried. |
| `NoOpReason` *(N-H-S1)* | `ade_ledger::receive::events` | 1 variant | Carried. |
| `ReceiveError` *(N-H-S1)* | `ade_ledger::receive::events` | 4 variants | Carried. |
| `TargetPoint` / `TipPoint` *(N-H-S1 — receive)* | `ade_ledger::receive::events` | 2 closed structs | Carried. |
| `PendingHeaderCache` *(N-H-S1)* | `ade_ledger::receive::pending_header_cache` | closed struct | Carried. |
| `PendingHeaderCacheError` *(N-H-S1)* | `ade_ledger::receive::pending_header_cache` | 1 variant | Carried. |
| `ChainDbWrite` trait *(N-H-S1; N-I-S3 extended)* | `ade_ledger::receive::chain_write` | 2 methods | Carried. |
| `ChainWriteError` / `ChainWriteErrorKind` *(N-H-S1)* | `ade_ledger::receive::chain_write` | 2 / 3 variants | Carried. |
| `ReceiveState` *(N-H-S2)* | `ade_ledger::receive::reducer` | closed struct | Carried. |
| `receive_apply` chokepoint *(N-H-S2; N-I-S6 extended)* | `ade_ledger::receive::reducer` | 1 function | Carried. |
| `receive_apply_sequence` driver *(N-H-S2)* | `ade_ledger::receive::reducer` | 1 function | Carried. |
| `PerPeerReceiveState` *(N-H-S4)* | `ade_runtime::receive::orchestrator` | closed RED struct | Carried. |
| `ReceiveDispatchError` *(N-H-S4)* | `ade_runtime::receive::orchestrator` | 3 variants | Carried. |
| `SnapshotReader` trait *(N-I-S1; **N-J extended**)* | `ade_ledger::rollback::traits` | 1 trait with 1 method | Closed seam. **N-J note:** now has **two production impls** (`InMemorySnapshotCache` from N-I, `PersistentSnapshotCache` from N-J). Trait surface unchanged. |
| `BlockSource` trait *(N-I-S1)* | `ade_ledger::rollback::traits` | 1 trait with 1 method | Carried. |
| `MaterializeError` *(N-I-S1)* | `ade_ledger::rollback::error` | 3 variants | Carried. |
| `CommitRollbackError` *(N-I-S1)* | `ade_ledger::rollback::error` | 1 variant | Carried. |
| `TargetPoint` *(N-I-S2 — rollback flavor)* | `ade_ledger::rollback::materialize` | closed struct | Carried. |
| `materialize_rolled_back_state` chokepoint *(N-I-S2 — CN-STORE-07)* | `ade_ledger::rollback::materialize` | 1 function | Carried. **N-J note:** unchanged; the persistent reader fits the existing `&dyn SnapshotReader` extension point. |
| `commit_rollback` chokepoint *(N-I-S3)* | `ade_ledger::rollback::commit` | 1 function | Carried. |
| `ChainDbWrite::rollback_to_slot` trait method *(N-I-S3)* | `ade_ledger::receive::chain_write` | 1 method | Carried. |
| `RollbackContext<'a>` *(N-I-S6)* | `ade_ledger::receive::reducer` | closed BLUE struct | Carried. |
| `SnapshotCadence` *(N-I-S4 — DC-STORE-07)* | `ade_runtime::rollback::cadence` | closed BLUE-structural struct (exactly 1 field) | Carried. |
| **`SnapshotEncodeError`** *(NEW in N-J-S1)* | `ade_ledger::snapshot::error` | 1 variant — `EraNotSupported { era: CardanoEra }` | Closed sum. No `#[non_exhaustive]`; no `String`. New variant = closed-sum extension (e.g. future `IndexCorrupt` for a sibling encoder). |
| **`SnapshotDecodeError`** *(NEW in N-J-S1)* | `ade_ledger::snapshot::error` | 5 variants — `Cbor(CodecError)`, `UnknownVersion { expected, found }`, `FingerprintMismatch { expected, actual }`, `EraNotSupported { era }`, `Structural { reason }` | Closed sum. No `#[non_exhaustive]`; no `String`. Round-trip-through-pattern-match test confirms a sixth variant fails to compile. |
| **`StructuralReason`** *(NEW in N-J-S1)* | `ade_ledger::snapshot::error` | 9 variants (`ArrayLengthMismatch`, `MapLengthExceeded`, `UnexpectedNull`, `UnexpectedNonNull`, `NonceLengthMismatch`, `PoolIdLengthMismatch`, `Hash32LengthMismatch`, `Hash28LengthMismatch`, `EraTagOutOfRange`) | Closed sum. `Copy`. Static tag rather than `String` so the sum stays closed. |
| **`encode_chain_dep` / `decode_chain_dep` chokepoint pair** *(NEW in N-J-S1 — CN-STORE-08)* | `ade_ledger::snapshot::chain_dep` | 2 functions — **THE SOLE `pub fn` pair encoding/decoding `PraosChainDepState` bytes in the workspace** | Single-authority pair. Composed by the framing layer. New chokepoint at this signature = strengthening (CI fail). |
| **`encode_utxo_state` / `decode_utxo_state` chokepoint pair** *(NEW in N-J-S2)* | `ade_ledger::snapshot::utxo_state` | 2 functions — sole pair for `UTxOState` bytes in-module | Sub-state encoder. BTreeMap-only traversal. |
| **`encode_cert_state` / `decode_cert_state` chokepoint pair** *(NEW in N-J-S3)* | `ade_ledger::snapshot::cert_state` | 2 functions — sole pair for `CertState` bytes in-module | Sub-state encoder. |
| **`encode_epoch_state` / `decode_epoch_state` chokepoint pair** *(NEW in N-J-S4)* | `ade_ledger::snapshot::epoch_state` | 2 functions — sole pair for `EpochState` (+ `SnapshotState`) bytes in-module | Sub-state encoder. |
| **`encode_pparams` / `decode_pparams` / `encode_gov_state` / `decode_gov_state` / `encode_conway_deposit_params` / `decode_conway_deposit_params`** *(NEW in N-J-S5)* | `ade_ledger::snapshot::gov_state` | 6 functions | Sub-state encoder triple. |
| **`encode_ledger_state` / `decode_ledger_state` chokepoint pair** *(NEW in N-J-S6 — CN-STORE-08)* | `ade_ledger::snapshot::ledger` | 2 functions — **THE SOLE `pub fn` pair encoding/decoding `LedgerState` bytes in the workspace** | Composite encoder. Assembles S2–S5 sub-state pairs in canonical field order matching `ade_ledger::fingerprint`. Single-authority via CI grep. |
| **`encode_snapshot` / `decode_snapshot` chokepoint pair** *(NEW in N-J-S7 — CN-STORE-08)* | `ade_ledger::snapshot::framing` | 2 functions — **THE SOLE `pub fn` pair encoding/decoding `(LedgerState, PraosChainDepState)` combined snapshot bytes in the workspace** | Combined-snapshot encoder. Wire layout: `array(4)[u32 version, bytes(32) fingerprint, bytes ledger, bytes chain_dep]`. Version verified BEFORE payload (DC-STORE-09). Fingerprint recomputed + verified AFTER decode (DC-STORE-08). Conway-only at encoder; pre-Conway → `EraNotSupported`. Single-authority via CI grep. |
| **`SCHEMA_VERSION: u32 = 1`** *(NEW in N-J-S7 — DC-STORE-09)* | `ade_ledger::snapshot::framing` | 1 `pub const` — **THE SOLE schema-version anchor in `crates/`** | Closed versioned-schema seam. Future v2 layout = bump to 2; decoder dispatches on tag; v1 readers fail-closed on v2 bytes. CI-defended: no other `pub const SCHEMA_VERSION` allowed across `crates/`. |
| **`PersistentSnapshotCache<'a, S: SnapshotStore + ?Sized>`** *(NEW in N-J-S8)* | `ade_runtime::rollback::persistent_cache` | closed GREEN struct `{ store: &'a S }` — **THE single production persistent impl of `SnapshotReader`** | Closed extension point. New persistent backends attach via the `SnapshotStore` trait, not by re-implementing `SnapshotReader`. Field set closed (1 field). |
| **`PersistentCacheError`** *(NEW in N-J-S8)* | `ade_runtime::rollback::persistent_cache` | 3 variants — `Encode(SnapshotEncodeError)`, `Decode(SnapshotDecodeError)`, `Store(ChainDbError)` | Closed sum. Carries upstream errors verbatim. No `String`. |
| **`PERSISTENT_CACHE_SCHEMA_VERSION: u32`** *(NEW in N-J-S8)* | `ade_runtime::rollback::persistent_cache` | 1 `pub const` — re-export of `framing::SCHEMA_VERSION` | Closed re-export. Test pins equality with framing's anchor. |
| `PlutusLanguage` | `ade_plutus::evaluator` | 3 variants | |
| Named ingress chokepoints (block CBOR) | `ade_codec::*` | 10 | |
| Conway cert/withdrawals sub-grammar decoders *(B3 / B4)* | `ade_codec::conway::{cert, withdrawals}` + `ade_codec::shelley::cert::read_pool_registration_cert` | 5 functions | Closed. |
| Named ingress chokepoint (Plutus script CBOR) | `ade_plutus::evaluator::PlutusScript::from_cbor` | 1 | |
| `PreservedCbor::new` constructor | `ade_codec::preserved` | 1 chokepoint, `pub(crate)` | |
| `CodecError` variants *(B3-extended)* | `ade_codec::error` | + `UnknownCertTag`, `DuplicateMapKey` | |
| Mini-protocol message enums | `ade_network::codec::*` | 11 closed enums | |
| Mini-protocol encode/decode chokepoints | `ade_network::codec::*::{encode_*, decode_*}` | 22 functions | |
| Mux frame chokepoints | `ade_network::mux::frame::{encode_frame, decode_frame}` | 2 free functions | |
| Mini-protocol transition functions | `ade_network::*::transition` + `n2c::local_*::transition` | 8 modules | |
| Mini-protocol version enums | `ade_network::codec::version::*` | 11 closed enums | |
| `ChainDb` / `SnapshotStore` / `Recoverable` trait surfaces | `ade_runtime::chaindb` + `ade_runtime::recovery` | closed | **N-J note:** `SnapshotStore` (3 methods — `put_snapshot`, `get_snapshot`, `list_snapshot_slots`) is now consumed at production scale by `PersistentSnapshotCache` (single production consumer); trait surface unchanged. |
| Hash domain functions | `ade_crypto::blake2b::*` | 4 named domains | |
| `ChainEvent` / `ChainSelectionReject` *(N-B)* | `ade_core::consensus::events` | 5 / 4 variants | |
| Consensus error families *(N-B)* | `ade_core::consensus::errors` | 8 closed error enums | |
| `StreamInput` / `OrchestratorError` / `DecodeError` / `GenesisParseError` / `GenesisBlob` / `NetworkMagic` *(N-B)* | various | closed | |
| `LedgerView` trait *(N-B; B1-refined)* | `ade_core::consensus::ledger_view` | 4 methods | |
| `HeaderVrf` *(N-B; B1)* | `ade_core::consensus::header_summary` | 2 variants | |
| `BlockValidityVerdict` / `BlockValidityError` etc. *(B1)* | `ade_ledger::block_validity::verdict` | closed | |
| `block_validity` chokepoint *(B1)* | `ade_ledger::block_validity::transition` | 1 function | Single chokepoint. Five public consumers: B1 validator, `self_accept` (N-C), `served_chain_admit` (N-G), `admit_via_block_validity` (N-H), `materialize_rolled_back_state` (N-I). N-J adds no new consumer. |
| `TxValidityVerdict` / `TxRejectClass` / `TxValidityError` / `SignerSource` / `WitnessClosureError` etc. *(B2)* | `ade_ledger::tx_validity::*` | closed | |
| `AdmitOutcome` / `MempoolState` / `OrderPolicy` *(B2)* | `ade_ledger::mempool::*` | closed | |
| `LeaderScheduleAnswer` / `is_leader_for_vrf_output` *(N-B)* | `ade_core::consensus::leader_schedule` | closed | |
| `PraosNonces` / `NonceScanError` *(B1)* | `ade_ledger::consensus_input_extract` | | |
| `PraosChainDepState` / `ChainEvent` canonical encodings *(N-B)* | `ade_core::consensus::encoding` | 4 chokepoints | **N-J note:** N-B's `encode_chain_dep_state` (`ade_core::consensus::encoding::encode_chain_dep_state`) is the consensus-internal canonical encoding; N-J's `encode_chain_dep` (`ade_ledger::snapshot::chain_dep::encode_chain_dep`) is the snapshot-layer encoding. **These are deliberately separate**: the snapshot layer adds restart-safety metadata (version, fingerprint) and uses snapshot-specific wire shape. CN-STORE-08's grep is scoped to the **N-J function names** (`encode_chain_dep` / `decode_chain_dep` / `encode_ledger_state` / `decode_ledger_state` / `encode_snapshot` / `decode_snapshot`); the N-B `encode_chain_dep_state` is not in scope. |
| `LedgerFingerprint` fold *(B3/B5)* | `ade_ledger::fingerprint` | | **N-J note:** consumed by `encode_snapshot` and `decode_snapshot` as the cross-check authority (embedded on encode; recomputed + verified on decode). `DC-STORE-08` cites `fingerprint(ledger).combined` as the canonical pre-image. |
| **CI check set** | `ci/ci_check_*.sh` | **55 scripts (54 → 55 in PHASE4-N-J)** | Existing checks may be tightened, never relaxed. |
| **Invariant registry families** | `docs/ade-invariant-registry.toml` | Families T / CN / DC / OP / RO; **N-J added 3 rules** (`DC-STORE-08` `enforced`, `DC-STORE-09` `enforced`, `CN-STORE-08` `enforced`); strengthened + closed `DC-CONS-21` (`open_obligation` removed). Total: **209 entries** (206 → 209). | Append-only IDs. |

### Extensible (open within constraints)

| Registry | Location | Extension Rule |
|----------|----------|---------------|
| `CostModels` map (Plutus V1/V2/V3 cost tables) | `ade_plutus::cost_model::CostModels` | Decoder-driven; constrained by closed `PlutusLanguage`. |
| `ProtocolParameters` / `ProtocolParameterUpdate` field set | `ade_ledger::pparams` | Era-versioned. |
| Pool / DRep / Stake registrations | `ade_ledger::state::{DelegationState, CertState}` | Shape closed; set open. |
| Governance proposal / committee / DRep registration set | `ade_ledger::state::ConwayGovState` | Shape closed; instance set open. |
| Tx-body `proposal_procedures` instance set *(PP)* | `ade_types::conway::tx::ConwayTxBody.proposal_procedures` | `Option<Vec<ProposalProcedure>>`. Shape closed; instance set open. |
| `OpCertCounterMap` *(N-B)* | `ade_core::consensus::praos_state` | BTreeMap; inserts strictly increasing per `(pool, kes_period)`. |
| `PoolDistrView` pool table *(B1)* | `ade_ledger::consensus_view::PoolDistrView::pools` | `BTreeMap<Hash28, PoolEntry>`. |
| Withdrawals map *(B3)* | `ade_codec::conway::withdrawals::decode_withdrawals` → `BTreeMap<RewardAccount, Coin>` | Never last-wins. |
| Mempool admitted set *(B2)* | `ade_ledger::mempool::admit::MempoolState::accepted` | `Vec<Hash32>`; shape closed; monotonic. |
| `SignerSource` provenance set *(B2)* | `ade_ledger::tx_validity::required_signers::RequiredSigners::{keys, provenance}` | Per-tx open. |
| `RollbackSnapshot` ring *(N-B)* | `ade_runtime::consensus::chain_selector::OrchestratorState::recent_snapshots` | Bounded ≤ 2160. Distinct from N-I `InMemorySnapshotCache` + N-J `PersistentSnapshotCache`. |
| `ServedChainSnapshot.blocks` admitted set *(N-G-S2)* | `ade_ledger::producer::served_chain::ServedChainSnapshot` | Shape closed; instance set open. |
| `PerPeerN2nServerState` instance set *(N-G-S6)* | `ade_runtime::network::n2n_server` | One instance per connected peer. |
| `PendingHeaderCache.entries` *(N-H-S1)* | `ade_ledger::receive::pending_header_cache::PendingHeaderCache` | `BTreeMap<(SlotNo, Hash32), Vec<u8>>`. |
| `PerPeerReceiveState` instance set *(N-H-S4)* | `ade_runtime::receive::orchestrator` | One instance per upstream peer. |
| `InMemorySnapshotCache.entries` *(N-I-S4)* | `ade_runtime::rollback::in_memory_cache::InMemorySnapshotCache` | `BTreeMap<SlotNo, (LedgerState, PraosChainDepState)>`. Shape closed; instance set open. No eviction (carried OQ-5). |
| **Persistent snapshot store contents** *(NEW in N-J-S8 — runtime-extensible **content** via the closed `PersistentSnapshotCache::capture` chokepoint + the underlying `SnapshotStore::put_snapshot` trait method)* | the `SnapshotStore` instance backing `PersistentSnapshotCache` | `BTreeSet<SlotNo>` per `list_snapshot_slots`. Shape closed; instance set open. The set grows via `PersistentSnapshotCache::capture` (single production write entry); shrinks via `SnapshotStore::delete_snapshot` (no production caller wired at this HEAD — eviction is the named follow-on candidate). |
| Oracle reference snapshots / regression corpus | `ade_testkit::harness::*` | Tooling-only. |
| Network corpus / Consensus corpus / Block-validity corpus / Tx-validity corpus / Mempool ingress corpus / PP canonical corpus / Producer corpus / Server-paths corpus / Receive-paths corpus | various | Tooling-only. |
| Receive-rollback integration test *(N-I-S6)* | `crates/ade_runtime/tests/receive_rollback_integration.rs` | Tooling-only. |
| **Persistent-cache inline test set** *(NEW in N-J-S8 — tooling-only)* | `crates/ade_runtime/src/rollback/persistent_cache.rs` (inline `#[cfg(test)] mod tests`) | Tooling-only. Inline tests prove: round-trip via `nearest_le`; cross-impl equivalence with `InMemorySnapshotCache` over a multi-probe sweep; empty-store returns `None`; pre-Conway capture surfaces `Encode/EraNotSupported`; corrupt persisted bytes yield `None` (no panic); `PERSISTENT_CACHE_SCHEMA_VERSION` equals `framing::SCHEMA_VERSION`. |
| Operator-action probe binaries *(N-B + N-E S6 + N-C S7 + N-G S7 + N-H S6)* | `ade_core_interop::bin::*` | RED operator-action; `#[ignore]`-gated. **N-J added no new binary.** |
| `KillStrategy<D>` trait impls | `ade_runtime::chaindb::crash_safety` | RED-only test infrastructure. |
| Recovery state types | callers of `Recoverable` | Open: any state with canonical encode + apply-block step. |
| Pinned external crates | `crates/*/Cargo.toml` | Tier-5 rationale doc required. |

### Candidates — extensible surfaces not yet wired

| Cluster | Candidate registry | Rationale |
|---------|-------------------|-----------|
| **Orchestrator-side persistent-capture wiring cluster** *(NEW candidate flagged by N-J close)* | **A post-admission hook that writes to `PersistentSnapshotCache` alongside `InMemorySnapshotCache`** | The persistent cache's `capture` method is wired only at the test layer today; production capture still goes via `maybe_capture_snapshot` to in-memory only. Tier-5 operational concern; no BLUE invariants change. |
| **Snapshot eviction policy cluster** *(carried from N-I; **now doubled by N-J — applies to both caches**)* | **Bounded ring + persistent retention policy** | Tier-5 operational concern. Both `InMemorySnapshotCache` and the persistent `SnapshotStore` grow monotonically at this HEAD. Must remain replay-deterministic. |
| **Pre-Conway snapshot encoder cluster** *(NEW low-priority candidate flagged by N-J close)* | **Widen `encode_ledger_state` + `decode_ledger_state` to Babbage and earlier eras** | No current operational need; rollback target windows are bounded. |
| **Snapshot schema migration v1 → v2 cluster** *(NEW candidate flagged by N-J close)* | **Future evolution of the framing wire format** | `SCHEMA_VERSION: u32 = 1` is the explicit anchor; first new field appended bumps to 2. No present cluster planned. |
| **Multi-peer fork choice cluster** *(carried; now doubly-enabled by N-J)* | **Praos longest-chain across competing peers** | Re-uses N-I `RollbackContext` to roll back losing forks. Now restart-safe via N-J. |
| **N2C local-chain-sync receive surface cluster** *(carried)* | Carried. | |
| **CE-N-H-6 / CE-N-G-8 / CE-N-C-8 operator-action live evidence** *(carried)* | Carried. | |
| **N-I+ Tier-5 — operator-tunable rollback policy** *(carried)* | Carried. | |
| **N-G+ Tier-5 — operator-tunable server policy** *(carried)* | Carried. | |
| **N-C+ Tier-5 — operator-tunable producer policy** *(carried)* | Carried. | |
| **CE-NODE-N2C-LTX** *(carried from N-E)* | Carried. | |
| **PP OQ-1..OQ-4** *(carried)* | Carried. | |
| N-A (deferred) | Peer address book | Runtime mutable. |
| N-F | Query API method set | Tier 5 wire / Tier 1 semantics. |
| N-F | Prometheus metric names | Tier 5; append-only registry expected. |

### Closed-grammar audit (PHASE4-N-J full close)

This sweep was performed after PHASE4-N-J full close (S1..S8).

1. **`SnapshotEncodeError` / `SnapshotDecodeError` / `StructuralReason`
   closed sums** — **closed by intent.** 1 / 5 / 9 variants; no
   `#[non_exhaustive]`; no `String`. Round-trip-through-pattern-match
   tests confirm sixth/sixth/tenth variant additions fail to compile.
2. **`encode_chain_dep` / `decode_chain_dep` chokepoint pair** —
   **closed by intent and CI-defended.** Sole pair for
   `PraosChainDepState` bytes in the workspace (CN-STORE-08 grep).
3. **`encode_utxo_state` / `encode_cert_state` / `encode_epoch_state`
   / `encode_pparams` / `encode_gov_state` /
   `encode_conway_deposit_params` (+ decoders)** — **closed by intent.**
   Each is the sole pair for its sub-state. Sub-state CI scope is
   in-module (the framing closure check enforces the composite
   layer's authority).
4. **`encode_ledger_state` / `decode_ledger_state` chokepoint pair**
   — **closed by intent and CI-defended.** Sole pair for `LedgerState`
   bytes in the workspace. Composes S2–S5 sub-state pairs in field
   order matching `ade_ledger::fingerprint`.
5. **`encode_snapshot` / `decode_snapshot` chokepoint pair** —
   **closed by intent and CI-defended.** Sole pair for combined
   `(LedgerState, PraosChainDepState)` snapshot bytes in the
   workspace. Wire layout closed; decoder verifies version BEFORE
   payload + fingerprint AFTER decode.
6. **`SCHEMA_VERSION: u32 = 1` schema anchor** — **closed by intent
   and CI-defended.** Sole `pub const SCHEMA_VERSION` in `crates/`.
   Future v2 bump = closed extension; decoder fails-closed on
   unknown versions.
7. **`PersistentSnapshotCache<'a, S: SnapshotStore + ?Sized>` GREEN
   reader** — **closed by intent.** Single production persistent
   impl of `SnapshotReader`. Borrows the store (lifetime `'a`); holds
   no in-memory state. New persistent backends attach via
   `SnapshotStore`, not via re-implementing `SnapshotReader`.
8. **`PersistentCacheError` closed sum** — **closed by intent.** 3
   variants (`Encode | Decode | Store`); carries upstream errors
   verbatim.
9. **`PERSISTENT_CACHE_SCHEMA_VERSION` re-export** — **closed by
   intent.** Test pins equality with `framing::SCHEMA_VERSION`.

**Gap note — orchestrator-side persistent-capture wiring.** The
persistent capture path is exposed at the BLUE + GREEN layers but is
not yet wired into the per-peer post-admission hook
(`maybe_capture_snapshot` still talks only to `InMemorySnapshotCache`).
Surfaced as the highest-priority post-N-J Tier-5 candidate seam.

**Gap note — snapshot eviction (carried, now doubled).** Both the
in-memory cache and the persistent `SnapshotStore` grow monotonically
at this HEAD. The persistent store already has a
`SnapshotStore::delete_snapshot(slot)` method (N-D); no policy
decides when to call it. Carried as a Tier-5 follow-on.

### Closed-grammar audit (carried — PHASE4-N-I / N-H / N-G / N-C / PROPOSAL-PROCEDURES-DECODE / N-E / B3 / B4 / B5)

All carried unchanged from prior revision.

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

- **Cardano-canonical CBOR wire format**.
- **Block envelope shape**: `[era_tag:u8, era_block:CBOR]`; era tags 0..=7.
- **`PreservedCbor<T>` invariant**.
- **Hash algorithms**: Blake2b-224 / 256, Ed25519, Byron-bootstrap,
  KES-sum, VRF-draft-03.
- **Era-correct block body hash** *(B1; strengthened in N-C, N-G, N-H)*:
  preserved-CBOR-segment bytes.
- **Single canonical body-hash authority** *(N-C-S4 — DC-CONS-16)*: carried.
- **Single canonical header/body splitter** *(N-G-S1 — DC-CONS-18)*: carried.
- **Server-agency closure for outgoing mini-protocol messages** *(N-G-S1 — CN-PROTO-06)*: carried.
- **Receive-event closure for incoming peer signals** *(N-H-S1 — CN-PROTO-07)*: carried.
- **Type-level receive admission gate** *(N-H-S1 — CN-CONS-07 strengthening)*: carried.
- **Receive-side admission state-isolation discipline (Invariant I-6)** *(N-H-S2 — CN-CONS-08 / DC-CONS-19)*: carried.
- **Single canonical receive-side rollback materialization authority** *(N-I-S2 — CN-STORE-07)*: carried.
- **Replay-forward correctness** *(N-I-S2 — DC-CONS-22)*: carried. `DC-CONS-22.strengthened_in += PHASE4-N-J` (replay-forward over persistent snapshots equals direct-apply — proven via `persistent_cache_matches_in_memory_cache_semantics`).
- **Atomic rollback commit discipline** *(N-I-S3)*: carried. `DC-CONS-20.strengthened_in += PHASE4-N-J` (rollback atomicity now restart-safe).
- **Receive-side atomic admit + rollback over ChainDb + LedgerState + PraosChainDepState** *(N-H-S2 + N-I-S6 — DC-CONS-20)*: carried; now restart-safe.
- **Receive-reducer rollback-context discipline** *(N-I-S6)*: carried.
- **Snapshot cadence determinism** *(N-I-S4 — DC-STORE-07)*: carried.
- **`ChainDbWrite::rollback_to_slot` trait method semantics** *(N-I-S3)*: carried.
- **Single canonical snapshot encoder authority** *(NEW in N-J-S7 — CN-STORE-08)*: `encode_snapshot` / `decode_snapshot` is the SOLE `pub fn` pair encoding/decoding the combined `(LedgerState, PraosChainDepState)` tuple bytes in the workspace; `encode_ledger_state` / `decode_ledger_state` is the SOLE pair for `LedgerState` bytes; `encode_chain_dep` / `decode_chain_dep` is the SOLE pair for `PraosChainDepState` bytes. CI enforcement via repo-wide grep in `ci_check_snapshot_encoder_closure.sh`.
- **Snapshot encoder canonicality** *(NEW in N-J — DC-STORE-08)*: `encode_snapshot(s)` is byte-identical across runs. Encoder uses BTreeMap iteration only; no HashMap; no wall-clock; no floats; no rand. Definite-length CBOR containers throughout. Fingerprint embedded on encode + recomputed + verified on decode. Snapshot bytes are byte-identical across runs for the same source state; corruption / schema drift fails closed.
- **Snapshot bytes version-tag + fingerprint discipline** *(NEW in N-J — DC-STORE-09)*: snapshot bytes carry a closed `u32` version tag (initial `== SCHEMA_VERSION == 1`) and the source state's blake2b-256 fingerprint. Decoder reads the version tag FIRST and rejects unknown versions BEFORE decoding the ledger or chain-dep payload; decoder recomputes the fingerprint on the decoded state and rejects on mismatch.
- **Snapshot encoder Conway-only scope** *(NEW in N-J)*: pre-Conway → `SnapshotEncodeError::EraNotSupported` / `SnapshotDecodeError::EraNotSupported` structurally. Mirrors N-I's `MaterializeError::EraNotSupported` discipline.
- **Persistent snapshot reader contract** *(NEW in N-J-S8)*: `PersistentSnapshotCache::nearest_le` walks the `SnapshotStore`'s ascending slot list, picks the largest ≤ target, decodes via `framing::decode_snapshot`, and surfaces `None` on decode failure (treats corruption as "no usable snapshot" rather than panic). The reader is observationally equivalent to `InMemorySnapshotCache` for the same admit sequence (proven by `persistent_cache_matches_in_memory_cache_semantics`).
- **Receive-side replay determinism** *(N-H-S3 — DC-PROTO-09)*: carried.
- **Per-peer receive-state independence across peers** *(N-H-S4)*: carried.
- **Key-boundary for receive paths** *(N-H-S4)*: carried.
- **Handshake-negotiated version threading through the receive reducer call site** *(N-H-S4 — DC-PROTO-06 strengthening)*: carried.
- **Served-bytes parity** *(N-G-S4 — DC-CONS-17)*: carried.
- **Header-body wire coherence** *(N-G-S5 — DC-CONS-18)*: carried.
- **Producer-side server-role transcript determinism** *(N-G-S5 — DC-PROTO-07)*: carried.
- **Deterministic-resolution discipline for server-agency waits** *(N-G-S3 — DC-PROTO-08)*: carried.
- **Type-level broadcast and serve gate** *(N-C-S5 — CN-CONS-07)*: carried.
- **Tx id over preserved body bytes** *(B2)*.
- **Conway certificate CDDL grammar** *(B3/B3F/B4)*.
- **Conway `DRep` decode grammar** *(B4)*.
- **Owner-tagged Conway cert-state apply contract** *(B4)*: DC-LEDGER-08.
- **Closed total gov-cert dispatch contract** *(B5)*: DC-LEDGER-09.
- **Fail-fast gov-cert environment** *(B5)*.
- **Checked DRep-expiry arithmetic** *(B5)*.
- **`ConwayGovState` deterministic-fold accumulation** *(B5)*. **N-J note:** `ConwayGovState` is now also one of the encoded sub-states (S5); the encoder reads its fields via `ade_ledger::fingerprint`'s field walk — same field set, same iteration order.
- **Conway withdrawals map grammar** *(B3)*: never last-wins.
- **Closed deposit-effect sum types** *(B3)*.
- **Canonical deposit-param authority** *(B3)*: DC-TXV-07.
- **Full Conway value-conservation equation** *(B3)*.
- **`LedgerFingerprint` Conway deposit-param fold** *(B3)*. **N-J note:** `LedgerFingerprint.combined` is the canonical pre-image of the snapshot fingerprint cross-check (DC-STORE-08).
- **Closed `proposal_procedures` wire grammar at Conway tx-body key 20** *(PP — DC-LEDGER-11)*.
- **Plutus script ingress chokepoint**: `PlutusScript::from_cbor`.
- **Plutus language set**: V1, V2, V3.
- **Aiken UPLC quarantine pin**: `aiken_uplc` at tag `v1.1.21`.
- **Ouroboros mux frame layout**: 8-byte big-endian header.
- **11 closed mini-protocol message enums** + **8 closed state graphs**.
- **`BootstrapAnchorHash` v1 preimage** *(N-B)*.
- **`EraSchedule` invariants** *(N-B)*.
- **`PraosChainDepState` / `ChainEvent` CBOR encodings** *(N-B)*. **N-J note:** the N-B `ade_core::consensus::encoding::encode_chain_dep_state` is the consensus-internal canonical encoding; the N-J `ade_ledger::snapshot::chain_dep::encode_chain_dep` is the snapshot-layer encoding with restart-safety metadata. Both frozen at their respective wire shapes; CN-STORE-08's single-authority grep targets the N-J names.
- **Consensus error taxonomies** *(N-B)*.
- **`StreamInput` 3-variant taxonomy** *(N-B)*. **`HeaderVrf` era model**.
- **`block_validity` composition contract** *(B1; N-I strengthened)*: carried.
- **`VerdictSurface` CBOR encoding** *(B1)*.
- **`LedgerView` trait shape** *(N-B; B1-refined)*.
- **`tx_validity` composition contract** *(B2)*.
- **`SignerSource` enumeration** *(B2)*.
- **Witness-closure contract** *(B2)*.
- **`TxVerdictSurface` CBOR encoding** *(B2)*.
- **Mempool admission contract** *(B2)*.
- **`mempool_ingress` chokepoint contract** *(N-E)*.
- **`IngressSource` source-invariance contract** *(N-E)*.
- **Verbatim tx-bytes flow through ingress** *(N-E; N-H mirror)*: carried.
- **GREEN single-step replay fold contract** *(N-E — DC-MEM-04)*.
- **Cross-cluster obligation pattern** *(N-E; carried)*.
- **Operator-action evidence pattern** *(N-B / N-E / N-C / N-G / N-H)*: carried. **N-J adds no new instance** — persistent snapshot encoding is wholly internal authority.
- **Closed credential discriminant contract** *(OQ5 / COMMITTEE / DREP / ENACTMENT / PP)*.
- **Committee-enactment write-back contract** *(ENACTMENT)*.
- **All canonical types**: shapes frozen at the era / version they entered.
- **Handshake-negotiated version threading** *(N-A; strengthened in N-G + N-H)*: carried.
- **TCB color assignments**: per `.idd-config.json` `core_paths`. **N-J additions:** `ade_ledger::snapshot::*` (8 files) are BLUE (under the already-BLUE `ade_ledger` crate prefix); `ade_runtime::rollback::persistent_cache` is GREEN-inside-RED-crate (single-file GREEN classification — pure adapter; no clock, no async, BTreeMap-only).
- **`ChainDb` / `SnapshotStore` / `Recoverable` trait shapes** (N-D). **N-J note:** `SnapshotStore`'s 3-method shape is now consumed at production scale by `PersistentSnapshotCache`; trait surface unchanged.
- **`AcceptedBlock` type-level broadcast gate** *(N-C-S5)*: carried.
- **`AdmittedBlock` type-level admission gate** *(N-H-S1)*: carried.
- **`RollbackContext` BLUE struct seam** *(N-I-S6)*: carried. **N-J note:** unchanged at the struct level; the `&dyn SnapshotReader` field now accepts the new persistent impl as a drop-in alternative to the in-memory impl.
- **`SnapshotCadence` BLUE-structural single-field discipline** *(N-I-S4 — DC-STORE-07)*: carried.
- **`forge_block` pure-transition contract** *(N-C-S3)*: carried.
- **Single source of leader truth** *(N-C-S3)*: carried.
- **Tx-admissibility prefix property** *(N-C-S3)*: carried.
- **Private-key custody RED-confinement** *(N-C-S1)*: carried.
- **Closed-grammar opcert byte authority** *(N-C-S2)*: carried.
- **OpCert serial counter strict monotonicity** *(N-C-S2)*: carried.

### Version-gated (can evolve across major versions)

- **New `CardanoEra` variant**: full coordinated change.
- **New Conway certificate tag** *(B3 / B4 / B5)*.
- **New `CoinSource` deposit-provenance** *(B3)*.
- **Pre-Conway single-tx validity** *(B2 extension point)*.
- **Full-scope `track_utxo=true` tx corpus** *(B2 extension point)*.
- **Conway block-body vkey-witness closure** *(B2-carried)*.
- **Conway governance certificate accumulation** *(B5)*.
- **Credential discriminant extension** *(declared non-goal)*.
- **Committee-enactment write-back** *(ENACTMENT)*.
- **Conway tx-body `proposal_procedures` decode** *(PP — wired)*.
- **TPraos full-block validity** *(B1 extension point)*.
- **TPraos producer** *(N-C declared non-goal — OQ-4 lock)*.
- **New `GovAction` / Plutus version variant**.
- **New `SignerSource` / `TxRejectClass` / `BlockRejectClass` / `OrderPolicy` variant**.
- **New protocol parameter field**. **N-J note:** new pparams field also means new wire field in `encode_pparams` — version-gated at the snapshot layer (today layout is implicit, anchored by `SCHEMA_VERSION = 1`; a future cluster ratifies the migration path).
- **New `ProducerTick` field** *(N-C extension point)*.
- **New `ForgeError` / `SchedulerInput` / `SchedulerEffect` variant**.
- **New `SelfAcceptError` variant** *(N-C extension point)*.
- **New `ServerStep` / `BlockFetchServerStep` / `ServerReply` etc.** *(N-G extension points)*: carried.
- **New `ReceiveEvent` variant** *(N-H — CN-PROTO-07 extension point)*: carried.
- **New `ReceiveEffect` variant** *(N-H)*: carried.
- **New `ReceiveError` variant** *(N-H)*: carried.
- **New `ChainDbWrite` impl** *(N-H; N-I extended to 2 methods)*: carried.
- **New `ChainDbWrite` trait method** *(N-H extension point)*: carried.
- **New `ReceiveDispatchError` variant** *(N-H)*: carried.
- **New `SnapshotReader` impl** *(N-I; **N-J used it once — added `PersistentSnapshotCache`**)*: closed seam. At this HEAD there are **two production impls** (`InMemorySnapshotCache`, `PersistentSnapshotCache`). New impls remain deliberate registry-tracked closed additions — not runtime plug-ins.
- **New `BlockSource` impl** *(N-I)*: carried; no second impl introduced by N-J.
- **New `MaterializeError` / `CommitRollbackError` variant** *(N-I)*: carried.
- **New `RollbackContext` field** *(N-I)*: carried.
- **New `SnapshotCadence` field** *(N-I — WITH MANDATORY CLUSTER RATIFICATION)*: carried.
- **New `SnapshotEncodeError` / `SnapshotDecodeError` / `StructuralReason` variant** *(NEW in N-J — extension point)*: closed sums; today 1 / 5 / 9 variants. New variant = closed-sum extension; version-gated.
- **New snapshot sub-state encoder/decoder pair** *(NEW in N-J — extension point — inside `ade_ledger::snapshot::*`)*: e.g. a future cluster adds a mempool snapshot or peer-table snapshot encoder as a sibling module. Each new sibling carries its own single-authority CI gate; CN-STORE-08's existing grep covers the workspace-wide tuple-bytes authority but new sibling sub-states need new grep entries.
- **`SCHEMA_VERSION` bump (v1 → v2)** *(NEW in N-J — extension point)*: closed versioned-schema seam. First field appended to the framing wire format triggers the bump. v1 readers fail-closed on v2 bytes (`SnapshotDecodeError::UnknownVersion`); a future cluster ratifies the v2 layout + decoder dispatch table.
- **New `PersistentSnapshotCache` field** *(NEW in N-J — extension point)*: closed struct; today 1 field (`store: &'a S`). Extension would be a deliberate cluster ratification (e.g. an in-memory hot-cache layer in front of the store).
- **New `PersistentCacheError` variant** *(NEW in N-J — extension point)*: closed sum; today 3 variants. Extension would map to a new upstream error category.
- **New CI check**: additive. (N-J added one — `ci_check_snapshot_encoder_closure.sh`.)
- **Pinned external crate bump**: Tier-5 rationale doc required.
- **New mini-protocol** / **Mini-protocol version-table bump**.
- **New `ChainEvent` / `ChainSelectionReject` / `StreamInput` variant**.
- **New `NetworkMagic`** *(N-B)*.
- **New `LedgerView` impl / LedgerState-backed `PoolDistrView` constructor**.
- **`BootstrapAnchorHash` preimage v2** *(N-B)*: hard version-gated.
- **N2N/N2C tx-submission → `mempool_ingress` ingress** *(N-E)*.
- **Live cardano-node N2N block-fetch acceptance / live N2N follow-mode admission** *(N-C / N-G / N-H)*: each reopens on operator availability.
- **Phase-4 cluster surface additions** (N-F): each cluster's wire surface gates additions via its own cluster doc.
- **Orchestrator-side persistent-capture wiring** *(NEW in N-J — extension point flagged by N-J close)*: Tier-5 wiring; today `maybe_capture_snapshot` writes only to `InMemorySnapshotCache`. A follow-on cluster extends the hook to also `PersistentSnapshotCache::capture(slot, &ledger, &chain_dep)`.

---

## 5. Module Addition Rules

Ade's workspace is small and color-disciplined. **PHASE4-N-J added
eight new BLUE submodules** (`ade_ledger::snapshot::{chain_dep,
utxo_state, cert_state, epoch_state, gov_state, ledger, framing,
error}` under a new `ade_ledger::snapshot` barrel), **one new GREEN
submodule inside `ade_runtime`** (`rollback::persistent_cache`,
inside the already-extant `ade_runtime::rollback` barrel from N-I),
**one new CI gate** (`ci_check_snapshot_encoder_closure.sh`), **three
new registry rules** (all `enforced`), and **flipped `DC-CONS-21`
from `declared` to `enforced` with `strengthened_in += PHASE4-N-J`
and `open_obligation` removed**. N-J added **no new crate**, **no
new external ingress wire-format frozen contract** (snapshot bytes
are internal-only persisted state), **no new operator-action probe
binary** (snapshot encoding has no Tier-1 wire-format counterpart).

**N-J also strengthened one cross-color dependency edge**:

1. `ade_runtime → ade_ledger` (already added in N-C; strengthened
   in N-G + N-H + N-I; **further strengthened in N-J**) — the GREEN
   `rollback::persistent_cache` adapter imports the new
   `ade_ledger::snapshot::framing::{encode_snapshot, decode_snapshot,
   SCHEMA_VERSION}` BLUE chokepoints + the `ade_ledger::snapshot::{SnapshotEncodeError,
   SnapshotDecodeError}` BLUE error sums + the existing
   `ade_ledger::rollback::SnapshotReader` trait + `ade_ledger::state::LedgerState`.
   Same direction (RED/GREEN → BLUE); allowed. Passes
   `ci_check_dependency_boundary.sh`.

**The module-addition rule N-J sets for future snapshot-side work:**

1. **A new snapshot-side BLUE primitive attaches inside
   `ade_ledger::snapshot::*`** (sibling of `chain_dep`,
   `utxo_state`, `cert_state`, `epoch_state`, `gov_state`, `ledger`,
   `framing`, `error`). The module MUST be BLUE: no clock, no rand,
   no I/O, no `HashMap`, no `tokio`, no `async`. New canonical types
   MUST be closed sums or closed structs; no `#[non_exhaustive]`;
   no `String`-bearing variants. Sub-state encoder/decoder pairs
   MUST be the SOLE pair for their sub-state's bytes (in-module
   single-authority); composite encoder/decoder pairs MUST be the
   SOLE pair for their composite shape (workspace-wide
   single-authority via CN-STORE-08 grep).
2. **A new snapshot encoder for a different state type attaches as
   a sibling module** under `ade_ledger::snapshot::*` (e.g. a future
   `ade_ledger::snapshot::mempool_state` or
   `ade_ledger::snapshot::peer_table`). Each sibling carries its own
   single-authority CI scope (extend `ci_check_snapshot_encoder_closure.sh`
   with the new pair).
3. **A new `SnapshotReader` impl attaches inside the appropriate
   runtime crate** (e.g. a hypothetical `ade_runtime::rollback::clustered_snapshot_store`
   for a multi-process backend). The module MUST be a pure function
   over its inputs (the decode side may consult I/O via the
   `SnapshotStore` trait but the result MUST be deterministic).
   Single production impl per snapshot backend; new backends attach
   via the `SnapshotStore` trait, not via re-implementing
   `SnapshotReader`.
4. **A new `SnapshotStore` impl** (the underlying byte store, e.g.
   a sharded redb backend or an S3-backed store) attaches inside
   `ade_runtime::chaindb` and is consumed by `PersistentSnapshotCache`
   unchanged (the cache is generic over `S: SnapshotStore + ?Sized`).
5. **A new closed snapshot error variant attaches inside
   `SnapshotEncodeError` / `SnapshotDecodeError` / `StructuralReason` /
   `PersistentCacheError`.** Closed-sum extension; version-gated; no
   `#[non_exhaustive]`.
6. **A `SCHEMA_VERSION` bump (v1 → v2)** is a deliberate cluster
   ratification — the decoder gains a v2 dispatch arm; v1 readers
   fail-closed on v2 bytes (`SnapshotDecodeError::UnknownVersion`);
   the migration discipline is set by N-J's `DC-STORE-09` enforcement.
7. **A new snapshot-paths registry rule attaches as a derived `DC-*` /
   `CN-*` family entry** with `code_locus`, `ci_script`, `tests`,
   `cross_ref`. Bidirectional cross-refs to consumed rules
   (`DC-CONS-21`, `DC-STORE-08`, `DC-STORE-09`, `CN-STORE-08`,
   `T-ENC-01`, `T-DET-01`, `CN-STORE-07`).

### Cross-cluster obligation pattern (carried — no N-J addition)

**N-J adds no new cross-cluster obligation** — snapshot encoding is
wholly internal and has no Tier-1 wire-format counterpart that
requires a live cross-impl probe. The N-H / N-G / N-C /
`blocked_until_operator_*_available` precedents stand unchanged.

### Operator-action evidence pattern (carried — no N-J addition)

**N-J adds no new operator-action probe binary** — the family
remains at five. Snapshot encoding is exercised by the inline test
set in `ade_ledger::snapshot::*` (deterministic round-trip,
fingerprint-equivalence, version-rejection, fingerprint-mismatch
rejection, pre-Conway rejection) plus the cross-impl equivalence
test `persistent_cache_matches_in_memory_cache_semantics` in
`ade_runtime::rollback::persistent_cache`.

### Cluster scope-edge pattern (carried — strengthened in N-J close)

**N-J carries the scope-edge pattern introduced by N-H + N-I** and
applies it to the **encoder era boundary**:

- The N-J scope edge (Conway-only encoder) is NOT a structured
  failure variant on every event — it is a deliberate cluster
  carve-out surfaced structurally as
  `SnapshotEncodeError::EraNotSupported` /
  `SnapshotDecodeError::EraNotSupported`. Pre-Conway is a low-
  priority future cluster (no current operational need; rollback
  target windows are bounded and Conway is the live era).
- **Unlike N-I's `DC-CONS-21` carve-out, N-J does NOT introduce a
  new registry rule with `open_obligation`** for pre-Conway —
  because there is no current operational need that the carve-out
  blocks. The discipline still holds: cluster carve-outs MUST be
  documented (here, in CODEMAP, and in the cluster doc); if a
  future operational need emerges, a registry rule is added at
  that point.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` | First line of every `.rs` is the contract banner. `lib.rs` carries `#![deny(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_arithmetic)]`. No `#[cfg(feature = ...)]`. No async. **N-J:** `ade_ledger::snapshot::*` modules have no `HashMap`/`HashSet`/`tokio`/`rand`/wall-clock (CI-defended by `ci_check_forbidden_patterns.sh` + `ci_check_snapshot_encoder_closure.sh`); `encode_snapshot` / `decode_snapshot` is the SOLE `pub fn` pair encoding/decoding the combined tuple bytes (CI grep); `SCHEMA_VERSION` is the SOLE `pub const` schema-version anchor (CI grep). | Other BLUE crates / submodules only. **N-J:** snapshot encoder composes `ade_ledger::fingerprint` (B3/B5) for the canonical pre-image of the fingerprint cross-check; reads `ade_ledger::state::LedgerState` + `ade_core::consensus::praos_state::PraosChainDepState` field-by-field; uses `ade_codec::cbor::*` primitives (`write_uint_canonical`, `write_bytes_canonical`, etc.); no direct dep on `ade_runtime`. | Any RED submodule or crate; GREEN in non-dev deps; `pallas_*` (except `ade_plutus`); async runtime; `HashMap`/`HashSet`/`IndexMap`; clock/rand/float/env/I/O. |
| **GREEN** | `ade_*` | Banner + deny attrs are project convention. **N-J:** `persistent_cache::PersistentSnapshotCache` is a pure adapter (no clock, no async, no `HashMap` — borrows the `SnapshotStore` and encodes/decodes via the BLUE framing layer); `PersistentCacheError` is a closed sum carrying upstream errors verbatim. | BLUE crates + standard library + ecosystem crates. **N-J:** the GREEN persistent-cache adapter lives inside `ade_runtime` (RED crate) — color is per-module per the cluster TCB Color Map. | `ade_runtime` for `ade_testkit`; RED submodules in non-test paths. Results must never feed back into a BLUE authoritative decision. |
| **RED** | `ade_*` | No special header. Free to use clocks, I/O, async, `HashMap`, signing keys. | Any BLUE / GREEN crate or submodule (one-way). **N-J strengthened the `ade_runtime → ade_ledger` edge** (RED/GREEN → BLUE via the new snapshot chokepoint pair + error sums + the existing `SnapshotReader` trait). | Cannot be depended on by BLUE. |

### New module checklist

1. **Add to `Cargo.toml` workspace members** (if a new crate).
2. **Declare TCB color** by editing `.idd-config.json` `core_paths` if BLUE.
3. **CI script update obligations** — extend the relevant BLUE-scoped
   scripts; for snapshot-domain sub-modules, model the new CI gate
   on `ci_check_snapshot_encoder_closure.sh` shape (workspace-wide
   single-authority grep + single-schema-version-const grep +
   positive presence of fingerprint + version cross-check paths).
4. **Add contract banner** (BLUE) to every `.rs` file.
5. **Add deny attributes** to `lib.rs` (BLUE).
6. **New canonical types:** add a `[[rules]]` block under family `T`
   in the invariant registry, plus a round-trip test. For new
   snapshot-domain authority rules, append `DC-STORE-0X` / `CN-STORE-0X`
   with bidirectional cross-ref to consumed rules.
7. **New operator-action probe binary:** (not applicable for the
   snapshot domain — internal authority).
8. **Cross-cluster obligation:** (not applicable for the snapshot
   domain at this cluster).
9. **Cluster scope-edge:** if the cluster deliberately scopes down a
   derived constraint, ship a separable registry rule with explicit
   `open_obligation` naming the follow-on cluster's deliverable —
   OR (if no current operational need blocks) document the carve-out
   in CODEMAP + the cluster doc without a registry entry. N-J's
   Conway-only encoder is the latter shape; N-I's `DC-CONS-21`
   `persistent_ledger_snapshot_encoding_follow_on_cluster` was the
   former shape (now closed by N-J itself).
10. **Run `cargo test --workspace` and the full CI script suite.**

### Phase 4 anticipated additions

- **PHASE4-N-J — FULLY CLOSED at this HEAD** (mechanical close;
  Conway-only encoder scope; pre-Conway → `EraNotSupported`
  structurally): code + CI gate + DC-STORE-08 + DC-STORE-09 +
  CN-STORE-08 (`enforced`) + DC-CONS-21 flipped to `enforced`
  (`open_obligation` removed) + 1 new CI script. No live-evidence
  obligation (internal authority).
- **PHASE4-N-I — FULLY CLOSED** (carried; `DC-CONS-21` `open_obligation`
  now removed by N-J). No carry-forward open obligation from N-I
  remains at this HEAD beyond OQ-5 snapshot eviction (Tier-5
  follow-on, no registry entry).
- **PHASE4-N-H — FULLY CLOSED** (carried). CE-N-H-6 live-evidence is
  `blocked_until_operator_peer_available`.
- **PHASE4-N-G — FULLY CLOSED** (carried). CE-N-G-8 live-evidence
  is `blocked_until_operator_peer_available`.
- **PHASE4-N-C — FULLY CLOSED** (carried). CE-N-C-8 live-evidence
  is `blocked_until_operator_stake_available`.
- **PROPOSAL-PROCEDURES-DECODE — FULLY CLOSED** (carried).
- **PHASE4-N-E — FULLY CLOSED** (carried).
- **NEW future cluster — Orchestrator-side persistent-capture wiring**
  *(NEW candidate flagged by N-J close)*: thin RED extension of
  `maybe_capture_snapshot` to also write through
  `PersistentSnapshotCache::capture`. Tier-5; no BLUE invariants
  change. Surface for the next planner; do not invent invariants
  here.
- **NEW future cluster — Snapshot eviction policy** *(carried from
  N-I; now doubled by N-J)*: Tier-5 operational concern applying to
  both the in-memory cache and the persistent store. Must remain
  replay-deterministic.
- **NEW future cluster — Multi-peer fork choice** *(carried; now
  doubly-enabled by N-I + N-J)*: Praos longest-chain across competing
  `PerPeerReceiveState[]`. Re-uses `RollbackContext` to roll back
  losing forks; now restart-safe via N-J's persistent snapshots.
- **NEW future cluster — N2C local-chain-sync receive surface**
  *(carried)*: operator-side N2C clients consume Ade's chain via
  `LocalChainSyncMessage`.
- **NEW low-priority future cluster — Pre-Conway snapshot encoder**
  *(NEW candidate flagged by N-J close)*: widen
  `encode_ledger_state` / `decode_ledger_state` to Babbage and
  earlier eras. No current operational need; rollback target windows
  are bounded.
- **NEW future-evolution seam — Snapshot schema migration v1 → v2**
  *(NEW candidate flagged by N-J close)*: `SCHEMA_VERSION: u32 = 1`
  is the explicit anchor; first field appended bumps to 2. No
  present cluster; discipline set by `DC-STORE-09`.
- **Future cluster — `CE-N-H-6` / `CE-N-G-8` / `CE-N-C-8` live
  evidence re-open triggers** (carried).
- **Future node-binary cluster (`CE-NODE-N2C-LTX`)** (carried).
- **Tx-validity completeness follow-ups** (carried).
- **PP OQ-1..OQ-4 follow-ups** (carried).
- **N-F (operator API)**: thin RED layer mapping a closed Query
  enum to gRPC/HTTP.

**These placements are candidates** — user confirmation needed at
cluster entry.

---

## 6. Forbidden Patterns (per color)

### BLUE (universal IDD prohibitions; enforced by CI where marked)

- No `HashMap`, `HashSet`, `IndexMap`, `IndexSet`.
- No `SystemTime`, `Instant`, `std::time::*` clocks.
- No `rand::thread_rng`, `thread::spawn`.
- No `f32`, `f64`, floating-point arithmetic.
- No `std::fs`, `std::net`, `tokio`, `async fn`.
- No `anyhow`; `unwrap`/`expect`/`panic` denied at the lint level.
- No `unsafe` outside an explicit allowlist.
- No `#[cfg(feature = ...)]` semantic gating.
- No signing patterns in BLUE.
- No re-hashing of `canonical_bytes` or re-encoded bytes — wire bytes only.
- No construction of `PreservedCbor` outside `ade_codec`.
- No raw CBOR decoding in any BLUE crate except `ade_codec` and the
  single allowlisted file `crates/ade_plutus/src/evaluator.rs`. **N-J
  carve-out:** `ade_ledger::snapshot::*` uses `ade_codec::cbor::*`
  read/write primitives directly (`write_uint_canonical`,
  `write_bytes_canonical`, `write_array_header`, `read_any_int`,
  `read_array_header`, `read_bytes`, etc.) — this is a structured
  composition over the BLUE codec layer, not raw CBOR decoding. The
  decoder produces `LedgerState` / `PraosChainDepState` field by
  field via codec primitives.
- No `pallas_*` reference outside `ade_plutus`.
- **(N-A specific)** Carried.
- **(N-B specific)** Carried.
- **(B1 specific)** Carried.
- **(B2 specific)** Carried.
- **(B3 / B4 / B5 specific)** Carried.
- **(OQ5 / COMMITTEE / DREP / ENACTMENT-COMMITTEE-WRITEBACK)** Carried.
- **(N-E specific — closed BLUE chokepoint `mempool_ingress`)** Carried.
- **(PP specific — closed BLUE sub-grammar `decode_proposal_procedures`)** Carried.
- **(N-C-S1..S7 specific)** All carried.
- **(N-G-S1..S4 specific)** All carried.
- **(N-H-S1..S6 specific)** All carried.
- **(N-I-S1..S6 specific)** All carried.
- **(N-J-S1 specific — closed BLUE snapshot error sums)**
  `SnapshotEncodeError` / `SnapshotDecodeError` / `StructuralReason`
  MUST be closed sums; no `#[non_exhaustive]`; no `String` (use
  `StructuralReason` for tag-rather-than-text rejection). Round-trip-
  through-pattern-match tests confirm sixth-variant additions fail
  to compile.
- **(N-J-S2..S5 specific — sub-state encoder/decoder discipline)**
  Each sub-state encoder/decoder pair (`encode_utxo_state`,
  `encode_cert_state`, `encode_epoch_state`, `encode_pparams`,
  `encode_gov_state`, `encode_conway_deposit_params`) MUST be the
  SOLE `pub fn` pair for its sub-state's bytes IN-MODULE. Each pair
  MUST use BTreeMap iteration only (canonical ordering); definite-
  length CBOR containers throughout; no `HashMap`/`HashSet`/`tokio`/
  `rand`/wall-clock/float.
- **(N-J-S6 specific — composite LedgerState encoder/decoder)**
  `encode_ledger_state` / `decode_ledger_state` MUST be the SOLE
  `pub fn` pair encoding/decoding `LedgerState` bytes in the
  workspace (CI grep CN-STORE-08). Field order MUST match
  `ade_ledger::fingerprint`'s deterministic field walk — adding a
  new field to `LedgerState` requires extending both
  `fingerprint` and `encode_ledger_state` / `decode_ledger_state` in
  the same cluster.
- **(N-J-S7 specific — combined-snapshot framing + CN-STORE-08 +
  DC-STORE-08 + DC-STORE-09)** `encode_snapshot` / `decode_snapshot`
  MUST be the SOLE `pub fn` pair encoding/decoding `(LedgerState,
  PraosChainDepState)` combined snapshot bytes in the workspace (CI
  grep CN-STORE-08). `encode_chain_dep` / `decode_chain_dep` MUST be
  the SOLE pair for `PraosChainDepState` bytes (CI grep). The decoder
  MUST verify the version tag BEFORE any payload work (DC-STORE-09 —
  unknown versions reject before decode of ledger or chain-dep) AND
  MUST recompute + verify the fingerprint AFTER decode (DC-STORE-08
  — fingerprint mismatch rejects). The encoder MUST embed the source
  state's `fingerprint(ledger).combined`. `SCHEMA_VERSION: u32 = 1`
  MUST be the SOLE `pub const SCHEMA_VERSION` in `crates/` (CI grep
  DC-STORE-09). Pre-Conway → `EraNotSupported` structurally on both
  encode and decode.

### GREEN (`ade_testkit` incl. all corpora; `ade_runtime::consensus::{candidate_fragment, chain_selector}`; `ade_ledger::mempool::{policy, canonicalize}`; the two `ade_core_interop` N-E bridges; `ade_runtime::producer::{tick_assembler, broadcast_to_served, served_chain_lookups}`; `ade_runtime::receive::{events_to_state, in_memory_chain_write}`; `ade_runtime::rollback::{cadence, in_memory_cache, chaindb_block_source}` (N-I-S4); **`ade_runtime::rollback::persistent_cache` — NEW in N-J-S8**)

- No nondeterminism that leaks into stored fixtures — fixtures must
  be byte-reproducible.
- No participation in authoritative outputs.
- No `HashMap` even in test helpers — `BTreeMap` only.
- No import of `ade_runtime` from `ade_testkit`.
- (carried bullets per prior revision)
- **(`ade_runtime::rollback::persistent_cache`, NEW in N-J-S8)**
  Single production persistent impl of `SnapshotReader`. Pure
  adapter over any `SnapshotStore`; borrows the store (lifetime
  `'a`); holds no in-memory state. MUST NOT use `HashMap`/`HashSet`/
  wall-clock/`tokio`/`rand`. `nearest_le` MUST treat decode failure
  as `None` (treats corrupt persisted bytes as "no usable snapshot
  here" — never panics). `PERSISTENT_CACHE_SCHEMA_VERSION` MUST
  equal `ade_ledger::snapshot::framing::SCHEMA_VERSION` (test pins
  the equality). MUST NOT introduce a second `pub fn` returning
  `Vec<u8>` from `&LedgerState` or `&PraosChainDepState` — the
  framing layer is the sole authority (CN-STORE-08 grep).

### RED (`ade_runtime`, `ade_node`, `ade_network::mux::transport`, `ade_network::session`, `ade_network::bin::capture_*`, `ade_runtime::consensus::genesis_parser`, `ade_core_interop` (incl. five live-session probe binaries), the RED-behavior `ade_ledger::consensus_input_extract` scan; `ade_runtime::producer::{signing, keys, scheduler, broadcast}` (N-C); `ade_runtime::network::n2n_server` (N-G-S6); `ade_runtime::receive::orchestrator` (N-H-S4); `ade_runtime::rollback::snapshot_writer` (N-I-S5))

- No direct mutation of `ade_ledger` state — all transitions go
  through the established BLUE chokepoints. **(N-J carve-out)**
  `PersistentSnapshotCache::capture` does not mutate `LedgerState` or
  `PraosChainDepState` — it reads from them by reference and writes
  encoded bytes to the underlying `SnapshotStore`. The cache itself
  holds no state; reading via `nearest_le` returns owned clones
  (decoded from bytes).
- No bypassing `ade_codec` to construct semantic types from raw bytes.
  **(N-J-strengthened)** Returning `Vec<u8>` from `&LedgerState` or
  `&PraosChainDepState` from any `pub fn` other than the canonical
  encoders in `ade_ledger::snapshot::*` is CI-forbidden (CN-STORE-08
  workspace-wide grep).
- (`ade_runtime` specifically) Existing `ade_runtime → ade_ledger`
  edge is **further strengthened in N-J** — the persistent-cache
  adapter consumes the new `ade_ledger::snapshot::framing::*` BLUE
  chokepoints + `ade_ledger::snapshot::{SnapshotEncodeError,
  SnapshotDecodeError}` error sums. Passes
  `ci_check_dependency_boundary.sh`.
- (`ade_network::mux::transport`) No protocol logic.
- (`ade_network::session`) Composition glue only.
- (`ade_network::bin::capture_*`) Live-interop tools only.
- (`ade_runtime::consensus::genesis_parser`) No re-derivation of the
  bootstrap anchor outside `compute_anchor_hash`.
- (`ade_ledger::consensus_input_extract`) Pure-over-bytes.
- (N-E live N2N operator-action session) Carried.
- (Deferred RED operator-action surfaces — CE-NODE-N2C-LTX) Carried.
- (`ade_core_interop`) Live-interop driver only; library tests
  `#[ignore]`-gated. **N-J added no new binary.**
- **(N-C-S1 / S6 specific — `ade_runtime::producer::*`)** All carried.
- **(N-G-S6 specific — `ade_runtime::network::n2n_server`)** Carried.
- **(N-H-S4 specific — `ade_runtime::receive::orchestrator`)** Carried.
- **(N-I-S5 specific — `ade_runtime::rollback::snapshot_writer`)**
  Carried. **N-J note:** the hook still talks only to
  `InMemorySnapshotCache`; the persistent-capture call site is the
  flagged Tier-5 follow-on (orchestrator-side persistent-capture
  wiring cluster).

### Project-specific additions

- **No commits of credentials, hostnames, IPs, private keys** —
  enforced by `ci_check_no_secrets.sh`. **N-J:** the snapshot
  encoder writes byte payloads derived from `LedgerState` /
  `PraosChainDepState` — these contain pool / DRep / stake-credential
  HASHES (no private keys; never has). Snapshot bytes are
  byte-safe under existing redaction posture; no new redaction
  obligation introduced.
- **No `Phase 4 internal-mode mock network`** — Tier 1 surfaces must
  be exercised against real cardano-node peers. **N-J:** snapshot
  encoding is internal authority (no Tier-1 wire-format counterpart);
  the cross-impl equivalence test
  `persistent_cache_matches_in_memory_cache_semantics` is a
  structural-agreement harness within the workspace (two production
  impls of the same trait produce identical lookups). The "real
  peer" rule does not apply.
- **No collapsing wire and canonical bytes** — dual-authority rule.
  **N-J:** snapshot bytes are CANONICAL bytes (single-authority,
  version-tagged, fingerprint-embedded); they are NOT wire bytes
  (never leave the local node). The two-authority rule is
  preserved: Cardano wire bytes stay frozen at the protocol spec;
  snapshot bytes stay frozen at the project-internal canonical
  schema. The framing layer's `SCHEMA_VERSION` is the version
  anchor; `fingerprint` is the cross-check.
- **No Tier 5 surface without a stated rationale**.
- **No "we'll match it later" stubs on Tier 1 surfaces** — Tier 1
  closure is hard-gated. **N-J:** the Conway-only encoder scope is
  NOT a Tier 1 stub — snapshot encoding is wholly internal authority
  (no Tier 1 counterpart). Pre-Conway encoding is a low-priority
  candidate seam with no current operational need; the carve-out
  surfaces structurally as `EraNotSupported` and is documented here
  + in CODEMAP. Same discipline as N-I's `RollbackOutOfScope`
  treatment of unreachable rollback ranges.

---

## Cross-references

- CODEMAP: `docs/ade-CODEMAP.md` — module-by-module authority table,
  upstream of this document. **Cross-reference check at this HEAD:**
  CODEMAP is being regenerated in parallel; pending the regen,
  CODEMAP may pin pre-N-J HEAD `75f75da`. The new BLUE submodule
  tree (`ade_ledger::snapshot::{chain_dep, utxo_state, cert_state,
  epoch_state, gov_state, ledger, framing, error}`) and the new
  GREEN submodule (`ade_runtime::rollback::persistent_cache`) are
  not yet in the prior CODEMAP. The next CODEMAP regen picks these
  up mechanically. CI count moves from 54 → 55.
- Invariant registry: `docs/ade-invariant-registry.toml` — rule
  families incl. T / CN / DC / OP / RO. **N-J added:**
  `DC-STORE-08` (`enforced`, `ci_script =
  ci/ci_check_snapshot_encoder_closure.sh`); `DC-STORE-09`
  (`enforced`, same CI script); `CN-STORE-08` (`enforced`, same CI
  script); flipped `DC-CONS-21` from `declared` to `enforced` with
  `strengthened_in += PHASE4-N-J` and `open_obligation` removed.
  Total: 206 → 209 entries.
- Phase 4 cluster plan: `docs/active/phase_4_cluster_plan.md`.
- Tier doctrine: `docs/active/CE-79_gate_statement.md` and
  `docs/active/CE-79_tier5_addendum.md`.
- Persistent snapshot encoder invariants sketch:
  `docs/planning/persistent-snapshot-encoder-invariants.md` (the
  upstream sketch the cluster doc derives from).
- Ledger snapshot + rollback invariants sketch (N-I upstream):
  `docs/planning/ledger-snapshot-rollback-invariants.md`.
- Receive-side bridge invariants sketch (N-H upstream):
  `docs/planning/receive-side-bridge-invariants.md`.
- Cluster N-D / N-A / N-B / N-H / N-I / B1 / B2 / B3 / B4 / B5 /
  OQ5-CREDENTIAL-FIDELITY / COMMITTEE-CRED-FIDELITY /
  DREP-VOTE-FIDELITY / ENACTMENT-COMMITTEE-FIDELITY /
  ENACTMENT-COMMITTEE-WRITEBACK / PHASE4-N-E /
  PROPOSAL-PROCEDURES-DECODE / PHASE4-N-C / PHASE4-N-G: all closed;
  cluster docs carried.
- **Cluster PHASE4-N-J (CLOSED at this HEAD; mechanical half;
  Conway-only encoder scope)**: the cluster doc + slices
  `cluster.md, N-J-S{1..8}.md` at `docs/clusters/PHASE4-N-J/`
  (pending archive to `completed/`). WIRES AND CLOSES the
  persistent ledger snapshot encoder: BLUE `PraosChainDepState`
  encoder/decoder + closed error sums (S1); BLUE `UTxOState` encoder/
  decoder (S2); BLUE `CertState` encoder/decoder (S3); BLUE
  `EpochState` (+ `SnapshotState`) encoder/decoder (S4); BLUE
  `ConwayGovState` + `ProtocolParameters` + `ConwayOnlyDepositParams`
  encoder/decoder triple (S5); BLUE composite `encode_ledger_state` /
  `decode_ledger_state` assemble (S6); BLUE combined-snapshot
  framing + `SCHEMA_VERSION` + CI gate (S7 — closes DC-STORE-08 +
  DC-STORE-09 + CN-STORE-08); GREEN `PersistentSnapshotCache` +
  cross-impl equivalence test (S8 — closes DC-CONS-21). Added one
  CI script (count 54 → 55); added three derived registry rules
  (total 206 → 209); flipped DC-CONS-21 to `enforced` with
  `strengthened_in += PHASE4-N-J` and `open_obligation` removed.
  Five operator-action probe binaries remain in the family (no N-J
  addition).
- **Future obligation: orchestrator-side persistent-capture wiring**
  — `maybe_capture_snapshot` extension to call
  `PersistentSnapshotCache::capture` alongside in-memory capture.
  Tier-5 operational follow-on.
- **Future obligation: snapshot eviction policy cluster** — Tier-5
  operational concern; now applies to BOTH in-memory cache AND
  persistent store; named candidate seam.
- **Future obligation: `CE-N-H-6`** — carried.
- **Future obligation: `CE-N-G-8`** — carried.
- **Future obligation: `CE-N-C-8`** — carried.
- **Future obligation: `CE-NODE-N2C-LTX`** — carried from N-E.
- **Future seam candidates (flagged by N-J close)**: orchestrator-side
  persistent-capture wiring (highest-priority Tier-5 follow-on);
  snapshot eviction policy cluster (doubled scope); pre-Conway
  snapshot encoder (low-priority); snapshot schema migration v1 → v2
  (no present cluster; discipline anchored); multi-peer fork choice
  cluster (now doubly-enabled by N-I + N-J restart-safe rollback);
  N2C local-chain-sync receive surface cluster.
