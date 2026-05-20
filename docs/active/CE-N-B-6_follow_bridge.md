# CE-N-B-6 follow-mode bridge — bounded smoke slice

> **Status**: bounded release slice (NOT a project arc). One question only:
> *Can Ade consume a real Haskell peer's ChainSync stream and hold the same
> selected tip under roll-forward and rollback?*
> Once yes, move on to the validation + block-production path.

## Why this is bounded

Live tip-following does **not** win the bounty. The bounty needs ledger
validity agreement, block production, full N2N/N2C, sync, recovery, and
Haskell agreement on block/tx/tip. This slice is a *smoke test for the
consensus/network bridge* and a producer of **reusable artifacts** that feed
the real validation work (B). It must not grow into a follower product.

Explicit non-goals (do NOT build): observability dashboard, multi-peer
networking, long-running production follower, broad relay compatibility,
partial *fake* validation that pretends to be ledger validation.

## What exists (reuse, don't rebuild)

- `crates/ade_network/src/bin/capture_chain_sync.rs` — working N2N handshake +
  ChainSync `FindIntersect → RequestNext` loop against a live relay (preprod
  default, magic 1). Lift this session pattern.
- `ade_codec::shelley::block` — decodes the Praos/TPraos header body:
  `block_number`, `slot`, `prev_hash`, `issuer_vkey`, `vrf_vkey`, `vrf`
  (Split for TPraos array(15) / Combined for Praos array(10)), `body_hash`,
  `operational_cert { sequence_number, kes_period, .. }`, `protocol_version`.
- `ade_core::consensus::{candidate, fork_choice, rollback, events}` — BLUE
  `select_best_chain`, `apply_rollback`, `TiebreakerView`, `Point`,
  `ChainSelectorState`, `ChainEvent`.
- Real preprod ChainSync captures: `corpus/network/n2n/chain_sync/`.
- Real block corpus (headers extractable): `corpus/boundary_blocks/…`.

## Design — follow mode is RED, peer-trusted, NO LedgerView

Follow mode runs **fork-choice + rollback only**, NOT `validate_and_apply_header`.
It trusts the peer's validation and checks *selection agreement* — it does not
verify VRF/leader/nonce (that needs ledger stake state = workstream B). This
bypass lives entirely in RED `ade_core_interop`; BLUE is untouched. Document
this honestly: "follow mode asserts tip-selection agreement with a peer that
has already validated; it is not block validation."

### Header → selection projection (RED helper)

For each ChainSync `RollForward { header_bytes }`:
1. Decode the wrapped header (N2N RollForward wraps the header; reuse whatever
   N-A's S-A4 chain-sync path uses to reach the era-tagged header CBOR).
2. Decode header body via `ade_codec::shelley::block`.
3. Project to fork-choice inputs:
   - `block_no = body.block_number`
   - `slot = body.slot`
   - `point.hash = blake2b256(header_cbor)` (the header hash = block hash)
   - `issuer_pool = blake2b224(body.issuer_vkey)` (Hash28)
   - `op_cert_counter = body.operational_cert.sequence_number`
   - `vrf_output_first_8` = first 8 bytes of the leader VRF output
     (`VrfData::Combined.vrf_result` for Praos; `VrfData::Split.leader_vrf`
     for TPraos)
4. Build a single-header `CandidateFragment` anchored at the current tip and
   call `select_best_chain`. Record the emitted `ChainEvent`.

For `RollBackward { point, tip }`: call `apply_rollback` with depth derived
from the follow state's recent points; record the event.

### Peer-tip agreement

ChainSync `Tip` carries `(Point { slot, hash }, block_no)`. After Ade is
caught up to the peer's tip (the last RollForward point == peer Tip point),
assert `Ade.current_tip == peer_tip`. Track a `disagreements` counter; any
disagreement is a hard failure.

## Deliverables (bounded)

1. **`ade_core_interop::follow`** (RED) — `FollowState`, `ingest_rollforward`,
   `ingest_rollbackward`, `agreement_status`. Pure-ish over decoded inputs;
   the socket lives in the binary.
2. **Offline replay test** (CI, NOT `#[ignore]`) — drive the follow bridge
   from **real header bytes already in the repo** (extract headers from
   `corpus/boundary_blocks/…` and/or decode `corpus/network/n2n/chain_sync/`
   frames). Assert: headers selected in block-number order; a synthetic
   rollback within-k rolls back correctly; tip tracking matches the input
   sequence. Deterministic, no network.
3. **Decoded-header corpus artifact** — emit the projected per-header fields
   (block_no, slot, hash, issuer_pool, op_cert_counter, vrf_output_first_8) as
   a small committed JSON corpus under `corpus/consensus/follow/`. Reusable by
   workstream B.
4. **Live driver** — extend `crates/ade_core_interop/src/bin/live_consensus_session.rs`
   to: handshake + ChainSync loop (reuse capture_chain_sync), intersect at the
   peer's current tip (so headers are current-era / Praos), follow forward,
   run the follow bridge, compare to peer Tip, write a transcript +
   peer-tip-comparison log. Flags: `--network preprod|mainnet` (default
   preprod), `--max-headers N` (default 1000). The `#[ignore]` test asserts the
   binary builds and starts; the sustained run is the operator evidence pass →
   `docs/clusters/PHASE4-N-B/CE-N-B-6_<date>.log`.

## Acceptance (this slice)

- [ ] `cargo build -p ade_core_interop` clean.
- [ ] `cargo test -p ade_core_interop` — offline replay test passes
      (deterministic, no network), `#[ignore]` live test present.
- [ ] `cargo clippy -p ade_core_interop --all-targets -- -D warnings` clean.
- [ ] Decoded-header corpus committed under `corpus/consensus/follow/`.
- [ ] No fake validation; follow mode documented as RED, peer-trusted,
      selection-only.
- [ ] BLUE `ade_core::consensus` unchanged (verify the 4 consensus CI scripts
      still pass).

## After this slice

Pivot to workstream B per [[project-bounty-requirements]]: real `LedgerView`
(stake snapshot + epoch nonce), VRF/leader validation on live headers, then tx
validity + block production. The artifacts above (decoded-header corpus, follow
bridge, peer-tip comparison) plug directly into that.
