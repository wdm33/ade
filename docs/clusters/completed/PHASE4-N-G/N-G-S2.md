# Invariant Slice — PHASE4-N-G S2

## Slice Header

**Slice Name:** `ServedChainSnapshot` canonical type + `served_chain_admit` (deterministic, `BTreeMap`-backed, append-only, broadcast-gate-preserving)
**Cluster:** PHASE4-N-G
**Status:** In Progress
**CEs addressed:** CE-N-G-2
**Registry effect on merge:** `CN-CONS-07.strengthened_in += "PHASE4-N-G"` (broadcast gate now preserved across the network seam — only `AcceptedBlock` can enter the served chain)
**Dependencies:** N-G-S1 (uses `accepted_block_header_bytes` reflectively for the iteration-ordering proof; not in the admit path)

---

## Intent

Introduce the BLUE canonical index the producer-side server reducers
(S3, S4) read from. `ServedChainSnapshot` is a deterministic,
append-only, `BTreeMap`-backed map from `(slot, block_hash)` to the
`AcceptedBlock` that admitted those bytes. The only path bytes enter
this index is `served_chain_admit(snapshot, accepted)`, which derives
the key from the bytes via `decode_block` — there is no separate
"asserted hash" the caller can mismatch.

This is the structural enforcement of N-G's primary invariant: every
byte the server pump later puts on the wire (header projection in
chain-sync `RollForward`, body bytes in block-fetch `Block { bytes }`)
is traceable through this single canonical index back to an
`AcceptedBlock` token that cleared `self_accept`. `CN-CONS-07` is
preserved across the network seam.

---

## The change

### 1. New module `crates/ade_ledger/src/producer/served_chain.rs`

```rust
use std::collections::BTreeMap;

use ade_types::{Hash32, SlotNo};

use crate::block_validity::{decode_block, BlockValidityError};
use crate::producer::AcceptedBlock;

/// Canonical, deterministic, append-only snapshot of `AcceptedBlock`
/// tokens keyed by `(slot, block_hash)`. `BTreeMap`-backed iteration
/// order is the only iteration order; no `HashMap`/`HashSet`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServedChainSnapshot {
    blocks: BTreeMap<(SlotNo, Hash32), AcceptedBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServedChainAdmitError {
    /// `decode_block` rejected the AcceptedBlock bytes. Should never
    /// fire for a real `AcceptedBlock` (the byte path through
    /// `self_accept` already proved decode-validity); the variant
    /// exists for strict totality.
    Decode(BlockValidityError),
    /// Two distinct AcceptedBlock byte sequences resolved to the same
    /// (slot, hash) key. Cryptographically unreachable under blake2b_256
    /// header hashing; the variant exists to make the structural
    /// invariant explicit.
    KeyByteConflict { slot: SlotNo, hash: Hash32 },
}

impl ServedChainSnapshot {
    pub fn new() -> Self { Self { blocks: BTreeMap::new() } }
    pub fn len(&self) -> usize { ... }
    pub fn is_empty(&self) -> bool { ... }

    /// Lookup the bytes admitted at `(slot, hash)`. Returns the
    /// AcceptedBlock-derived slice byte-identically.
    pub fn block_bytes(&self, slot: SlotNo, hash: &Hash32) -> Option<&[u8]>;

    /// Iterate `(slot, hash, bytes)` in BTreeMap order over an
    /// inclusive range of keys. Used by S4's block-fetch reducer.
    pub fn range_bytes(
        &self,
        from: (SlotNo, Hash32),
        to: (SlotNo, Hash32),
    ) -> impl Iterator<Item = (SlotNo, &Hash32, &[u8])> + '_;

    /// Iterate every admitted block in BTreeMap order. Used by S3 +
    /// fingerprint.
    pub fn iter(&self) -> impl Iterator<Item = (SlotNo, &Hash32, &[u8])> + '_;

    /// Deterministic fingerprint over the snapshot: blake2b_256 of
    /// the concatenated `(slot_be8 || hash || bytes)` triples in
    /// BTreeMap order. Two snapshots admitting the same blocks (in
    /// any admission order) have identical fingerprints.
    pub fn fingerprint(&self) -> Hash32;
}

/// Admit one AcceptedBlock into the served chain. Total, pure,
/// deterministic. Idempotent on byte-identity at the same key.
pub fn served_chain_admit(
    served: ServedChainSnapshot,
    block: AcceptedBlock,
) -> Result<ServedChainSnapshot, ServedChainAdmitError>;
```

