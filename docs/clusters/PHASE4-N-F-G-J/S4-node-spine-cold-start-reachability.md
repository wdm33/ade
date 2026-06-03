# Invariant Slice — PHASE4-N-F-G-J S4: Node-spine cold-start first-block reachability

> **Status:** Planning artifact (non-normative). Normative authority is the registry + CI.

## §2 Slice Header

- **Slice:** PHASE4-N-F-G-J S4 — node-spine first-block reachability (both tips `None` → forge block 0 + `Genesis` through the accepted path, exactly one genesis handoff per cold-start execution).
- **Cluster:** PHASE4-N-F-G-J — Genesis-successor block correctness (`c167cd41`).
- **Status:** Proposed (doc-before-implement).
- **Cluster Exit Criteria addressed — CE-G-J-4 (verbatim):** "a hermetic first-block-from-empty-feed forge tick fires, self-accepts → handoff → served from the recovered lineage when both tips are `None` + eligible feed + `ForgeIntent::On`, exactly once. Named test resolved in the S4 slice doc; `DC-NODE-08` declared → enforced." *(CE-G-J-1/2/3 already met; CE-G-J-5 C1 rehearsal out of scope.)*

## §3 Slice Dependencies

- **S3** (`CN-WIRE-09` position clause, enforced — `0c1939a1`) — **hard dependency**: the genesis block this slice reaches (`block_number 0` + `PrevHash::Genesis`) is admitted only because `check_header_position` accepts it and `forge_to_self_accept_succeeds` already proved the forge → KES pre-image → `decode_block` → `self_accept` path for exactly that block.
- **S2** (`CN-WIRE-09` codec) — `PrevHash::Genesis` exists.
- **S1** (`CN-NODE-04`, enforced) — reused: `FeedReason::is_forge_eligible` is the feed-eligibility gate; the closed `NodeSchedEvent` vocabulary records the new path's outcome.

## §4 Intent (invariant impact)

Make the genesis-successor block **reachable on the real `--mode node` spine**: when `ChainDb::tip()` **and** `recovered.tip` are *both* `None` (a from-genesis cold start), and only when the recovered seed-epoch lineage is present, `ForgeIntent::On` with complete keys, the feed is forge-eligible, and the slot/epoch/KES/leader guards pass, the node forges `block_number 0` carrying `PrevHash::Genesis` through the **same** `run_real_forge → self_accept → SelfAcceptedHandoff` path S3 proved — emitting exactly one successful genesis handoff per cold-start execution, advancing **no** durable tip. This closes the falsified-original defect: the spine no longer halts with `NoTipAvailable` at genesis. It also collapses the **two** cold-start block-number conventions into **one** (`0`).

## §5 Scope / What is built

1. **`ade_node::node_sync::forge_one_from_recovered` — accept the no-tip (genesis) case.**
   - Signature `selected_tip: &ChainTip → selected_tip: Option<&ChainTip>`.
   - `None` (cold start) ⇒ `block_number = 0`, `prev_hash = PrevHash::Genesis`.
   - `Some(tip)` ⇒ **unchanged** — `block_number = recovered.chain_dep.last_block_no + 1`, `prev_hash = PrevHash::Block(tip.hash)`.
   - **Convention reconciliation:** the `recovered.chain_dep.last_block_no … .unwrap_or(1)` cold-start default (`node_sync.rs:555-559`) is removed; the genesis case is driven by `selected_tip == None ⇒ 0`, matching `ChainEvolution::next_block_number()` (tip `None ⇒ 0`). One cold-start convention, not two.
