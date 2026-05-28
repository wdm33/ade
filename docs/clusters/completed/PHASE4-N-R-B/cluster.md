# PHASE4-N-R-B — Served snapshot + per-peer dispatch (cluster doc)

> **Status:** Planning. 4-slice sub-cluster closing the
> `BroadcastBlock → push_atomic → per-peer block-fetch serve`
> path. Wraps the pure-value `ServedChainSnapshot` with a RED
> `watch::channel`-backed handle; replaces the `_ => {}` arm
> in `produce_mode::handle_listener_event`; fixes the
> partial-overlap `RequestRange` non-compliance found in
> N-R-A A1 OQ8.
>
> **Predecessor:** PHASE4-N-R-A (HEAD `56657df`).
> **Successor:** PHASE4-N-R-C (bounty artifact run path).
> **Inputs:** [`docs/planning/phase4-n-r-invariants.md`](../../planning/phase4-n-r-invariants.md)
> + [`docs/planning/phase4-n-r-cluster-slice-plan.md`](../../planning/phase4-n-r-cluster-slice-plan.md).

## §1 Primary invariant

> A forged block becomes visible to peers only after
> `ServedChainHandle::push_atomic(artifact)` succeeds.
> Per-peer dispatch in `produce_mode::handle_listener_event`
> routes every `PeerN2nServerChainSyncFrame` /
> `PeerN2nServerBlockFetchFrame` event through
> `n2n_server::dispatch_chain_sync_frame` /
> `dispatch_block_fetch_frame` — no event is absorbed by a
> catch-all arm. `RequestRange` covering unknown or
> partially-unavailable slots returns `NoBlocks` per the
> Cardano block-fetch protocol; no partial ad-hoc response.

## §2 Design decisions (locked per DQ-B1 / DQ-B2)

- **Shared handle:** `tokio::sync::watch::channel<ServedChainSnapshot>`
  wrapped in a RED `ServedChainHandle` type.
- **`push_atomic` API:** `push_atomic(artifact: ForgedBlockArtifact)
  -> Result<ServedTip, PushError>` where `ServedTip { slot,
  hash }` is closed.
- **`read_snapshot` API:** per-peer tasks call
  `handle.borrow()` → `Ref<'_, ServedChainSnapshot>`; deref
  to `&ServedChainSnapshot` for dispatch.
- **N4 fix:** `producer_block_fetch_serve` updated to require
  both endpoints present + contiguous range coverage before
  issuing `StartBatch + Block* + BatchDone`. Otherwise
  `NoBlocks` (matches Haskell `ouroboros-network`).

## §3 Slice index

| Slice | Purpose | Closes (invariant IDs) |
|---|---|---|
| **B1** | Planning + 3 registry entries declared (`CN-SNAPSHOT-01`, `CN-SNAPSHOT-02`, `DC-SNAPSHOT-01`). | — |
| **B2** | `ServedChainHandle` (RED) + `push_atomic` + `BroadcastBlock` effect handler. | I4, N11, D6 |
| **B3** | Per-peer dispatch wiring in `produce_mode::handle_listener_event`. | I5, N5 |
| **B4** | OQ8 partial-overlap fix in `producer_block_fetch_serve` + integration tests + sub-cluster close. | N4, R2; sub-cluster close. Flips registry entries to `enforced`; carry-forward strengthenings for `CN-PROD-01`, `DC-CONS-17`. |

## §4 Exit criteria

- [ ] CE-1: `ServedChainHandle` + `push_atomic` API land in
  `ade_runtime::producer`; tests cover atomic semantics +
  `ServedTip` return.
- [ ] CE-2: `produce_mode::apply_effects_with_forge_handler`
  calls `push_atomic` from the `BroadcastBlock` arm;
  fail-closed shutdown on `PushError`.
- [ ] CE-3: `produce_mode::handle_listener_event` routes
  `PeerN2nServerChainSyncFrame` / `PeerN2nServerBlockFetchFrame`
  events into `n2n_server::dispatch_*`. No `_ => {}` arm
  remains for those variants.
- [ ] CE-4: `producer_block_fetch_serve` rejects partial-overlap
  `RequestRange` with `NoBlocks`.
- [ ] CE-5: Integration tests cover: synthetic dialer fetches a
  pushed block byte-identical; partial-overlap range → `NoBlocks`;
  no-torn-snapshot during concurrent read + push.
- [ ] CE-6: Registry: `CN-SNAPSHOT-01`, `CN-SNAPSHOT-02`,
  `DC-SNAPSHOT-01` flipped to `enforced` at B4 close.
- [ ] CE-7: `CN-PROD-01.open_obligation` (per-peer dispatch
  closure) cleared.
- [ ] CE-8: `DC-CONS-17.strengthened_in += "PHASE4-N-R-B"`.
- [ ] CE-9: `cargo test --workspace --lib` clean.

## §5 References

- Predecessor close: [[project-phase4-n-r-a-closed]].
- Planning: [`../../planning/phase4-n-r-cluster-slice-plan.md`](../../planning/phase4-n-r-cluster-slice-plan.md).
- N-R-A A1 OQ8 audit: [`../PHASE4-N-R-A/S1.md`](../PHASE4-N-R-A/S1.md) §2.OQ8.
- N-R-A A1 OQ9 audit: same §2.OQ9 (dispatch signatures).
