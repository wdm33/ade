# Invariant Slice — PHASE4-N-AE.A: Forge-on-Followed-Tip Gate + Followed-Block Serve Continuity

## §2 Slice Header
- **Slice Name:** forge-on-followed-tip admission gate + followed-block serve continuity
- **Cluster:** PHASE4-N-AE (Recover→Serve Continuity and Forge Admissibility) — primary invariants **DC-NODE-15** + **DC-CONS-24**
- **Status:** Proposed
- **Cluster Exit Criteria Addressed:** **CE-A1** (DC-NODE-15 — forge admissible only when `durable_servable_tip == followed_peer_tip`; typed `ForgeRefused::NotCaughtUp`; no `recovered.tip` forge-base fallback; peer-tip is admissibility-only), **CE-A2** (DC-CONS-24 — forged parent byte-equals the followed peer tip), **CE-A3** (DC-NODE-14 *followed-tip lineage clause* — served chain intersects at the followed tip and rolls forward to the forged successor), **CE-A4** (T-REC-05 — recover→follow→forge replay determinism), **CE-A5** (live C2-LOCAL relay-adoption manifest, *operator-gated* — the bounty-relevant #8–#9 closure; **not a CI criterion**). *(CE-B1/B2 recovered-anchor intersectability = AE.B, out of scope.)*

## §3 Dependencies
Cluster entry conditions only (N-U `pump_block`/`ChainDbServedSource`/extend-only durable chain; N-M-* `seed_to_snapshot` recover; N-F-C/D/E/F relay loop + `forge_one_from_recovered` + `forge_header_position` + the DC-EPOCH-03 off-epoch gate; the follow path durably stores the peer tip as a `StoredBlock` via `pump_block`). **No dependency on AE.B** — AE.A is the C2-LOCAL closer; AE.B's anchor work builds on it.

## §4 Intent (invariant impact)
Introduce **DC-NODE-15**: a `--mode node` forge is admissible **only** when the durable servable tip equals the followed peer tip, fail-closed to a typed `ForgeRefused::NotCaughtUp` otherwise — and **DC-CONS-24**: the forged successor's parent hash **byte-equals** that peer-visible selected tip. Before AE.A the ForgeTick `selected_tip` falls back to the snapshot-only recovered anchor (`node_lifecycle.rs:1102`) when the durable tip is empty, so a forge on a `NoWorkReady` gap builds a successor a peer cannot intersect (PO2). After AE.A the forge waits until the follow has durably stored the peer tip `T`, builds `T+1` on it, and the relay intersects at `T` and adopts — closing C2-LOCAL #8–#9. Partially enforces **DC-NODE-14** (the *followed-tip lineage clause*: every claimed forge parent is a servable point; the recovered-anchor clause stays open for AE.B).

## §5 Scope / What is built
- **NEW closed GREEN classifier** in `ade_node::node_sync` (sibling to `forge_epoch_admission`), pure/deterministic (no I/O/clock/rand/float), comparing **hash AND block_no**:
  ```
  fn forge_followed_tip_admission(
      durable_servable_tip: Option<TipPoint>,
      followed_peer_tip: Option<TipPoint>,
  ) -> ForgeFollowedTipAdmission

  enum ForgeFollowedTipAdmission { CaughtUp, NotCaughtUp { reason: NotCaughtUpReason } }
  enum NotCaughtUpReason { NoFollowedPeerTip, NoDurableServableTip, TipMismatch }
  ```
  Both tips are `Option<TipPoint>` (`ade_ledger::receive::events::TipPoint { slot, hash, block_no }`) — absence is a distinct, named reason, **never** a fake peer tip and never silently treated as an equality failure.
- **NEW typed structured refusal** (dedicated closed sum, semantically distinct from a forge *error*):
  ```
  enum ForgeRefused {
      NotCaughtUp { local_servable_tip: Option<TipPoint>, followed_peer_tip: Option<TipPoint>, reason: NotCaughtUpReason },
  }
  enum NodeForgeOutcome { Forged(/* existing success carrier */), Refused(ForgeRefused), Failed(NodeForgeError) }
  ```
  **Refused** = the admissibility gate prevented the forge; **no state transition attempted**, tip unchanged, no handoff. **Failed** = the forge path was attempted and failed. *(If the existing code already carries an equivalent refusal/outcome sum that `NotCaughtUp` should join — e.g. the closed `CoordinatorEvent` refusal surface — the implementer joins it instead of adding `NodeForgeOutcome`, provided the refusal stays typed + structured carrying the tips + reason; it is **never** buried as a log message or a generic handoff error.)*
- **REMOVE the `recovered.tip` fallback** at the ForgeTick `selected_tip` derivation (`node_lifecycle.rs:1098-1103`). The forge base is the durable servable tip; `recovered.tip` is **no longer a forge base**.
- **THREAD a structured followed-peer-tip admissibility input** into the ForgeTick arm. Since `NodeBlockSource` deliberately **skips** `AdmissionPeerEvent::TipUpdate` (`node_sync.rs:135,282`), AE.A introduces a *separate* structured input carrying the followed peer tip. **The followed-peer-tip signal may only prevent a forge. It may not select, replace, reorder, or prefer chains.**
- **Followed-block serve continuity is expected from the existing `pump_block` durability and is proven by this slice's tests; no serve-projection change is intended.** The follow already stores the peer tip as a `StoredBlock` (DC-NODE-12); gating the forge to build on it makes `ChainDbServedSource::intersect([T]) == Some(T)` + `next_after(T)` project the forged successor (DC-NODE-14 followed-tip clause). This slice *mechanically proves* that continuity rather than assuming it.
- **NEW gate** `ci/ci_check_forge_followed_tip_admission.sh`.
- **Fold in the red diagnostic** `crates/ade_node/tests/phase4_n_ae_recover_serve_continuity_diag.rs`: un-ignore + adjust the AE.A fixture (`forge_base_falls_back_to_snapshot_anchor` → asserts the refusal / no-fallback) and add the positive caught-up tests; the **AE.B fixtures stay `#[ignore]`d** (red-on-demand until AE.B).

## §6 Execution Boundary (TCB color)
- **GREEN (new):** `node_sync::forge_followed_tip_admission` + `ForgeFollowedTipAdmission` + `NotCaughtUpReason` (GREEN-by-fn, like `forge_epoch_admission`); the followed-peer-tip → structured admissibility-input conversion.
- **RED (changed):** `ade_node::node_lifecycle` (remove the `recovered.tip` fallback; thread the followed-peer-tip signal into ForgeTick), `ade_node::node_sync` (call the classifier before leadership; surface `ForgeRefused::NotCaughtUp` / `NodeForgeOutcome`).
- **RED (reused, unchanged):** `ade_runtime::forward_sync::pump` (the follow stores the peer tip), `ade_runtime::network::served_chain_projection` (`ChainDbServedSource` — read only), `ade_runtime::{chaindb, wal}`.
- **BLUE (reused, NOT edited — no new type):** `ade_ledger::{block_validity, receive::events::TipPoint, producer::{forge, self_accept}}`, `ade_core::consensus`. `fork_choice`/`select_best_chain` is **not** on this path.
- `ForgeRefused` / `ForgeFollowedTipAdmission` / `NodeForgeOutcome` are closed RED/GREEN sums, **not** canonical types.

## §7 Invariants Preserved
DC-NODE-05 (forge subordinate to the sync spine — preserved + reinforced: the forge now also waits for the durable tip to reach the peer tip), DC-NODE-12 (durable admit via `pump_block` — unchanged), DC-NODE-13 / CN-CONS-07 (serve-as-projection + serve provenance — unchanged; the gate gives the projection an intersectable parent), DC-CONS-23 (extend-only — the gate prevents forking off a stale/anchor tip), DC-CONS-03 (Praos fork-choice authority — **not** touched; no fork-choice added), CN-FORGE-01 (self-accept token unchanged), DC-NODE-08/10 (cold-start / forge-successor position — upstream of the gate), DC-EPOCH-03 (off-epoch gate runs alongside the new caught-up gate), DC-SYNC-01/02 / CN-NODE-02 (`pump_block` stays the sole tip authority; containment gates **not regressed**).

## §8 Invariants Strengthened or Introduced
One invariant family — **forge-on-followed-tip admissibility** (the forge produces a peer-adoptable successor on the followed tip; the facets below are inseparable properties of that one law):
- **DC-NODE-15** — introduced → **enforced** (the gate; the slice's primary invariant).
- **DC-CONS-24** — introduced → **enforced** (forged parent byte-equals the peer-visible selected tip).
- **DC-NODE-14** — introduced → **partial** (followed-tip lineage clause enforced; recovered-anchor clause = AE.B).
- **Strengthens** (`strengthened_in += "PHASE4-N-AE"`): **DC-EPOCH-03** (a sibling fail-closed forge-admissibility boundary), **DC-NODE-05** (forge-subordination now also requires caught-up-to-peer-tip), **T-REC-05** (replay determinism extends to the recover→follow→forge served chain).

## §11 Replay / Crash / Epoch Validation
- **Replay (in-run determinism):** the served chain + forged successor are a deterministic function of (recovered anchor, followed canonical blocks). Test: `recover_follow_forge_two_runs_byte_identical` (CE-A4).
- **Crash recovery:** unchanged — the follow + forged successor ride the existing `pump_block` durable admit + `warm_start_recovery` (T-REC-05, from N-U/N-AD); not weakened. No new recovery path.
- **Epoch:** unchanged — DC-EPOCH-03's off-epoch gate is upstream; the new caught-up gate is a sibling fail-closed boundary in the same forge fence.
- **Live closure (CE-A5, operator-gated — NOT a CI test):** the non-producing-relay C2-LOCAL venue → `ba02_evidence::correlate` manifest with **forged hash == adopted hash** (`AddedToCurrentChain`). Non-promotable rehearsal; **flips no RO-LIVE rule**.

## §12 Mechanical Acceptance Criteria
- `cargo test -p ade_node` green incl. NEW / un-ignored (in `phase4_n_ae_recover_serve_continuity_diag.rs` + `node_sync`/`node_lifecycle` tests):
  - `forge_refused_not_caught_up` — `durable_servable_tip != followed_peer_tip` ⇒ typed `ForgeRefused::NotCaughtUp` with the correct `NotCaughtUpReason`, no forge, **no `recovered.tip` base** (the adjusted `forge_base_falls_back_to_snapshot_anchor`).
  - `forge_on_followed_tip_proceeds_with_parent_byte_equal` — caught-up ⇒ forge proceeds; forged `prev_hash` byte-equals `T`; `block_no == T.block_no + 1` (DC-CONS-24).
  - `served_chain_intersects_at_followed_tip_and_rolls_to_forged` — `ChainDbServedSource::intersect([T]) == Some(T)`; `next_after(T)` projects the forged successor (DC-NODE-14 followed-tip clause).
  - `recover_follow_forge_two_runs_byte_identical` — replay determinism (CE-A4).
- NEW `ci/ci_check_forge_followed_tip_admission.sh` green: (a) **no `recovered.tip` fallback** at the ForgeTick `selected_tip`; (b) forge fires only when `durable_servable_tip == followed_peer_tip` (hash AND block_no); (c) `NotCaughtUp` is a typed refusal carrying `{local_servable_tip, followed_peer_tip, reason}` (no log-string-only path); (d) the followed-peer-tip signal does **not** reach `select_best_chain` / `chain_selector` / `fork_choice` (static grep/check).
- `ci/ci_check_node_run_loop_containment.sh`, `ci/ci_check_served_chain_projection.sh`, `ci/ci_check_loop_planner_closed.sh` green (**no regression** — allow-list may be updated for the gate call, no containment relaxed).
- `cargo test --workspace` green (the AE.A diagnostic fixtures now run un-ignored; AE.B fixtures stay `#[ignore]`d; `ade_testkit` corpus-suite environmental timeout reported honestly at cluster-close).

## §13 Failure Modes
- `ForgeRefused::NotCaughtUp { local_servable_tip, followed_peer_tip, reason }` — **fail-closed, deterministic, no replay impact**: the forge does not fire that slot (no partial state, tip unchanged), exactly like an off-epoch / not-leader skip. The forge **never** builds on a non-servable base. The three reasons (`NoFollowedPeerTip`, `NoDurableServableTip`, `TipMismatch`) are diagnostic-distinct.

## §14 Hard Prohibitions
**Inherited (cluster §11):** no synthetic servable `StoredBlock` bytes; **no `recovered.tip` fallback as a forge base**; no parent-hash inference from block number; **no peer-tip signal as a chain selector**; no C2-only bypass; no fork-choice / multi-producer intake; no new BLUE authority or canonical type; no second durable tip-advance path; no containment-gate regression; no RO-LIVE flip on local/hermetic evidence.
**Slice-specific:**
- The classifier is a **pure GREEN fn** (no I/O/clock/rand/float).
- `ForgeRefused::NotCaughtUp` is a typed structured refusal (never a log-string-only path); `Refused` is semantically distinct from `Failed` (gate-prevented, no state transition).
- The followed-peer-tip is an **admissibility input only — it may only prevent a forge; it may not select, replace, reorder, or prefer chains** (must not call / reach `select_best_chain` / `chain_selector` / `fork_choice`).
- Do **not** revive `AdmissionPeerEvent::TipUpdate` as a sync / chain-selection tip authority — introduce a separate structured admissibility input.
- **No anchor materialization** (that is AE.B); no Option A/B work here.

## §15 Explicit Non-Goals
Recovered-anchor intersectability / Option A/B (AE.B); fork-choice / multi-producer candidate intake (Gap 1 — separate future cluster); any ChainDB redesign; any RO-LIVE flip / preprod live pass; reviving `TipUpdate` as a sync authority.
