# Invariant Slice — PHASE4-N-U S3: serve-as-durable-chain projection

## §2 Slice Header
- **Slice Name:** serve-as-durable-chain projection
- **Cluster:** PHASE4-N-U (forged-block durability) — primary invariant **DC-NODE-12**; S3 enforces **DC-NODE-13** and restates the serve-provenance clause of **CN-CONS-07**.
- **Status:** done — S3 complete + merged (`a49563bc` doc + CN-CONS-07 restatement, `8e0dbe99` impl); DC-NODE-13 → enforced; CN-CONS-07 + DC-NODE-11 strengthened (`strengthened_in += PHASE4-N-U`)
- **Cluster Exit Criteria Addressed:** **CE-7** (DC-NODE-13 — the `--mode node` served view is a deterministic projection of the durable ChainDb; a follower fetches coherent durable history A→B incl. a feed-ingested predecessor; the G-R monotone-serve-gate accumulator workaround is retired). *(CE-1…CE-4 = S1; CE-5/6 = S2.)*

## §3 Dependencies
- **S1** (`admit_forged_block_durably` → `pump_block` — DC-NODE-12): produces the durable ChainDb the projection serves.
- **S2** (`warm_start_recovery` forward-replay — T-REC-05): the durable ChainDb the projection reads survives restart (the accumulator did not — the bug this slice closes).
- Cluster entry conditions: N-G BLUE serve reducers (`producer_chain_sync_serve`, `producer_block_fetch_serve`) + their `ServedHeaderLookup` / `ServedRangeLookup` trait seams; N-F-G-H shared serve-dispatch authority (`dispatch_server_frame_event_to_outbound`, DC-NODE-07); N-D ChainDb (`iter_from_slot` / `get_block_by_hash` / `get_block_by_slot` / `tip`).

## §4 Intent (invariant impact)
Introduce **DC-NODE-13**: the `--mode node` served view (ChainSync header advertisement + BlockFetch body) is a deterministic **projection of the durable adopted chain** — the ChainDb whose sole production writers are the validated durable admit (`pump_block`, DC-NODE-12/S1) and the validated warm-start/genesis replay (`bootstrap_initial_state`) — **not** an independent in-memory accumulator.

This **restates the serve-provenance clause of CN-CONS-07** from "every served byte originated as an in-memory `AcceptedBlock` token" (the N-G S2 effective rule) to "every served byte is a projection of the durable ChainDb, whose contents entered ONLY through the single validated durable admit chokepoint." The TRUE CN-CONS-07 invariant — *no unvalidated bytes leave the node* — is **preserved and strengthened** (the durable ChainDb holds only `block_validity`-cleared bytes, whether forged via `self_accept` or received via `admit_via_block_validity`, so the one durable-serve seam covers both gates). The accidental implementation dependency — *hold a live `AcceptedBlock` token in memory*, gone after restart — is removed, so a follower fetches **coherent history A→B** (the durable chain says A→B; the served view serves A and B, never B without A) and serving survives restart. **Supersedes** the PHASE4-N-F-G-R monotone serve-gate workaround (`serve_gate_admits` gated an accumulator; the extend-only durable chain holds exactly one block 0 by construction — DC-CONS-23 — so the projection serves a stable block 0 without a gate).

## §5 Scope / What is built
- **NEW RED projection adapter** `ChainDbServedSource` in `ade_runtime` (new module `network::served_chain_projection`) wrapping `&dyn ChainDb`, implementing both BLUE seams:
  - `ServedHeaderLookup::{next_after, intersect, tip}` — `next_after(cursor)` reads `iter_from_slot(cursor.slot)` (or from 0), skips keys `≤ cursor`, takes the first durable block, `decode_block` for `block_no`/`era`, projects the header via `block_header_bytes` (below); `intersect(points)` matches each `Point::Block{slot,hash}` against `get_block_by_hash`; `tip()` reads `ChainDb::tip` then `decode_block` for `block_no`.
  - `ServedRangeLookup::range_bytes(from, to)` — collects durable blocks whose `(slot, hash)` key lies in `[from, to]` (tuple-lexicographic, ascending), serving `stored.bytes` **verbatim** (no re-encode). Replicates the `ServedChainSnapshot::range_bytes` BTreeMap-range semantics over the linear durable chain; the reducer's existing both-endpoints-present check (CN-SNAPSHOT-02) is unchanged.
  - Read-only; on a `ChainDbError` a lookup yields `None`/empty (serve nothing this round — availability, never wrong/partial bytes).
