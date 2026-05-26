# Persistent ledger snapshot encoder — Invariants sketch

**Concept.** Close PHASE4-N-I's deferred `DC-CONS-21` open_obligation
by making snapshot bytes persistent, canonical, versioned,
fingerprint-checked, and usable through the same `SnapshotReader`
authority as N-I. Rollback advances from "works with in-memory
snapshots" to **restart-safe as long as a persisted snapshot exists**.

Cluster-id candidate: **PHASE4-N-J**.

**Status.** Invariants sketch only. Cluster plan + slice docs
follow. Not yet implementation-bound.

## Scope decisions (locked before this sketch)

1. **Conway-only encoder scope; fail-closed for pre-Conway.**
   `SnapshotEncodeError::EraNotSupported { era }` on encode of a
   pre-Conway `LedgerState`. `SnapshotDecodeError::EraNotSupported
   { era }` on decode of bytes whose embedded era field is
   pre-Conway. This cluster closes DC-CONS-21 **explicitly scoped
   to the persistent Conway snapshot encoder**; pre-Conway support
   is a future cluster.
2. **Schema version = 1.** Bytes start with a closed `u32` version
   tag. Unknown version → `SnapshotDecodeError::UnknownVersion`.
   Future schema changes bump version; migration is a future
   cluster.
3. **Fingerprint cross-check embedded.** Bytes include source
   state's `LedgerFingerprint.combined`. Decode recomputes on the
   decoded state and rejects on mismatch.
4. **Canonical CBOR.** Encoder uses `ade_codec::cbor::*` writers.
   Definite-length arrays/maps; BTreeMap iteration only; no
   HashMap/floats/wall-clock.
5. **Single encoder authority (CN-STORE-08 new).** The
   `encode_*` + `decode_*` pairs live at exactly one site per
   sub-state, mirroring CN-STORE-07's single-materialize
   discipline.
6. **No eviction in this cluster.** Persistent store grows
   monotonically until operator-driven `delete_snapshot`.
   Eviction is operational policy; it must not become entangled
   with the semantic question "are persisted snapshot bytes
   canonical and replay-safe?" A future cluster may introduce
   eviction with an explicit rule such as "never evict snapshots
   required to satisfy the supported rollback window," but that
   is not part of N-J.
7. **Cross-impl equivalence required.** `PersistentSnapshotCache`
   and `InMemorySnapshotCache` must produce snapshot triples with
   equal `LedgerFingerprint.combined` for the same admitted state.
   This is the witness that "in-memory and persistent are the same
   authority."
8. **Split per sub-state; UTxOState gets its own slice.** Mechanical
   volume in this cluster is high; per-sub-state slices keep
   review tractable and make slice-exit criteria meaningful.
   UTxOState is the largest field and the most important place to
   prove deterministic BTreeMap traversal — it gets a dedicated
   slice.

## 1. What must always be true

- **I-1 — Encode/decode round-trip preserves fingerprint
  (DC-CONS-21 closure target).** For any reachable `(LedgerState,
  PraosChainDepState)`, `fingerprint(decode_snapshot(encode_snapshot
  (s).unwrap()).unwrap()) == fingerprint(s)`. The fingerprint
  function (already in `ade_ledger::fingerprint`) is the canonical
  equivalence witness.
- **I-2 — Encoder canonicality.** `encode_snapshot(s)` is
  byte-identical across runs. BTreeMap iteration only; no
  HashMap; no wall-clock; no floats; definite-length CBOR
  containers.
- **I-3 — Version tag is the first item.** Bytes start with a
  CBOR-encoded `u32` version (== 1). Decoder reads version first
  and rejects unknown versions **before decoding the ledger or
  chain-dep payload** (the version's own CBOR item is parsed; no
  payload parsing on mismatch).
- **I-4 — Source fingerprint embedded + cross-checked.** Encoder
  emits source state's `LedgerFingerprint.combined` as a framing
  field; decoder recomputes the fingerprint on the decoded state
  and rejects on mismatch. Catches corruption + accidental schema
  drift.
