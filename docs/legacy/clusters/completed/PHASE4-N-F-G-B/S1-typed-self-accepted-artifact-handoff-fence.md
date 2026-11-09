# PHASE4-N-F-G-B — Slice S1: Typed self-accepted-artifact handoff fence

> **Status:** slice doc (IDD Part IV). Companion to `cluster.md` (S1 row + CE-G-B-1). Code-verified
> against HEAD `1806584c`.
>
> **Slice S1 in one line:** surface the **BLUE `AcceptedBlock`** that the forge already produces at
> its `self_accept` step (today discarded) into a **typed, constructor-fenced handoff carrier**
> whose only provenance is that self-accept — so a serve task (S2) can be typed to receive *only* a
> self-accepted artifact, never raw bytes / a failed outcome / a self-declared flag / a peer
> verdict. **No serve task, no `push_atomic`, no block-fetch in this slice.**

## 1. Slice identity
- **Cluster:** PHASE4-N-F-G-B (self-accept→serve handoff). S1 is the **fence**; S2 is the sibling
  serve task; S3 is the block-fetch payload + gate.
- **Slice:** S1 — typed self-accepted-artifact handoff fence (the GREEN constructor fence +
  surfacing the BLUE `AcceptedBlock` from the forge path).
- **Modules:** **GREEN** new `SelfAcceptedHandoff` carrier (a constructor-fenced newtype over the
  BLUE `AcceptedBlock`); **RED** the node forge path (`forge_one_from_recovered` / `run_real_forge`)
  surfaces the already-produced `AcceptedBlock` instead of discarding it. **No BLUE change; no new
  `CoordinatorEvent` variant.**

## 2. Cluster Exit Criteria addressed (verbatim)
- **CE-G-B-1 (handoff fence)** — only a BLUE self-accepted artifact (the `AcceptedBlock` from
  `self_accept`) enters the serve task via a typed constructor-fenced handoff; candidate tests
  `handoff_carrier_constructs_only_from_self_accepted_forge`, `serve_ingress_rejects_failed_forge_outcome`
  (or a compile-time unrepresentability assertion), `handoff_carrier_has_no_raw_bytes_constructor`.
  *(introduces the GREEN fence backing `DC-NODE-06`.)*

(CE-G-B-2 = S2 sibling serve task, CE-G-B-3 = S3 block-fetch payload + gate — out of S1 scope.)

## 3. Intent (invariant impact)
Make **"serve an artifact that was not BLUE self-accepted" unrepresentable** at the type level.
Today the node forge tick produces a `CoordinatorEvent` (`ForgeSucceeded { artifact:
ForgedBlockArtifact{bytes} }`) and the BLUE `AcceptedBlock` minted inside `run_real_forge`
(produce_mode.rs:904) is **dropped** — the only way produce-mode later serves is to *re-validate*
the bytes through `self_accept` again (`ChainEvolution::advance`). S1 closes that gap on the node
spine: the forge surfaces the **original** `AcceptedBlock` into a typed `SelfAcceptedHandoff` whose
**sole constructor** takes an `AcceptedBlock` (itself producible only by `self_accept`,
CN-FORGE-01). The serve task (S2) will be typed to consume `SelfAcceptedHandoff`, so raw bytes, a
`ForgeNotLeader`/`ForgeFailed` outcome, a boolean "accepted" flag, or a peer verdict **cannot**
reach it. This is the handoff fence; the serve mechanism is S2.

## 4. Pre-conditions (verified at HEAD `1806584c`)
- **The BLUE accept + the discard:** `run_real_forge` (produce_mode.rs:642) step 6 calls
  `self_accept(&forged.bytes, …)` → `accepted: AcceptedBlock` (produce_mode.rs:904-919); it is used
  only for a hash defense (`accepted_block_hash`, :922) and then **dropped** —
  `artifact_from_accepted(_accepted, hash, bytes)` ignores `_accepted` (:953-963) and
  `ForgeSucceeded` carries `ForgedBlockArtifact{slot,hash,bytes}` (:925-928).
