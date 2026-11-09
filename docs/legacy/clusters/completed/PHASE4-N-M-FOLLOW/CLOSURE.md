# PHASE4-N-M-FOLLOW — Closure record

**Closed:** 2026-05-27
**Closure HEAD:** (set on commit)
**Predecessor HEAD:** `d8feabb` (PHASE4-N-M-SCHED — single-block agreed transcript).

## Goal

Extend admission from "1 BlockAdmitted + 1 Agreed at the
peer's tip" (SCHED) to "N consecutive BlockAdmitted in stream
order with zero divergence" (FOLLOW). Surface and close any
real-pool consensus rules that the single-block scope masked.

## Outcome

**Sustained admission proven.** 34 consecutive blocks admitted
in slot order with zero divergence against the local docker
`cardano-node-preprod`. Plus a real consensus bug fixed
inline: the op-cert counter rule over-rejected equal counters,
which is normal pool operation under the Cardano protocol.

| Surface | Pre-FOLLOW | Post-FOLLOW |
|---|---|---|
| BlockAdmitted in one pass | 1 (SCHED) | **34** |
| Slot range admitted | 1 slot | **124137045 → 124137868** (823 slots = ~14 min of mainnet chain time) |
| Diverged | 0 | **0** |
| Mismatched-hash BlockAdmitted | 0 | **0** |
| Agreed | 1 (SCHED) | 0 (FOLLOW; SCHED's 1 still stands) |
| Lagging | 0 | 34 (peer's tip is ahead — by design we walk from intersect) |

## What shipped

### BLUE / GREEN code

1. **Wire pump chain-sync follow** (`crates/ade_runtime/src/admission/wire_pump.rs`):
   - `extract_chain_sync_header_point(envelope)`: SOLE
     header-point extractor. Parses
     `[serialisationInfo, tag(24, bytes(header_cbor))]`,
     hashes inner CBOR with Blake2b-256 → `block_hash`,
     reads `header_body[1]` for `slot`, returns
     `Point::Block { slot, hash }`. Babbage/Conway Praos
     only (Conway-only admission per N-M-C).
   - `IntersectFound`: emit TipUpdate; queue chain-sync
     RequestNext (NOT block-fetch). The pre-FOLLOW
     tip-jump caused `SlotBeforeLastApplied` rejections on
     subsequent in-order RollForwards.
   - `RollForward`: emit TipUpdate; extract header point;
     queue block-fetch RequestRange for the rolled-forward
     point. Don't queue chain-sync RequestNext here —
     block-fetch BatchDone is the sequencing anchor.
   - Malformed RollForward header → fail-closed
     (`ChainSyncDecode`), no silent skip.

2. **Op-cert counter rule** (`crates/ade_core/src/consensus/praos_state.rs`
   + `crates/ade_core/src/consensus/header_validate.rs`):
   - `OpCertCounterMap::upsert_strict`: regression rule is
     now `counter < existing` (was `<=`). Equal counter is
     no-op (same op-cert re-used across blocks within a KES
     period — normal pool operation per the Cardano
     protocol). Strictly greater = new op-cert rotation,
     updates the map.
   - `header_validate` Step 4: regression check uses `<`
     instead of `<=`.
   - This rule was over-rejecting because the pre-FOLLOW
     positive Conway corpus covers 14 different pools each
     appearing once, so equal-counter cases never surfaced.
     Sustained live admission against a real peer triggered
     the bug on the third admitted block.

### Tests (10 new / changed)

In `crates/ade_runtime/src/admission/wire_pump.rs::tests`:
- `extract_chain_sync_header_point_returns_slot_and_hash`
- `extract_chain_sync_header_point_rejects_malformed_envelope`
- `pump_emits_tip_update_and_request_next_on_intersect_found_no_block_fetch`
  (renamed; assertion flipped to no-block-fetch)
- `rollforward_drives_block_fetch_then_request_next` (new)

In `crates/ade_core/src/consensus/praos_state.rs::tests`:
- `op_cert_upsert_accepts_equal_counter_as_noop` (renamed +
  assertion flipped)
- `op_cert_upsert_accepts_monotonic_increasing` (renamed;
  asserts equal-counter is OK)

In `crates/ade_core/src/consensus/op_cert.rs::tests`:
- `apply_op_cert_accepts_equal_counter_as_noop` (renamed +
  assertion flipped)

Totals: 6 wire_pump tests green (was 2), 68 ade_core
consensus tests green (was 67 + replacements), 34 ade_node
admission tests green. `cargo test --workspace` clean.

### Evidence

`docs/evidence/phase4-n-m-follow-sustained-transcript.jsonl`
(committed) — 105 JSONL lines:
- 1 `admission_started` with `consensus_inputs_fingerprint`
- 1 `bootstrap_complete` with `initial_ledger_fp`
- 34 `block_received` + 34 `block_admitted` + 34
  `agreement_verdict { kind: "lagging" }` interleaved in
  the natural order chain-sync emits them
- 0 `agreement_verdict { kind: "diverged" }`
- 0 mismatched-hash `block_admitted`
- 1 clean `admission_shutdown { reason: "signal_received" }`

The 34 admitted blocks span slots 124137045 → 124137868
(~823 slots, mainnet ~14 minutes of chain time). Walking
from the seed point at slot 124136968, the first admit is
the first block AFTER the seed point.

The committed SCHED transcript at
`docs/evidence/phase4-n-m-c-operator-pass-transcript.jsonl`
still stands as the load-bearing evidence for
DC-EVIDENCE-01's literal "≥1 Agreed" requirement.

## Why all 34 verdicts are `lagging`, not `agreed`

The GREEN verdict reducer (`ade_node::admission::verdict::derive`)
emits `Agreed { slot, hash }` only when:
- Our admit's slot == peer's TIP slot, AND
- Our hash == peer's TIP hash.

Under chain-sync-driven sequential admission, we walk from
the intersect point forward; the peer's tip is far ahead
(typical preprod block time ~20s, our admit rate ~5s/block,
so we close the gap at ~4 blocks/min over a ~600-block
backlog → ~150 min to catch up). For this slice's bounded
sustained transcript, we stay in lagging territory.

`Lagging` is evidence-state, not failure — per
[[feedback-evidence-reducers-are-green-not-authority]]. The
reducer never says "agreed" for non-tip blocks even though
those blocks came FROM the peer's chain-sync stream
(implicit agreement). A future cluster could enrich the
reducer to emit `agreed` for chain-sync-streamed blocks; out
of FOLLOW's scope.

## Registry effects

- `CN-PUMP-01`, `DC-PUMP-01`, `DC-PUMP-02`:
  `strengthened_in` += `PHASE4-N-M-FOLLOW`. Evidence
  extended with the sustained transcript.
- `DC-EVIDENCE-01`: `strengthened_in` += `PHASE4-N-M-FOLLOW`.
  Stays `enforced` (literal claim met by SCHED transcript).
  Evidence extended.
- `DC-EVIDENCE-02` (adversarial false-accept):
  `strengthened_in` += `PHASE4-N-M-FOLLOW`. The 34-block
  natural-stream complement to the 4-mutation adversarial
  corpus.
- `RO-LIVE-05`: `strengthened_in` += `PHASE4-N-M-FOLLOW`.
  Same shape.

## Open obligations (post-FOLLOW)

- **RO-LIVE-03 (wide 30-min admission against arbitrary
  peer)** — still open. FOLLOW closes the BOUNDED sustained
  admission against local docker preprod for ~3 minutes.
- **ChainDb persistence of admitted blocks** — still open.
  Current state: in-memory only; restart re-bootstraps.
- **Rollback replay** — still open. FOLLOW handles
  RollBackward by emitting TipUpdate + RequestNext only; no
  re-derivation against already-admitted blocks.
- **Multi-epoch admission** — still open.
- **Block production live pass** — still open.
- **Verdict reducer enrichment** (`agreed` for chain-sync-
  streamed blocks even when peer tip is ahead) — future
  cluster, not blocking.

## What's NOT in this cluster

- Catching up to the live tip in one pass.
- Verdict reducer changes.
- ChainDb persistence.
- Rollback replay.
- Block production.

## References

- Cluster doc: `docs/clusters/PHASE4-N-M-FOLLOW/cluster.md`.
- Slice doc: `docs/clusters/PHASE4-N-M-FOLLOW/F1.md`.
- Predecessor closures: `d8feabb` (SCHED), `4d3dc98` (FRAG),
  `03d1d24` (A1.1), `8843e20` (N-M-C).
- Doctrine: [[feedback-tx-validity-priority]],
  [[feedback-evidence-reducers-are-green-not-authority]],
  [[feedback-shell-must-not-overstate-semantic-truth]],
  [[feedback-fail-closed-validation]].