- **I-5 — Single canonical encoder authority (CN-STORE-08 new).**
  `encode_ledger_state` + `decode_ledger_state` are the SOLE
  `pub fn` pair in the project encoding/decoding `LedgerState`
  to/from bytes. Same for `encode_chain_dep` /
  `decode_chain_dep`. Same for `encode_snapshot` /
  `decode_snapshot`. CI grep enforces.
- **I-6 — Pre-Conway encode/decode is structurally rejected.**
  Encoder: pre-Conway era field → `EraNotSupported`. Decoder:
  bytes whose encoded era field is pre-Conway →
  `EraNotSupported`. Same scope discipline as N-I.
- **I-7 — `PersistentSnapshotCache` satisfies the
  `SnapshotReader` trait contract.** `nearest_le(target_slot)`
  returns the largest snapshot slot ≤ target, decoded into
  `(SlotNo, LedgerState, PraosChainDepState)`. Returns `None`
  when no such snapshot exists in the underlying `SnapshotStore`.
- **I-8 — Cross-impl equivalence.** For any admitted state `S`,
  one `put_snapshot` round trip through a `PersistentSnapshotCache`
  yields a snapshot triple whose fingerprint equals what an
  `InMemorySnapshotCache` would have returned for the same `S`
  at the same target slot.

## 2. What must never be possible

- **¬P-1.** Decoding garbage bytes silently producing a
  `LedgerState` with arbitrary contents. (Version tag +
  fingerprint cross-check rejects.)
- **¬P-2.** `encode_snapshot(s)` producing different bytes across
  runs that reached the same state `s`. (Canonical encoder.)
- **¬P-3.** Decode succeeding for bytes whose version tag doesn't
  match the encoder's emitted version. (Forward compatibility
  protection — rejected before payload decode.)
- **¬P-4.** Decode succeeding when the recomputed fingerprint
  differs from the embedded fingerprint. (Corruption detection.)
- **¬P-5.** Two parallel encoders/decoders for `LedgerState` or
  `PraosChainDepState` existing in the workspace. (CI single-
  authority gate.)
- **¬P-6.** HashMap iteration / wall-clock / floats / rand in any
  encoder/decoder production code.
- **¬P-7.** Encoding a pre-Conway `LedgerState` without an
  `EraNotSupported` error. Decoding bytes whose embedded era is
  pre-Conway without an `EraNotSupported` error.
- **¬P-8.** A `PersistentSnapshotCache` returning a triple whose
  fingerprint differs from the `InMemorySnapshotCache` for the
  same admitted state. (Cross-impl equivalence.)
- **¬P-9.** A sub-state slice merging "encoder mostly done,
  decoder later." Every sub-state slice must prove
  `decode(encode(sub_state)) == sub_state-equivalent`.

## 3. What must remain identical across executions

- **Snapshot bytes** for any reachable `(LedgerState,
  PraosChainDepState)`. `encode_snapshot(s)` byte-identical
  across runs.
- **Decoded state fingerprint** for any valid snapshot bytes.
  `fingerprint(decode_snapshot(b).unwrap())` deterministic over
  `b`.
- **`PersistentSnapshotCache::nearest_le(target_slot)` triple
  fingerprint** for the same persistent store contents and target.

## 4. What must be replay-equivalent

For a synthetic fixture corpus of `(initial_state,
applied_block_sequence)` triples:

- Two runs admit the same blocks → produce identical states →
  produce byte-identical snapshot bytes via `encode_snapshot`.
- A snapshot encoded at slot `T`, written to a `SnapshotStore`,
  decoded back via `PersistentSnapshotCache::nearest_le(T)` →
  the decoded state's fingerprint equals the original.
- N-I's existing replay test `materialize_replay_forward_equals_
  direct_apply` continues to pass with `PersistentSnapshotCache`
  in place of the in-memory cache.

## 5. State transitions in scope

```text
// BLUE — sub-state encoders/decoders (per slice)
fn encode_chain_dep(&PraosChainDepState) -> Result<Vec<u8>, SnapshotEncodeError>;
fn decode_chain_dep(&[u8]) -> Result<PraosChainDepState, SnapshotDecodeError>;