- **The BLUE fence (CN-FORGE-01):** `AcceptedBlock { bytes }` has a **private field** with a
  module-only constructor — "only `self_accept` returning `Ok(...)` produces" it (`self_accept.rs`);
  it cannot be fabricated from raw bytes.
- **The node forge tick:** `forge_one_from_recovered` (node_sync.rs:378) → `run_real_forge` →
  `CoordinatorEvent`; the tick pushes it to `hermetic_forge_outcomes` (node_lifecycle.rs:715),
  self-accept-only, advances no tip.
- **The serve consumer (S2, not S1):** `ServedChainHandle::push_atomic(accepted: AcceptedBlock)`
  (served_chain_handle.rs:101) is the single served-chain mutation authority.
- **The closed evidence enum:** `CoordinatorEvent` is a closed GREEN enum shared with produce_mode
  (coordinator.rs:192) — S1 must **not** add a variant or a field to `ForgeSucceeded`.

## 5. The fix (surface the BLUE `AcceptedBlock`; wrap it in a fenced carrier)
1. **GREEN carrier** (new): `SelfAcceptedHandoff` — a newtype holding the BLUE `AcceptedBlock`, with
   a **single** constructor `from_self_accepted(AcceptedBlock) -> Self` and a `slot`/identity
   accessor for S2's serve. No constructor from `Vec<u8>` / `ForgedBlockArtifact` / `CoordinatorEvent`
   / a bool / a peer verdict. Pure; deterministic.
2. **Surface the `AcceptedBlock` (RED, minimal):** `run_real_forge` returns the `AcceptedBlock` it
   already mints alongside its `CoordinatorEvent` — i.e. `(CoordinatorEvent, Option<AcceptedBlock>)`,
   `Some` exactly on the `ForgeSucceeded`/self-accept-`Ok` path, `None` on
   `ForgeNotLeader`/`ForgeFailed`. **`ForgeSucceeded` itself is unchanged** (no new field — the
   carrier rides a separate return component, per the cluster's S1 + the edit-3 constraint).
   `forge_one_from_recovered` wraps the `Some(AcceptedBlock)` into
   `SelfAcceptedHandoff::from_self_accepted(...)` and surfaces it for the (S2) serve task; on `None`
   there is no handoff. produce_mode's existing serve path is functionally unaffected (it ignores
   the new component this slice).
3. **No re-validation, no re-fabrication:** the carrier holds the *original* self-accept token; it
   never re-runs `self_accept` on bytes and never reinterprets `ForgedBlockArtifact.bytes` as
   accepted.

## 6. TCB color (execution boundary)
- **GREEN (new):** `SelfAcceptedHandoff` — the constructor-fenced carrier (pure; no I/O / clock /
  rand / float).
- **RED (minimal):** `run_real_forge` return-shape extension + `forge_one_from_recovered` wrapping —
  surfaces a BLUE token already produced; no new I/O, no serve, no `push_atomic`.
- **BLUE (consume only):** `ade_ledger::producer::{self_accept, AcceptedBlock}`. No BLUE change.

## 7. Invariants preserved (must not weaken) — by registry ID
- `CN-FORGE-01` — `self_accept` stays the sole `AcceptedBlock` producer; the carrier wraps that
  token, it does not add a second producer or a raw-bytes path.
- `DC-NODE-05` / `CN-NODE-02` — the relay-loop forge tick stays self-accept-only, advances no
  durable tip, serves nothing; surfacing the token to a return value is not a serve/tip mutation.
  `ci_check_node_run_loop_containment.sh` unchanged.
- `T-DET-01` — the carrier is pure; no determinism tripwire.
- The closed `CoordinatorEvent` surface — **no new variant, no new field on `ForgeSucceeded`**;
  produce_mode untouched.