- **NEW `ServedChainSource` enum** in `network::serve_dispatch`: `Snapshot(&ServedChainView)` (produce-mode / migration) | `DurableChainDb(&dyn ChainDb)` (`--mode node`). `dispatch_server_frame_event_to_outbound` reads from it — **still exactly one serve-dispatch definition** (DC-NODE-07): the `Snapshot` arm scopes the `watch::Ref` exactly as today; the `DurableChainDb` arm builds a `ChainDbServedSource`. `--mode produce` passes `Snapshot`; `--mode node` passes `DurableChainDb`.
- **BLUE reuse-exposure (not a new authority):** factor the existing body of `accepted_block_header_bytes` into `block_header_bytes(block_cbor: &[u8]) -> Result<&[u8], BlockValidityError>` in `block_validity::header_input`; `accepted_block_header_bytes(accepted)` becomes `block_header_bytes(accepted.as_bytes())`. Same `header_cbor_slice` recipe (the single DC-CONS-18 projection authority) — no new logic, no new type, **no parallel splitter** in `ade_runtime`.
- **`ade_node::node_lifecycle` wiring:** `chaindb` becomes `Arc<PersistentChainDb>` (shared with the spawned serve task; redb reads are MVCC — safe concurrent with the relay loop's writes). `run_node_serve_task` takes `Arc<PersistentChainDb>` and dispatches with `ServedChainSource::DurableChainDb(serve_chaindb.as_ref())`. **RETIRE** the node-path push sibling (`handoff_tx`/`handoff_rx`, `serve_handle`/`serve_view`, the `tokio::spawn` push task), the ForgeTick `tx.send(h)` serve forward, and `serve_gate_admits` (+ its in-module test).
- **RETIRE** the G-R gate `ci/ci_check_served_chain_stability.sh` (its DC-NODE-11 mechanism is superseded). **NEW gate** `ci/ci_check_served_chain_projection.sh` (CE-7).
- **Out of scope:** `--mode produce` serve (keeps the `Snapshot` accumulator source — no durable admit path; produce's accumulator is still fed only by `self_accept`'d `AcceptedBlock`s, satisfying the original CN-CONS-07 token-proof); the legacy `n2n_server::dispatch_*_frame` per-peer fns (produce/legacy, untouched); `ServedChainHandle`/`ServedChainView`/`push_atomic` (retained — produce uses them).

## §6 Execution Boundary (TCB color)
- **RED (new/changed):** `ade_runtime::network::served_chain_projection` (`ChainDbServedSource` — new), `ade_runtime::network::serve_dispatch` (`ServedChainSource` enum + source-read), `ade_node::node_lifecycle` (`Arc<PersistentChainDb>`, serve-task wiring, retirements).
- **RED (reused, unchanged):** `ade_runtime::chaindb` (reads only), `ade_node::node_sync::admit_forged_block_durably`.
- **GREEN (reused, retained for produce):** `ade_runtime::producer::{served_chain_lookups (Snapshot source), served_chain_handle (ServedChainView/Handle)}`.
- **BLUE (reused; one minimal reuse-exposure — NOT new authority):** `ade_ledger::block_validity::{block_header_bytes (factored core of the existing DC-CONS-18 authority), accepted_block_header_bytes (now delegates), decode_block}`; the serve reducers `ade_network::{chain_sync,block_fetch}::server` (UNCHANGED — they read the new source through the same trait seams).
- **No new BLUE authority or canonical type.** The header-projection recipe (`header_cbor_slice`) is byte-identical; only its input signature is generalized from `&AcceptedBlock` to `&[u8]`.

## §7 Invariants Preserved
- **DC-NODE-07** (single serve-dispatch authority) — preserved: `dispatch_server_frame_event_to_outbound` stays the ONE definition; it now reads a `ServedChainSource` parameter (a read-source enum, not a second dispatch).
- **CN-CONS-07 TRUE invariant** (no unvalidated bytes leave the node) — preserved + strengthened (durable-provenance covers both the forged and received gates; see §8).
- **DC-CONS-17 / DC-CONS-18** (block-fetch byte-identity + single header-projection authority) — preserved: the projection serves `stored.bytes` verbatim and reuses the one `header_cbor_slice` recipe (no parallel splitter); `compose_blockfetch_block` / `compose_rollforward_header` tag-24 wrap (CN-WIRE-08) unchanged.
- **DC-NODE-12 / DC-CONS-23 / DC-WAL-04 / T-REC-05** — preserved: the durable admit path and warm-start are untouched; serve is read-only over the durable ChainDb.
- **DC-NODE-09** (serve outlives feed-end) — preserved: the serve task is still gated on the operator `shutdown` watch, awaited after the relay loop.
- **DC-SYNC-01 / DC-SYNC-02 / CN-NODE-02** — preserved: serve advances no tip, admits nothing; `pump_block` stays the sole durable tip authority.

## §8 Invariants Strengthened or Introduced
- **DC-NODE-13** declared → **enforced** (the slice's primary invariant; serve-as-projection of the durable chain; gate + tests below).
- **CN-CONS-07** — `strengthened_in += "PHASE4-N-U"`: the serve-provenance clause is restated from in-memory-token-proof to durable-provenance-proof, closing the durable-serve seam (a relay node serves its adopted chain — forged AND received — and every byte traces to the validated durable admit). The broadcast/admission gate clauses are unchanged.
- **DC-NODE-11** — `strengthened_in += "PHASE4-N-U"`, **mechanism superseded** (supersede-via-cross-ref, mirroring DC-NODE-05→DC-NODE-12 in S1): the monotone serve-gate (`serve_gate_admits` over an accumulator) is retired; its invariant — *a follower sees a stable, coherent served block 0, no block-0-replaces-block-0 churn* — is now provided more strongly by serve-as-projection of the extend-only durable chain (which holds exactly one block 0 by DC-CONS-23). Its `code_locus`/`tests`/`ci_script` migrate to the projection; `ci_check_served_chain_stability.sh` is replaced by `ci_check_served_chain_projection.sh`.

## §11 Replay / Crash / Epoch Validation
- **Replay (determinism):** the served frame sequence is a deterministic function of the committed durable chain — `iter_from_slot` is slot-ascending, the chain is linear (extend-only), `stored.bytes` are served verbatim. Same durable chain → byte-identical served headers + block payloads. Test: `served_view_projects_durable_chain`.
- **Crash recovery:** the durable ChainDb the projection reads is recovered by S2 (`warm_start_recovery`, T-REC-05); serving therefore survives restart — the precise gap the accumulator left. `follower_fetches_coherent_history_incl_ingested_predecessor` proves A→B coherence (never B without A) over a multi-block durable chain incl. a non-forged predecessor.
- **Epoch:** unchanged — serve is downstream of admit; no epoch transition logic here.

## §12 Mechanical Acceptance Criteria
- `cargo test -p ade_node` green incl. NEW: `served_view_projects_durable_chain`, `follower_fetches_coherent_history_incl_ingested_predecessor`, `served_view_retires_accumulator`.
- NEW `ci/ci_check_served_chain_projection.sh` green: (a) the node serve path reads the durable ChainDb projection (`ServedChainSource::DurableChainDb` / `ChainDbServedSource`), not the accumulator; (b) `serve_gate_admits` is **gone** from `node_lifecycle.rs`; (c) no `push_atomic` / `handoff_tx` serve forward remains in the `--mode node` arm; (d) the projection serves `stored.bytes` verbatim (no re-encode) and reuses `block_header_bytes` (single header authority — no parallel splitter / no envelope re-walk in `ade_runtime`); (e) DC-NODE-13 present-and-enforced.
- `ci/ci_check_single_serve_dispatch_authority.sh` green (still exactly one `dispatch_server_frame_event_to_outbound`).
- `ci/ci_check_served_chain_stability.sh` **removed** (G-R mechanism retired); DC-NODE-11 registry evidence updated to record the supersession.
- `ci/ci_check_node_serve_lifetime.sh` green (DC-NODE-09 preserved).
- S1/S2 gates (`ci_check_forged_durable_admit_via_pump.sh`, `ci_check_node_run_loop_containment.sh`, `ci_check_node_sync_via_pump.sh`) green (admit/loop bodies untouched).
- Registry: DC-NODE-13 declared → enforced; CN-CONS-07 + DC-NODE-11 `strengthened_in += "PHASE4-N-U"`.
- Relevant crate tests green; full `cargo test --workspace` is the cluster-close gate (timeouts reported honestly). The C1 genesis-rehearsal reproduction remains the release regression target (a follower must still adopt block 0 — now from the durable projection).

## §14 Hard Prohibitions
**Inherited (cluster §11):** no new BLUE authority/canonical type; no second durable tip-advance path; no admit-time fork-choice; no re-encode; no bypass of `self_accept`; no RO-LIVE flip; no Mithril/bootstrap change.
**Slice-specific (user boundaries):**
- No serving raw arbitrary ChainDb bytes — serve ONLY via the BLUE reducers reading the projection through the trait seams.
- No serving bytes that bypassed `pump_block` — the projection reads the durable ChainDb, whose sole production writers are `pump_block` (DC-NODE-12) + validated warm-start replay (`bootstrap_initial_state`).
- No second serve authority — one `dispatch_server_frame_event_to_outbound`; `ServedChainSource` is a read-source enum, not a parallel dispatch.
- No `AcceptedBlock` reconstruction hack — the projection reads raw `stored.bytes` + `block_header_bytes(&[u8])`; it never rebuilds an `AcceptedBlock` token.
- No re-validation shortcut as a substitute for provenance — provenance is structural (durable ChainDb ⇐ `pump_block` ⇐ `block_validity`); the serve path does NOT re-run `block_validity` to "prove" a byte is safe.

## §15 Explicit Non-Goals
Migrating `--mode produce` serve to the durable projection (produce has no durable admit; it keeps the `Snapshot` source — out of N-U scope). A proactive `advance_tip` / `serve_view.changed()` reactor (serve stays request-driven per DC-NODE-09). The legacy `n2n_server::dispatch_*_frame` per-peer functions. Any RO-LIVE flip, BA-02, or bounty acceptance claim (durability + coherent serve ≠ operator-witnessed peer acceptance — RO-LIVE-01 stays operator-gated).