fn encode_utxo_state(&UTxOState) -> Result<Vec<u8>, SnapshotEncodeError>;
fn decode_utxo_state(&[u8]) -> Result<UTxOState, SnapshotDecodeError>;

fn encode_cert_state(&CertState) -> Result<Vec<u8>, SnapshotEncodeError>;
fn decode_cert_state(&[u8]) -> Result<CertState, SnapshotDecodeError>;

fn encode_epoch_state(&EpochState) -> Result<Vec<u8>, SnapshotEncodeError>;
fn decode_epoch_state(&[u8]) -> Result<EpochState, SnapshotDecodeError>;

fn encode_gov_state(&Option<ConwayGovState>, &Option<ConwayOnlyDepositParams>,
                    &[GovActionState], &committee_map, &quorum_pair)
    -> Result<Vec<u8>, SnapshotEncodeError>;
fn decode_gov_state(&[u8]) -> Result<(...), SnapshotDecodeError>;

// BLUE — assemble
fn encode_ledger_state(&LedgerState) -> Result<Vec<u8>, SnapshotEncodeError>;
fn decode_ledger_state(&[u8]) -> Result<LedgerState, SnapshotDecodeError>;

// BLUE — combined snapshot with version + fingerprint framing
fn encode_snapshot(&LedgerState, &PraosChainDepState)
    -> Result<Vec<u8>, SnapshotEncodeError>;
fn decode_snapshot(&[u8]) -> Result<(LedgerState, PraosChainDepState),
                                    SnapshotDecodeError>;

// GREEN — persistent SnapshotReader adapter
struct PersistentSnapshotCache<'a, S: SnapshotStore> { store: &'a S }
impl<'a, S> SnapshotReader for PersistentSnapshotCache<'a, S> { ... }

