# Invariant Slice — PHASE4-N-G S1

## Slice Header

**Slice Name:** Header projection authority (`accepted_block_header_bytes`) + closed `ServerReply` wrappers (compile-time server-agency closure)
**Cluster:** PHASE4-N-G
**Status:** In Progress
**CEs addressed:** CE-N-G-1
**Registry flips on merge:** `CN-PROTO-06` → `enforced`
**Dependencies:** PHASE4-N-C closed (provides `AcceptedBlock`); PHASE4-N-A closed (provides `ChainSyncMessage` / `BlockFetchMessage` + `Tip` / `Point` / `Range` types)

---

## Intent

Make it impossible for the producer-side server pump to (a) split a Cardano
block envelope's header/body bytes through a code path other than the
existing validator-shared body-hash recipe (`DC-CONS-16`), and (b)
construct any outgoing chain-sync or block-fetch message tagged with
client agency. (a) is enforced by exposing a single public accessor
`accepted_block_header_bytes(&AcceptedBlock) -> Result<&[u8], _>` that
reuses the existing `header_cbor_slice` walker, plus a CI grep gate
that rejects any new header/body splitter. (b) is enforced at the
type level: two closed enum wrappers (`chain_sync::server::ServerReply`,
`block_fetch::server::ServerReply`) carry only server-agency variants;
their inner enum field is private; their constructors only exist for
server-agency variants; the corresponding wire `ChainSyncMessage` /
`BlockFetchMessage` is reachable only through `into_message()`.

---

## The change

### 1. New public function `accepted_block_header_bytes`

Added in `crates/ade_ledger/src/block_validity/header_input.rs`:

```rust
/// Return the header CBOR sub-slice of an `AcceptedBlock` — the same
/// bytes the validator's body-hash recipe hashes via
/// `header_cbor_slice`. Reuses the existing canonical walker; this is
/// the single header-projection authority for the producer-side
/// server pump (PHASE4-N-G, DC-CONS-18). No parallel splitter.
pub fn accepted_block_header_bytes(
    accepted: &crate::producer::AcceptedBlock,
) -> Result<&[u8], BlockValidityError> {
    let bytes = accepted.as_bytes();
    let env = decode_block_envelope(bytes).map_err(codec)?;
    let inner = &bytes[env.block_start..env.block_end];
    let header = header_cbor_slice(inner)?;
    // Project back into the full-envelope coordinate space.
    let offset_in_bytes = (header.as_ptr() as usize)
        .checked_sub(bytes.as_ptr() as usize)
        .ok_or_else(|| /* impossible: header is &inner is &bytes */ panic!_dead_code)?;
    Ok(&bytes[offset_in_bytes..offset_in_bytes + header.len()])
}
```

(See actual implementation — the offset arithmetic is done safely
via slice positions, not pointer casts, in the real code.)

### 2. New module `crates/ade_network/src/chain_sync/server/mod.rs`

```rust
//! BLUE producer-side chain-sync server-role surface (PHASE4-N-G S1).
//!
//! Closed type wrapper: only Server-agency-legal variants of
//! `ChainSyncMessage` are constructible. No public constructor exists
//! for `RequestNext`, `FindIntersect`, or `Done` on `ServerReply` —
//! attempting to call one is a compile error. The inner enum field is
//! private; the only path from `ServerReply` to wire bytes is via
//! `into_message()` (which returns a `ChainSyncMessage`) followed by
//! the existing codec.

use crate::codec::chain_sync::{ChainSyncMessage, Point, Tip};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerReply(ServerVariant);

#[derive(Debug, Clone, PartialEq, Eq)]
enum ServerVariant {
    RollForward { header: Vec<u8>, tip: Tip },
    RollBackward { point: Point, tip: Tip },
    AwaitReply,
    IntersectFound { point: Point, tip: Tip },
    IntersectNotFound { tip: Tip },
}

impl ServerReply {
    pub fn roll_forward(header: Vec<u8>, tip: Tip) -> Self { ... }
    pub fn roll_backward(point: Point, tip: Tip) -> Self { ... }
    pub fn await_reply() -> Self { ... }
    pub fn intersect_found(point: Point, tip: Tip) -> Self { ... }
    pub fn intersect_not_found(tip: Tip) -> Self { ... }

    pub fn into_message(self) -> ChainSyncMessage { ... }
}
```

### 3. New module `crates/ade_network/src/block_fetch/server/mod.rs`

