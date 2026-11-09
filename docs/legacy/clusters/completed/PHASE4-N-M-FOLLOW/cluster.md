# PHASE4-N-M-FOLLOW — Sustained admission via chain-sync follow (cluster doc)

> **Status:** Planning. Single-slice strengthening of
> PHASE4-N-M-SCHED's live operator pass. Extends the
> "1 BlockAdmitted + 1 Agreed" closure (committed at SCHED) to
> "N consecutive BlockAdmitted with zero divergence" by making
> the admission wire pump *follow* the peer's chain, not just
> admit the tip at intersect time.

**Predecessors:** PHASE4-N-M-A1.1 (full preprod seed import),
PHASE4-N-M-FRAG (per-mini-protocol reassembly + adjacent
fixes), PHASE4-N-M-SCHED (era-schedule epoch wiring + the
literal "≥1 BlockAdmitted + ≥1 Agreed" transcript).

**Successor:** none planned in N-M. Future clusters work on
ChainDb persistence of admitted blocks, multi-epoch admission,
and block production.

## §1 Primary invariant

> The admission wire pump walks the peer's chain in order
> starting from the intersect point. Each `RollForward` from
> chain-sync triggers a block-fetch for the rolled-forward
> block's exact point (slot + header hash). Block-fetch
> BatchDone gates the next chain-sync RequestNext. Across the
> stream, every admitted block's hash matches the hash the
> peer's chain-sync server announced for that slot, by
> construction — divergence at any block halts admission
> fail-closed.
>
> Additionally, the op-cert counter rule MUST permit the same
> op-cert counter across multiple blocks within a KES period
> (normal pool operation per the Cardano protocol). Only a
> strictly-less counter is a regression.

### Why this matters

The SCHED closure proved the **literal** DC-EVIDENCE-01
statement on a single block: ≥1 BlockAdmitted + ≥1 Agreed +
0 Diverged + 0 mismatched-hash BlockAdmitted. But that
transcript came from a shortcut — the pre-FOLLOW pump
block-fetched the peer's *current tip* on IntersectFound,
admitting one tip block and then idle-polling chain-sync
without further block-fetches.

For the bounty's tx/block-validity agreement claim
(`[[project-bounty-requirements]]`, `[[feedback-tx-validity-priority]]`),
sustained agreement across many real blocks matters more than
a single tip-admit. A single-block transcript can't surface
issues that only appear in repeated-pool scenarios (e.g., the
op-cert counter equal-reuse case FOLLOW exposed). 34 blocks
admitted in order with zero divergence is a much stronger
fail-closed proof.

## §2 Scope

### In scope

- `crates/ade_runtime/src/admission/wire_pump.rs`:
  - New `extract_chain_sync_header_point(envelope_bytes)`
    helper. Decodes the
    `[serialisationInfo, tag(24, bytes(header_cbor))]`
    envelope, hashes the inner header_cbor with Blake2b-256
    for `block_hash`, parses the header_body's 2nd field for
    `slot`, returns `Point::Block { slot, hash }`. Babbage/
    Conway Praos shape only (one-element honest scope).
  - `RollForward` handler: replace "emit TipUpdate + queue
    chain-sync RequestNext" with "emit TipUpdate + extract
    header point + queue block-fetch RequestRange". Sequenced
    so chain-sync RequestNext only queues on block-fetch
    `BatchDone`.
  - `IntersectFound` handler: drop the initial-tip block-fetch
    (it caused `SlotBeforeLastApplied` rejections when
    chain-sync subsequently rolled forward from `intersect+1`).
    Emit TipUpdate + queue RequestNext only; the first real
    block-fetch fires on the first RollForward.

- `crates/ade_core/src/consensus/praos_state.rs`:
  - `OpCertCounterMap::upsert_strict`: fix the regression
    rule. Per the Cardano protocol, the op-cert counter is
    monotonically NON-decreasing within a KES period (equal
    is allowed — same op-cert re-used). Strict-less is the
    only regression. The old code rejected equal counters,
    which over-rejected legitimate repeated op-certs in real
    pool operation (masked pre-FOLLOW because the positive
    Conway corpus covers 14 different pools each appearing
    once).