2. **`ade_node::node_lifecycle` `LoopStep::ForgeTick` arm — reach the genesis forge.**
   - Today (`:1081`) `if let Some(tip) = selected_tip` skips when both tips are `None` → `forged = false` → `NoTipAvailable`. S4: a **GREEN permission gate** — when `selected_tip == None`, forge the genesis block **iff** the recovered lineage is present, `ForgeIntent::On` (the `forge` activation is `Some`), and `source.feed_reason()` is forge-eligible (`FeedReason::is_forge_eligible`, S1). Calls `forge_one_from_recovered(act.recovered, None, …)`.
   - **Exactly-one reachability (per cold-start execution):** one scheduled `ForgeTick` under both-tips-`None` reaches `forge_one_from_recovered(None)` and makes exactly one forge attempt — the arm already does one forge per tick (no loop). S4 proves *reachability*, not durable post-genesis progression, and adds **no** persistent or semi-persistent `genesis_forged` latch: the authority over whether `block 0` exists stays with the recovered lineage / accepted handoff / durable chain state, never a new local lifecycle flag. The node-spine must not emit duplicate genesis handoffs *within a single scheduled cold-start step / harness execution*. Durable suppression of repeated `block 0` forging after restart or after handoff persistence is the durability/live-progression slice (N-U), not S4.
3. **CI gate** `ci/ci_check_genesis_successor_reachability.sh` — `forge_one_from_recovered` takes `Option<&ChainTip>`; the `None` arm emits `PrevHash::Genesis` + `block_number 0`; no `unwrap_or(1)` cold-start default survives (single convention); the genesis arm is gated by `is_forge_eligible` + recovered-lineage + forge-activation, and writes no `ChainDb` tip.

**Out of scope:** the WITH-tip forge path (untouched beyond the `Option` wrap); any change to `run_real_forge` / `forge_block` / `self_accept` (S3-proven, reused verbatim).

## §6 Execution Boundary (TCB color)

- **GREEN** — the node-spine first-block **permission** decision (a pure selection over recovered state + feed eligibility + forge intent: "may a genesis block be forged here?"). Deterministic; proposes, does not define truth.
- **RED** — `ade_node::node_lifecycle` relay-loop `ForgeTick` arm wiring; `ade_node::node_sync::forge_one_from_recovered` shell composition (the `Option<&ChainTip>` plumbing + the genesis ctx assembly).
- **BLUE (reused, unchanged)** — `run_real_forge` / `forge_block` (emits `PrevHash::Genesis` for `block 0`, S3) + `self_accept` → `decode_block` → `check_header_position` (admits block 0 + `Genesis`, S3). S4 adds **no** BLUE code; it routes the cold-start ctx into the existing BLUE authority.

## §7 Invariants Preserved

- **`CN-WIRE-09`** (S3) — the genesis block reached here carries `PrevHash::Genesis` and passes `check_header_position`; S4 produces no position-illegal pair (the cold-start ctx sets `block 0` + `Genesis` together).
- **`DC-NODE-06` / `DC-NODE-07`** — the genesis block flows through the existing `SelfAcceptedHandoff` → single `ServedChainView`; no new serve/handoff surface.
- **`DC-FORGE-01` / `CN-FORGE-01..04`** — forge determinism + the closed `RequestForge → ForgeResult` transition; reused unchanged.
- **`CN-CINPUT-03` / `DC-CINPUT-02b`** — the forge base is the recovered surface only (`PoolDistrView::from_seed_epoch_consensus_inputs`); no from-genesis-file / bundle construction.
- **`DC-EPOCH-03`** — single recovered seed-epoch containment (`forge_epoch_admission` fail-closed off-epoch); unchanged.
- **`CN-NODE-04`** (S1) — feed/forge diagnostics, emit-only; reused as the eligibility source, byte-unchanged.
- **Containment** — the durable tip advances only through the accepted path, never from forge scheduling; the genesis `ForgeTick` advances **no** `ChainDb` tip (hermetic), exactly as the existing with-tip arm.
- **`RO-LIVE-01` / `RO-LIVE-06`** — no flip; the live operator-accept stays gated.

## §8 Invariants Strengthened