// RED — persistent snapshot-write hook (extends N-I's S5 hook)
fn maybe_capture_snapshot_persistent<S: SnapshotStore>(
    store: &S,
    cadence: SnapshotCadence,
    effect: &ReceiveEffect,
    state: &ReceiveState,
) -> Result<bool, /* encode err | store err */>;
```

All BLUE functions pure, total. Errors are structured closed sums.

## 6. TCB color hypothesis

- **BLUE (new):**
  - `ade_ledger::snapshot::error` — `SnapshotEncodeError` +
    `SnapshotDecodeError` closed sums.
  - `ade_ledger::snapshot::chain_dep` — `encode_chain_dep` +
    `decode_chain_dep`.
  - `ade_ledger::snapshot::utxo_state` — `encode_utxo_state` +
    `decode_utxo_state` (UTxOState alone; the largest sub-state).
  - `ade_ledger::snapshot::cert_state` — `encode_cert_state` +
    `decode_cert_state`.
  - `ade_ledger::snapshot::epoch_state` — `encode_epoch_state` +
    `decode_epoch_state`.
  - `ade_ledger::snapshot::gov_state` — encoder/decoder for the
    Conway governance bundle.
  - `ade_ledger::snapshot::assemble` — `encode_ledger_state` /
    `decode_ledger_state` composing the above.
  - `ade_ledger::snapshot::combined` — `encode_snapshot` /
    `decode_snapshot` (version tag + fingerprint framing).
- **GREEN (new):**
  - `ade_runtime::rollback::persistent_cache` —
    `PersistentSnapshotCache` impl of `SnapshotReader`.
  - `ade_runtime::rollback::snapshot_writer` (extended) —
    `maybe_capture_snapshot_persistent` variant alongside the
    in-memory `maybe_capture_snapshot`.
- **RED (no new module):**
  - `ade_runtime::chaindb::SnapshotStore` (existing N-D trait +
    impls) is the persistent storage backend. The encoder/decoder
    layer doesn't touch RED.

## 7. Open questions (must resolve before cluster-plan)

1. **Sub-state encoder ordering within the LedgerState wrapper.**
   Lean: same field order as
   `ade_ledger::fingerprint::fingerprint` already walks (canonical
   and tested). Decoder reverses trivially.
2. **`Option`-typed Conway fields** (`gov_state`,
   `conway_deposit_params`). Encode as CBOR `null` for `None`,
   else the encoded sub-state. Closed two-variant shape.
3. **Rational values in protocol params.** Integer-ratio;
   encode as CBOR `[num, den]` array. No floats anywhere.
4. **UTxOState encoding scale concern.** Mainnet UTxO is large
   (millions of entries). N-J ships the encoder; runtime
   performance is a future Tier-5 concern. Round-trip tests use
   small synthetic fixtures + Conway-576 corpus subset.
5. **Eviction.** Out of scope per scope decision #6.
6. **CI gate for canonicality.** Lean YES — grep gate forbidding
   `HashMap` / `SystemTime` / `tokio` / `rand` / float literals
   in any `ade_ledger::snapshot::*` module.
7. **`maybe_capture_snapshot_persistent` error union shape.**
   Either a closed sum wrapping both encoder + store errors, or
   the function returns `Result<bool, PersistentCaptureError>`
   with `PersistentCaptureError::{Encode, Store}` variants. Lean
   the latter (closed sum).

## 8. Acceptance evidence shape

Mechanical CEs prove:

- Per-sub-state encode/decode round-trip → equivalent value
  (sub-state slices S1-S5). Each sub-state slice's exit criterion
  is `decode(encode(sub_state)) == sub_state-equivalent`.
- End-to-end `encode_snapshot` / `decode_snapshot` round-trip →
  fingerprint preserved + version verified + fingerprint
  cross-check verified (S7).
- Cross-impl equivalence:
  `InMemorySnapshotCache::nearest_le(T).fingerprint ==
  PersistentSnapshotCache::nearest_le(T).fingerprint` for the
  same admitted state (one `put_snapshot` round trip) (S8).
- N-I's `materialize_replay_forward_equals_direct_apply`
  continues to pass with `PersistentSnapshotCache` as the
  SnapshotReader impl (S8).

**DC-CONS-21 closes** when
`encode_then_decode_roundtrips_via_fingerprint` passes over a
corpus-derived set of admitted states, scoped explicitly to the
**persistent Conway snapshot encoder**. The `open_obligation` on
DC-CONS-21 lifts; status flips `declared` → `enforced`. Pre-Conway
support remains a future cluster.

No new `RO-LIVE` entry — persistence is purely internal authority;
no peer-facing surface change.

---

## Slice shape (preview — locks at /cluster-plan)

| Slice | Scope | Exit criterion |
|----|----|----|
| S1 | `PraosChainDepState` encoder/decoder | round-trip equality |
| S2 | `UTxOState` encoder/decoder | deterministic BTreeMap traversal + round-trip |
| S3 | `CertState` encoder/decoder | delegation/pool/DRep round-trip |
| S4 | `EpochState` encoder/decoder | protocol params + snapshots + rewards round-trip |
| S5 | `ConwayGovState` encoder/decoder | proposals + committee + quorum + Option fields |
| S6 | `LedgerState` assemble wrapper | field order matches fingerprint walk |
| S7 | combined snapshot framing | version tag + embedded fingerprint + cross-check |
| S8 | `PersistentSnapshotCache` + N-I integration | in-memory vs persistent equivalence; DC-CONS-21 closes |

---

## Proposed registry entries (3 new + 1 in-place close, to be confirmed)

```toml
[[rules]]
id = "DC-STORE-08"
tier = "derived"
statement = """
Snapshot encoder canonicality: encode_snapshot(s) is byte-identical
across runs. Encoder uses BTreeMap iteration only; no HashMap, no
wall-clock, no floats, no rand. Definite-length CBOR containers.
"""
source = "docs/planning/persistent-snapshot-encoder-invariants.md §1 (I-2)"
cross_ref = ["T-DET-01", "T-ENC-01", "DC-CONS-21"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"

[[rules]]
id = "DC-STORE-09"
tier = "derived"
statement = """
Snapshot bytes carry a closed u32 version tag (initial == 1) and
the source state's blake2b-256 fingerprint. Decoder reads the
version tag first and rejects unknown versions before decoding the
ledger or chain-dep payload; decoder recomputes the fingerprint on
the decoded state and rejects on mismatch.
"""
source = "docs/planning/persistent-snapshot-encoder-invariants.md §1 (I-3, I-4)"
cross_ref = ["DC-CONS-21", "DC-STORE-08"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"

[[rules]]
id = "CN-STORE-08"
tier = "release"
statement = """
Single encoder authority: encode_ledger_state + decode_ledger_state
+ encode_chain_dep + decode_chain_dep + encode_snapshot +
decode_snapshot are the SOLE pub fn pairs in the project encoding
or decoding LedgerState / PraosChainDepState / (LedgerState,
PraosChainDepState) to/from bytes. No parallel canonical encoders.
Type-level + CI grep enforcement, mirroring CN-STORE-07.
"""
source = "docs/planning/persistent-snapshot-encoder-invariants.md §1 (I-5)"
cross_ref = ["CN-STORE-07", "DC-CONS-21", "DC-STORE-08"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"
attack_rationale = "A parallel encoder may produce bytes that don't satisfy round-trip equivalence — a corruption attack vector if the wrong encoder is invoked."
evidence_notes = "Enforcement is type-level (single module hosts the encoder/decoder pair) + CI grep (no other pub fn returning Vec<u8> from &LedgerState or &PraosChainDepState across the workspace)."
```

And the **DC-CONS-21 closure** (updated in-place on S8's merge —
flip `status` from `declared` → `enforced`, populate code/tests/ci,
remove `open_obligation`):

```toml
# update in-place on S8 merge:
[[rules]]
id = "DC-CONS-21"
# ... statement / source / cross_ref unchanged ...
code_locus = "<sub-state encoders + assemble + combined + persistent cache>"
tests = ["encode_then_decode_roundtrips_via_fingerprint", "<8 sub-state round-trips>", "<cross-impl equivalence>"]
ci_script = "<snapshot encoder canonicality CI>"
status = "enforced"
# REMOVE: open_obligation = "persistent_ledger_snapshot_encoding_follow_on_cluster"
# Closure is explicitly scoped to the persistent Conway snapshot encoder.
# Pre-Conway support remains a future cluster.
```

## Existing rules this cluster will eventually strengthen

(`strengthened_in` appends recorded at `/cluster-doc` time.)

- `T-DET-01` — new authoritative-deterministic surface (snapshot
  bytes are deterministic encoding of authoritative state).
- `T-ENC-01` — **canonical persisted/replay byte path for internal
  snapshot evidence**. (Not a hash-critical Cardano protocol
  wire-byte path; the dual byte authority model distinguishes
  preserved wire bytes from internal canonical encoding.)
- `DC-CONS-20` — rollback atomicity is now restart-safe (the
  persistent SnapshotReader closes the across-restart gap that
  N-I's in-memory variant left open).
- `DC-CONS-22` — replay-forward correctness over persistent
  snapshots equals direct-apply (cross-impl equivalence is the
  end-to-end witness).
- `CN-STORE-07` — single materialize authority's input source is
  now persistent (no behavior change in the materialize driver
  itself; just a new production `SnapshotReader` impl).

## Related

- [[project-phase4-n-i-handoff]] — the deferring cluster;
  DC-CONS-21's `open_obligation` names this cluster.
- [[project-phase4-n-h-handoff]] — receive-side bridge; rollback
  authority becomes restart-safe.
- [[feedback-fail-closed-validation]] — applies to ¬P-1..¬P-4
  (decode error paths).
- [[feedback-diverge-on-internal-surfaces]] — snapshot bytes are
  permitted internal divergence (Tier 5); Ade's canonical format,
  NOT cardano-node parity.