- `crates/ade_core/src/consensus/header_validate.rs`:
  - Step 4 op-cert counter pre-check: change `<= existing`
    rejection to `< existing` rejection. Matches the
    `OpCertCounterMap::upsert_strict` semantics.

### Out of scope (explicit)

- Catching up to the live tip (would require running long
  enough for our admit rate to overtake peer's block-
  production rate — operator-time, not slice scope).
- Modifying the GREEN verdict reducer to emit `agreed` when
  the admitted block came from the peer's chain-sync stream
  (a richer reducer is a future cluster — the current reducer
  only emits `agreed` when our slot equals peer's tip slot,
  by design).
- ChainDb persistence of admitted blocks across runs.
- Rollback handling (replaying a rollback against
  already-admitted blocks is a future cluster).
- Block production.

## §3 Slice index

| Slice | Purpose | New rules / strengthenings |
|---|---|---|
| **F1** | Chain-sync follow + RollForward → block-fetch + op-cert counter fix | strengthens CN-PUMP-01, DC-PUMP-01, DC-PUMP-02, DC-EVIDENCE-01, RO-LIVE-05, DC-EVIDENCE-02 |

## §4 Exit criteria (cluster-level MACs)

1. `extract_chain_sync_header_point` exists, is the SOLE
   header-point extractor, and has unit tests covering the
   happy path + malformed-envelope error paths.
2. `pump_emits_tip_update_and_request_next_on_intersect_found_no_block_fetch`
   asserts that IntersectFound NO LONGER triggers an immediate
   block-fetch — only TipUpdate + chain-sync RequestNext.
3. `rollforward_drives_block_fetch_then_request_next` asserts
   the new sequencing: RollForward → TipUpdate → block-fetch;
   chain-sync RequestNext only after block-fetch BatchDone.
4. `OpCertCounterMap::upsert_strict` accepts equal counter as
   a no-op; `apply_op_cert_accepts_equal_counter_as_noop`
   replaces the old `_rejects_equal_counter` test.
5. Live operator pass against fully-synced docker preprod
   produces ≥ 3 consecutive `block_admitted` events with
   monotonically increasing slots, all with `our_hash` matching
   the peer's announced hash for the same slot, 0 `diverged`,
   0 mismatched-hash `block_admitted`.
6. Captured transcript committed at
   `docs/evidence/phase4-n-m-follow-sustained-transcript.jsonl`.
7. `cargo test --workspace` clean.
8. Commit + push with the project-override trailer.

## §5 Hard prohibitions

- No silent skip on malformed RollForward header bytes
  (fail-closed via `AdmissionWirePumpError::ChainSyncDecode`).
- No I/O / eprintln in BLUE (`ade_core`, `ade_ledger`,
  `ade_codec`, `ade_crypto`). RED diagnostics in
  `wire_pump::finalize` + `process_block` are the operator-
  facing surface.
- No silent partial state update on op-cert counter — the
  upsert semantics MUST be: strict-less = error, equal =
  no-op, greater = update.
- No re-introducing the IntersectFound tip-jump (it caused
  the SlotBeforeLastApplied issue this slice eliminates).

## §6 Replay obligations preserved

- T-DET-01 — unchanged; FOLLOW only changes wire-pump
  sequencing + op-cert rule semantics. The reducer remains
  pure.
- DC-EVIDENCE-01 — already `enforced` by SCHED (committed
  transcript with ≥1 Agreed). FOLLOW strengthens by
  demonstrating 34 sustained admits with zero divergence
  against the same peer.
- DC-EVIDENCE-02 (adversarial false-accept corpus) —
  strengthened: the FOLLOW transcript widens the live no-
  divergence surface from 1 block to 34 consecutive blocks.
  The synthetic 4-mutation-class corpus stays load-bearing
  for the explicit fail-closed claim; FOLLOW adds the
  natural-stream complement.

## §7 References

- Predecessor closures: `d8feabb` (SCHED), `4d3dc98` (FRAG),
  `03d1d24` (A1.1), `8843e20` (N-M-C).
- Operator-pass README §10 (SCHED closure) + §11 (this
  cluster, added at close).
- Doctrine: [[feedback-tx-validity-priority]],
  [[feedback-evidence-reducers-are-green-not-authority]],
  [[feedback-shell-must-not-overstate-semantic-truth]].