### 2. Re-export from `crates/ade_ledger/src/producer/mod.rs`

```rust
pub mod served_chain;
pub use served_chain::{ServedChainSnapshot, ServedChainAdmitError, served_chain_admit};
```

### 3. CI gate `ci/ci_check_served_chain_closure.sh`

- Forbids `HashMap` / `HashSet` / `std::collections::Hash*` in
  `crates/ade_ledger/src/producer/served_chain.rs`.
- Forbids any new public constructor of `ServedChainSnapshot` outside
  the canonical `new()` + `served_chain_admit` admit path.

---

## §12 Mechanical Acceptance Criteria (named tests)

All tests in `crates/ade_ledger/src/producer/served_chain.rs` (or in
`producer/self_accept.rs` where corpus + valid AcceptedBlock
construction lives).

- `served_chain_admit_admits_corpus_block` — admit one corpus block;
  `snapshot.len() == 1`; `block_bytes` returns byte-identical slice.
- `served_chain_admit_idempotent_on_byte_identity` — admitting the
  same AcceptedBlock twice yields equal snapshots (PartialEq) and
  equal fingerprints.
- `served_chain_admit_independent_of_order` — admitting blocks
  `[a, b, c]` vs `[c, b, a]` yields equal fingerprints (BTreeMap
  order canonical).
- `served_chain_snapshot_iteration_is_btreemap_ordered` — `iter()`
  visits entries in `(SlotNo, Hash32)`-sorted order.
- `served_chain_block_bytes_accessor_returns_accepted_block_slice` —
  for every admitted block, `block_bytes(slot, &hash)` returns a
  slice byte-identical to `AcceptedBlock::as_bytes()`.
- `served_chain_range_bytes_returns_inclusive_window` — admit three
  blocks at distinct slots, query a range containing two of them,
  receive exactly those two in BTreeMap order.
- `served_chain_fingerprint_replay_byte_identical` — admit the same
  blocks in two different orders; assert equal fingerprints
  (replay-equivalence over arbitrary admission order).

CI: `ci/ci_check_served_chain_closure.sh` (new).

---

## §14 Hard Prohibitions

- No `HashMap` / `HashSet` / `std::collections::Hash*` in
  `served_chain.rs`.
- No public field on `ServedChainSnapshot`; only the methods listed.
- No `pub fn` constructing `ServedChainSnapshot` outside `new()` +
  the admit path.
- No mutation of an admitted `AcceptedBlock`'s bytes after admission
  (impossible by ownership; AcceptedBlock is moved into the map).
- No `impl Serialize for ServedChainSnapshot` that exposes internal
  iteration order outside BTreeMap canonical order.

---

## §15 Explicit Non-Goals

- Chain-sync server reducer (S3).
- Block-fetch server reducer (S4) — uses this type but is its own
  slice.
- GREEN adapter / RED orchestrator (S5/S6).
- Persistence across restarts — narrow scope: in-memory only;
  ChainDB bridge is N-D-bridge cluster scope.
- Eviction — narrow scope: snapshot only ever grows during a session.

---

## Replay obligations

The snapshot fingerprint surface introduced here is the canonical
admission-order-independent replay anchor for S5's session-transcript
replay corpus.

---

## Authority reminder

This document is a planning aid. If it ever conflicts with the
project's normative specifications or the invariant registry, the
normative documents win.