**`DC-NODE-08`** declared → **enforced** (cold-start reachability). The node-spine genesis-successor reachability — both-tips-`None` + recovered lineage + `ForgeIntent::On` + eligible feed (`no_block_available | clean_empty`) + slot/epoch/KES/leader guards ⇒ forge `block 0` (`PrevHash::Genesis`) through `self_accept → SelfAcceptedHandoff → ServedChainView`, emitting exactly one successful genesis handoff per cold-start execution, advancing no durable tip — becomes mechanically backed by the §11/§12 tests + the CI gate. Durable post-genesis progression (`block 1+` after a durable tip) is structurally available (the unchanged WITH-tip path) but exercised in the durability slice (N-U). **Registry:** append the S4 tests + gate to `DC-NODE-08`; flip `status = "declared" → "enforced"`. No new rule.

## §9 Open questions resolved in this slice

- **OQ-D → resolved:** eligibility reuses S1 `FeedReason::is_forge_eligible` (`no_block_available | clean_empty`); `unknown_disconnected` / error feeds never reach the genesis forge (tested negatively).
- **Cold-start convention → resolved:** one convention — `block_number 0` at no-tip, in **both** `ChainEvolution::next_block_number()` and `node_sync` (the `unwrap_or(1)` is deleted).
- **"Exactly-one" → scoped to the S4 hermetic cold-start execution:** one scheduled cold-start `ForgeTick` emits exactly one successful genesis handoff (the arm's natural one-forge-per-tick). S4 adds **no** persistent/semi-persistent latch deciding whether `block 0` exists — that authority stays with the recovered lineage / accepted handoff / durable chain state. Durable suppression of repeated `block 0` forging after restart or handoff persistence, and progression to `block 1+`, are the durability/live-progression slice (N-U). DC-NODE-08's "once a durable tip exists, block_number > 0 takes the normal path" is satisfied structurally (the WITH-tip path exists, unchanged) but not durably exercised here.

## §11 Replay / Crash / Epoch Validation

- **Genesis reachability (new):** `node_spine_cold_start_forges_genesis_block_zero` — both tips `None` + recovered lineage + eligible feed + `ForgeIntent::On` ⇒ the `ForgeTick` reaches `forge_one_from_recovered(None)` and yields `ForgeSucceeded` with `block_number 0` + `PrevHash::Genesis`, self-accepted (`Some` handoff).
- **Exactly-one per cold-start execution (new):** `node_spine_single_cold_start_tick_emits_one_genesis_handoff` — a single scheduled cold-start `ForgeTick` emits exactly one successful genesis `SelfAcceptedHandoff`; no duplicate genesis forge attempts within the execution.
- **Cold-start position authority (new, unit — the GREEN `forge_header_position`):** `forge_one_from_recovered_cold_start_is_block_zero_genesis` (`None` ⇒ 0 + Genesis) + `forge_one_from_recovered_with_tip_is_block_n_plus_one_block_prev` (the unchanged `Some` path) + `forge_header_position_some_tip_without_block_no_fails_closed` (the malformed-height edge fails closed).
- **One convention (new):** `cold_start_block_number_is_zero_single_convention` — `node_sync` cold-start `block_number == ChainEvolution::next_block_number()` at tip `None` (both `0`), cross-checked against a real `ChainEvolution::seed(..., None, ...)`.
- **Permission gate matrix (new — the GREEN `may_cold_start_forge`):** `cold_start_gate_allows_genesis_when_eligible_and_recovered`, `node_spine_cold_start_ineligible_feed_does_not_forge` (ineligible ⇒ no forge), `cold_start_gate_blocks_without_recovered_lineage`, `cold_start_gate_inactive_when_tip_present`. (`ForgeIntent::Off` ⇒ no `ForgeTick` is a planner precondition — the arm runs only with the forge activation present.)
- **Exactly-one + containment (structural, gate-pinned):** `forge_one_from_recovered` takes no `ChainDb` handle, so it cannot advance the durable tip; the `ForgeTick` arm makes one forge per tick and `forge_one_from_recovered(None)` returns exactly one `Option<SelfAcceptedHandoff>`. Enforced by `ci_check_genesis_successor_reachability.sh` (e), not a full-loop harness.
- **Crash/epoch:** none new — no WAL/checkpoint change; off-epoch stays fail-closed (`DC-EPOCH-03`, reused).

## §12 Mechanical Acceptance Criteria

Complete only when all pass in CI. The two node-spine decisions are extracted as
named GREEN functions (`forge_header_position`, `may_cold_start_forge`) and tested
directly; "advances no durable tip" + "exactly-one per execution" are pinned
structurally by the gate (the forge engine takes no `ChainDb` handle; the arm
makes one forge per `ForgeTick`) rather than by a full-loop integration harness.

- [ ] `forge_one_from_recovered_cold_start_is_block_zero_genesis`, `forge_one_from_recovered_with_tip_is_block_n_plus_one_block_prev`, `forge_header_position_some_tip_without_block_no_fails_closed` (the GREEN cold-start position authority).
- [ ] `cold_start_block_number_is_zero_single_convention` (cross-checks `ChainEvolution::next_block_number()` at tip None — one convention).
- [ ] `node_spine_cold_start_forges_genesis_block_zero` (`forge_one_from_recovered(None)` reaches the genesis forge over the recovered base; on self-accept the artifact is block 0 + `PrevHash::Genesis`).
- [ ] `cold_start_gate_allows_genesis_when_eligible_and_recovered`, `node_spine_cold_start_ineligible_feed_does_not_forge`, `cold_start_gate_blocks_without_recovered_lineage`, `cold_start_gate_inactive_when_tip_present` (the GREEN `may_cold_start_forge` permission matrix).
- [ ] `bash ci/ci_check_genesis_successor_reachability.sh` green — Option-tip signature; cold-start `Genesis` + block 0; no `.unwrap_or(1)` (one convention); lineage + eligibility gating; no-durable-tip enforced structurally (the forge engine takes no `ChainDb` handle).
- [ ] `cargo test -p ade_node -p ade_runtime` green (unmasked exit code). *(Full `cargo test --workspace` unmasked is the cluster-close gate, `RO-CLOSE-01`.)*

## §13 Failure Modes

- **Ineligible feed / `ForgeIntent::Off` / missing recovered lineage at cold start** — fail-closed: no genesis forge; the arm records the closed `NoTipAvailable` / `FeedUnavailable` `NodeSchedEvent` (S1), advances nothing. Deterministic, no `String`.
- **Off-epoch / not-leader / KES-out-of-range at the genesis slot** — the existing `forge_one_from_recovered` fail-closed outcomes (`ForgeNotLeader` / `ForgeFailed`), no handoff. Unchanged.
- **A mis-paired cold-start ctx** (defense-in-depth) — `self_accept`'s `check_header_position` rejects (S3); the node-spine surfaces it as the closed forge-failed outcome, never a served block.

## §14 Hard Prohibitions

Inherits cluster §11 in full. Slice-specific:
- **No persistent or semi-persistent `genesis_forged` latch** — the authority over whether `block 0` exists stays with the recovered lineage / accepted handoff / durable chain state, never a new local lifecycle flag. Exactly-one is proved *per cold-start execution*, not by durable suppression.
- **No durable tip advance** from the genesis `ForgeTick` (containment); the genesis forge writes **no** `ChainDb` tip and gossips/serves nothing beyond the existing `SelfAcceptedHandoff`.
- **No second cold-start convention** — `block_number 0` at no-tip is the single rule; `unwrap_or(1)` must not survive.
- **No BLUE change** — `run_real_forge` / `forge_block` / `self_accept` / `check_header_position` are reused verbatim.
- **No forge from raw/unanchored genesis** — the base is the recovered surface only; both-`None` without recovered lineage must **not** forge.
- **No `RO-LIVE-01/06` flip**, no co-producer workaround, no private-only / C1-only flag.

## §15 Explicit Non-Goals

Chain-linkage validation (does `prev_hash` equal the real parent); Mithril behavior; full live-node sync / unbounded peer following; durable single-progression to `block 1+` and durable suppression of repeated `block 0` forging across restarts (needs a durable tip advance — N-U); C1/preprod rehearsal (CE-G-J-5); any change to the WITH-tip forge path beyond the `Option<&ChainTip>` wrap.