## 8. Invariants strengthened (one family: typed self-accepted handoff provenance)
**Family:** *the only artifact that can be handed toward serving is a BLUE self-accepted
`AcceptedBlock`, carried in a typed fence whose provenance chains to `self_accept` — never raw
bytes, a failed outcome, a flag, or a peer verdict.*
- `DC-NODE-06` — S1 **introduces the GREEN constructor fence** that backs this rule's "typed,
  constructor-fenced artifact whose only provenance is the `AcceptedBlock` produced by BLUE
  `self_accept`" clause. **No registry edit / status flip in this slice** (DC-NODE-06 flips
  declared→enforced at G-B close, when CE-G-B-1..3 are all green, per the G-A per-slice pattern).

## 9. Slice-entry decisions (settled)
- **D-1 — how the `AcceptedBlock` reaches the carrier (DECIDED: surface the original from
  `run_real_forge`).** The forge already mints the `AcceptedBlock` and discards it; S1 returns it
  (`(CoordinatorEvent, Option<AcceptedBlock>)`) and wraps it. **Not** re-validated via
  `ChainEvolution::advance` (the produce_mode pattern) and **not** reconstructed from
  `ForgedBlockArtifact.bytes` — carrying the original is the faithful reading of "carry the BLUE
  `AcceptedBlock`, not re-derive it." `ForgeSucceeded` is unchanged (the token rides a sibling
  return component, so the shared GREEN evidence enum is not widened).
- **D-2 — carrier home (DECIDED: `ade_runtime::producer`).** `SelfAcceptedHandoff` lives beside
  `served_chain_handle` (the S2 consumer) and wraps `ade_ledger::producer::AcceptedBlock`;
  GREEN-by-content. (Alternative `ade_node` rejected — the carrier is shared between the forge
  surface and the serve task, both of which sit at the `ade_runtime` producer seam.)
- **D-3 — failed/non-leader outcomes (DECIDED: no carrier).** `ForgeNotLeader` / `ForgeFailed` →
  `None` → no `SelfAcceptedHandoff`. The "handoff a failed outcome" state is unrepresentable (the
  constructor takes only an `AcceptedBlock`).

## 10. Replay / determinism obligations
`SelfAcceptedHandoff` is a pure wrapper — same `AcceptedBlock` → same carrier. No
wall-clock/rand/float. The `run_real_forge` return-shape change preserves its existing determinism
(`run_real_forge_is_byte_identical_across_two_runs` must stay green). No new authoritative state, no
new canonical type (the wrapped bytes are the existing canonical forged block), no WAL/checkpoint
change.

## 11. Replay / crash / epoch validation (tests by name)
- **New (the fence):**
  - `handoff_carrier_constructs_only_from_self_accepted_forge` — a `SelfAcceptedHandoff` is
    obtainable only from an `AcceptedBlock` produced by `self_accept` (constructed end-to-end from a
    forge whose self-accept passes).
  - `handoff_carrier_has_no_raw_bytes_constructor` — there is no public constructor from `Vec<u8>` /
    `ForgedBlockArtifact` / `CoordinatorEvent` (compile-fenced; asserted structurally + by a
    trybuild-style or doc-level negative if used).
  - `forge_surfaces_accepted_block_only_on_self_accept` — `run_real_forge` /
    `forge_one_from_recovered` returns `Some(AcceptedBlock)` on `ForgeSucceeded` and `None` on
    `ForgeNotLeader` / `ForgeFailed`.
  - `serve_ingress_type_rejects_failed_forge_outcome` — the handoff-typed ingress cannot be
    constructed from a non-`ForgeSucceeded` outcome (type-level; the carrier has no such
    constructor).
- **Preserved:** `run_real_forge_is_byte_identical_across_two_runs`,
  `broadcast_rejects_non_self_accepted_block` (the produce-mode self-accept negative) stay green.
- **No serve/block-fetch test here** — that is S2/S3.