```rust
//! BLUE producer-side block-fetch server-role surface (PHASE4-N-G S1).

use crate::codec::block_fetch::BlockFetchMessage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerReply(ServerVariant);

#[derive(Debug, Clone, PartialEq, Eq)]
enum ServerVariant {
    StartBatch,
    NoBlocks,
    Block { bytes: Vec<u8> },
    BatchDone,
}

impl ServerReply {
    pub fn start_batch() -> Self { ... }
    pub fn no_blocks() -> Self { ... }
    pub fn block(bytes: Vec<u8>) -> Self { ... }
    pub fn batch_done() -> Self { ... }

    pub fn into_message(self) -> BlockFetchMessage { ... }
}
```

### 4. CI gate `ci/ci_check_no_parallel_header_splitter.sh`

Greps the repo for any new header/body splitter outside the canonical
authority. The canonical site is
`crates/ade_ledger/src/block_validity/header_input.rs::header_cbor_slice`
+ the new public `accepted_block_header_bytes` accessor; no other
`pub fn` may return a header-byte sub-slice extracted by walking the
block envelope.

---

## §12 Mechanical Acceptance Criteria (named tests)

All tests in `crates/ade_ledger/src/block_validity/header_input.rs`
and `crates/ade_network/src/{chain_sync,block_fetch}/server/mod.rs`.

- `accepted_block_header_bytes_equals_validator_split_on_corpus` —
  for each block in the Conway-576 corpus, the accessor returns the
  identical byte slice as `header_cbor_slice` on the decoded inner
  block, lifted into envelope coordinates.
- `accepted_block_header_bytes_is_subslice_of_as_bytes` — the
  returned slice is a contiguous subslice of `accepted.as_bytes()`
  (same start pointer + length within bounds).
- `accepted_block_header_bytes_rejects_malformed_envelope` —
  attempting to call on a corrupted (non-envelope) byte stream
  returns a structured `BlockValidityError`, not a panic.
- `chain_sync_server_reply_round_trips_through_codec` — every
  `ServerReply` constructor produces a value whose `into_message()`,
  when encoded by the existing codec, round-trips byte-identically.
- `chain_sync_server_reply_into_message_only_yields_server_variants`
  — `into_message()` outputs `RollForward` / `RollBackward` /
  `AwaitReply` / `IntersectFound` / `IntersectNotFound` only;
  exhaustive match in the test asserts no other arm is reachable.
- `block_fetch_server_reply_round_trips_through_codec` — same for
  block-fetch.
- `block_fetch_server_reply_into_message_only_yields_server_variants`
  — `into_message()` outputs `StartBatch` / `NoBlocks` / `Block` /
  `BatchDone` only.

CI scripts:
- `ci/ci_check_no_parallel_header_splitter.sh` (new) — grep gate
  forbidding new `pub fn` returning header/body byte sub-slices
  outside the canonical authority.

(Compile-fail tests via `trybuild` are deliberately omitted; the
type-level enforcement is achieved by the private inner-enum field
plus the absence of client-variant constructors. There is no public
API surface to construct a `ServerReply::request_next()`, so any
attempt to call one is a compile error against undefined items.)

---

## §14 Hard Prohibitions

- No new `pub fn .*split.*header` or `pub fn .*header_bytes` outside
  `ade_ledger::block_validity::header_input` (CI gate).
- No public constructor of `ServerReply` for `RequestNext`,
  `FindIntersect`, `Done` (chain-sync) or `RequestRange`, `ClientDone`
  (block-fetch). Enforced at the type level.
- No `pub use ServerVariant` — the inner enum stays private.
- No `impl From<ChainSyncMessage> for ServerReply` — wire bytes can
  only be assembled by constructing `ServerReply` first, never by
  wrapping an arbitrary `ChainSyncMessage`.
- No re-export of the existing `chain_sync::transition::*` or
  `block_fetch::transition::*` from the new `server` modules — those
  remain the BLUE state-machine grammar gate; the server modules
  carry the closed reply types only (until S3/S4 add the reducers).
- No commit without the Ade trailer.

---

## §15 Explicit Non-Goals

- The chain-sync server reducer (`producer_chain_sync_serve`) — S3.
- The block-fetch server reducer (`producer_block_fetch_serve`) — S4.
- The `ServedChainSnapshot` type — S2.
- The GREEN adapter — S5.
- The RED orchestrator — S6.
- N2C `local-chain-sync` / `local-tx-submission` server surface — out
  of scope for this cluster.

---

## Replay obligations

None added by this slice. The header-projection accessor reuses the
existing recipe; the new wrappers carry no state. Replay-equivalence
surfaces are introduced in S5.

---

## Authority reminder

This document is a planning aid. If it ever conflicts with the
project's normative specifications or the invariant registry, the
normative documents win.