## 12. Mechanical acceptance criteria
- [ ] `cargo test -p ade_runtime --lib` + `cargo test -p ade_node --lib` — the four S1 tests green;
      `run_real_forge_is_byte_identical_across_two_runs` + `broadcast_rejects_non_self_accepted_block`
      still green.
- [ ] `ci_check_node_run_loop_containment.sh` green + **byte-unchanged** (no serve/tip token added
      to the loop body).
- [ ] `ci_check_no_independent_forge_codepath.sh` green (no parallel forge path).
- [ ] `grep` proof: `SelfAcceptedHandoff` exposes no constructor taking `Vec<u8>` /
      `ForgedBlockArtifact` / `CoordinatorEvent` (folded into the S3 `ci_check_served_chain_handoff_fence.sh`,
      or a focused S1 assertion) — **no new serve gate required to land S1**; the fence is
      type-level.
- [ ] `cargo build` + `cargo clippy` clean on touched crates; `rustfmt` on changed files only (no
      workspace `cargo fmt -p`).
- [ ] No new `CoordinatorEvent` variant or `ForgeSucceeded` field (diff inspection).
- [ ] Acceptance scoped to `ade_runtime` + `ade_node` (consumed `ade_ledger`) — not the full
      `ade_testkit` corpus lane.

## 13. Failure modes
All **fail-closed / unrepresentable**:
- A non-self-accepted outcome → `None` → no carrier (cannot hand off a failed forge).
- No raw-bytes / event / flag / verdict constructor exists for `SelfAcceptedHandoff` (type-level).
- The carrier holds the original BLUE token; no re-validation path, no bytes-as-accepted
  reinterpretation.

## 14. Hard prohibitions (inherits the cluster "Forbidden during this cluster" list)
- **No constructor for `SelfAcceptedHandoff`** from raw bytes, `ForgedBlockArtifact`, a
  `CoordinatorEvent`, a self-declared flag, or a peer verdict.
- **No new `CoordinatorEvent` variant; no new field on `ForgeSucceeded`** (the shared GREEN evidence
  enum stays closed).
- **No re-validation via `self_accept`/`advance` and no reinterpreting `ForgedBlockArtifact.bytes`
  as accepted** — carry the original token.
- **No serve / `push_atomic` / block-fetch / sibling-task wiring** (that is S2/S3); no durable-tip
  mutation.
- **No relay-loop containment relaxation** (`ci_check_node_run_loop_containment.sh` byte-unchanged).
- No new **BLUE authority / canonical type**.
- **Hard line:** if surfacing the `AcceptedBlock` needs a BLUE change, a new evidence variant, or
  any serve wiring — **stop and re-scope.**

## 15. Explicit non-goals
No sibling serve task / `push_atomic` (S2). No block-fetch payload / tag-24 proof / handoff gate
(S3). No live feed / `WirePump` (G-C). No peer acceptance / BA-02 (G-C). No change to produce_mode's
existing serve path (this slice only adds the surfaced return component it may ignore).

## 16. Completion checklist
- [ ] `SelfAcceptedHandoff` added (GREEN, single `from_self_accepted(AcceptedBlock)` constructor);
      `run_real_forge` surfaces `Some(AcceptedBlock)` on self-accept (`ForgeSucceeded` unchanged);
      `forge_one_from_recovered` wraps it; `None` on failed outcomes.
- [ ] All §12 tests green; containment + no-independent-forge gates green & unchanged; `clippy`
      clean; changed files rustfmt'd.
- [ ] Slice doc committed standalone (`docs:`) before implementation; impl committed (`feat:`/
      `test:`) after green, model-attribution trailer. **No registry edit** (DC-NODE-06 flip
      deferred to G-B close).

## Authority
Registry IDs `DC-NODE-06` (introduces the GREEN handoff fence; registry flip **deferred to G-B
close**); `CN-FORGE-01` / `DC-NODE-05` / `CN-NODE-02` / `T-DET-01` (preserved). The cluster doc
`cluster.md` and `docs/ade-invariant-registry.toml` are authoritative; this slice doc refines, it
does not override.
