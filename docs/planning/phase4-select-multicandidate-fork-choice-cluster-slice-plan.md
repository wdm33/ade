# Cluster/Slice Plan — Ade · Multi-candidate fork-choice SELECT

> **Status:** Cluster/slice plan (IDD Part I / Part IV) — **overall plan only**; the full cluster doc is
> `/cluster-doc PHASE4-N-AO`. Paired with `docs/planning/phase4-select-multicandidate-fork-choice-invariants.md`.
> **Registry untouched** — rules are *declared* at `/cluster-doc`, not here.
>
> **Predecessor / precedent:** PHASE4-N-AI (single-best-peer rollback-FOLLOW, `DC-NODE-23…29`, live-proven
> CE-AI-6). This cluster reuses that precedent: latent-until-wired slices, the hermetic mechanism enforced
> first, the operator-gated live convergence CE last.

---

## Cluster Index (Dependency Order)

1. **PHASE4-N-AO — live multi-candidate fork-choice SELECT + adopt** — primary invariant: *Ade routes a
   competing multi-peer candidate set to the **existing** `select_best_chain`, durably adopts the
   fork-choice-maximal chain, and rolls back its current durable chain **only after the selected
   replacement branch is proven valid and fork-choice-maximal** — never abandoning a validated chain for
   an unvalidated one.*

> **Single cluster (not split).** S1 (peer-identity restoration) is necessary foundation but is **not, by
> itself, the SELECT invariant** — it proves no user-visible/select behavior on its own. Splitting it into
> a foundation sub-cluster would add process overhead without proving a complete SELECT behavior. It stays
> as slice 1, latent-until-wired, exactly as PHASE4-N-AI kept its detector/resolver/apply slices latent
> until the wiring slice. The hermetic mechanism (S1–S5) enforces at close; `CN-CONS-03` is **operator-
> gated** (CE-AO-6), as it stayed `declared` through N-AI with CE-AI-6 gated.

---

## PHASE4-N-AO — live multi-candidate fork-choice SELECT + adopt

- **Primary invariant:** A competing Participant candidate set is routed to the **single existing** BLUE
  `select_best_chain` (no second selector); the fork-choice-maximal chain is durably adopted through the
  **existing** enforced apply authorities; an Ade-initiated reselection (`RollbackReason::ForkChoiceWin`)
  is canonically bound, replay-equivalent, and **never abandons Ade's current durable chain until the
  selected replacement branch is fetched, linked, and validated as a complete candidate branch**.

- **TCB partition:**
  - **BLUE (reused — hypothesis: zero new canonical type):** `ade_core::consensus::fork_choice::select_best_chain`;
    `ade_core::consensus::header_validate::validate_and_apply_header`;
    `ade_ledger::rollback::materialize::materialize_rolled_back_state`; `ade_ledger::receive`
    (`commit_rollback`, `pump_block` reducer); `ade_ledger::wal::event::{WalEntry::RollBack,
    RollbackReason::ForkChoiceWin}`.
  - **GREEN / BLUE-candidate pending proof:** the per-peer candidate aggregator + candidate-set ordering.
    **This is GREEN only if S2 proves every candidate summary is derived from validated/canonical inputs
    and no RED-minted summary crosses into `select_best_chain`.** Candidate aggregation is the dangerous
    seam — if it constructs candidate summaries, filters forks, or orders candidates in a semantically
    meaningful way, it may become BLUE-adjacent; it is **not** pre-colored GREEN until S2 proves the
    authority boundary. *(The reconciliation projection is GREEN.)*
  - **RED (new wiring):** `ade_node::node_lifecycle::run_participant_sync` (dispatch + fork-switch driver),
    `ade_runtime::admission::wire_pump` (per-peer feed + selected-range fetch), `ade_node::node_sync`
    (`NodeSyncItem` peer threading), convergence-evidence emission.

- **Cluster Exit Criteria:**
  - **CE-AO-1:** Peer identity is preserved end-to-end (`AdmissionPeerEvent` → `NodeSyncItem` → participant
    loop); single-peer FOLLOW admission behavior is **byte-unchanged + replay-equivalent**.
  - **CE-AO-2:** A competing Participant candidate is assembled into a per-peer `CandidateFragment`
    **only** from Ade-validated headers (`validate_and_apply_header` output, `DC-NODE-24`); a `follow.rs`-
    minted / peer-trusted summary reaching `select_best_chain` is mechanically impossible. *(The §1 proof
    obligation, mechanically gated.)*
  - **CE-AO-3:** The live `NeedsForkChoice` arm routes the candidate **set** to the existing
    `select_best_chain`; the selected tip is **arrival-order-independent** over the live multi-peer set;
    `TiebreakerLossKeepCurrent` makes no durable change.
  - **CE-AO-4:** On a fork-choice win, **Ade never commits a rollback of its current durable chain until
    the replacement branch's bodies are fetched, linked, and validated as a complete candidate branch**
    (block-fetched via `RequestRange` anchor→tip from the winning peer, the fork anchor canonically bound
    to Ade's durable stored slot+hash); adoption proceeds via `RolledBack(fork_anchor)+ChainSelected(body)×N`
    through the existing `apply_chain_event` arms (`RollbackReason::ForkChoiceWin`). A failed / lying /
    Byzantine / incomplete replacement branch leaves Ade's current durable chain **unchanged** (FC-6 — the
    H-1 class prevented at fork-choice scale).
  - **CE-AO-5:** A `ForkChoiceWin` reselection is durably WAL-recorded and **replay-equivalent** (same
    ordered multi-peer feed → byte-identical durable tip + ledger fp + `PraosChainDepState`, including the
    reselection); `selector.current_tip == ChainDb::tip` after every applied decision; no forge across a
    pending multi-peer reselection.
  - **CE-AO-6 (operator-gated — flips `CN-CONS-03`):** On the CE-AI-6 multi-pool `cardano-testnet` venue
    (magic 42, k=5) extended to **two simultaneous competing producers**, a live convergence is captured +
    committed: Ade selects the best chain across a real fork **and rolls back its current durable chain
    only after the selected replacement branch is proven valid and fork-choice-maximal**, re-converging to
    `agreement_verdict{agreed}` (`our_hash == peer_hash`, 0 diverged).
    `blocked_until_operator_multiproducer_pass_executed`.

- **Slices:**
  - **S1 peer-identity restoration** — invariant: `peer` threaded through `NodeSyncItem` + the
    `from_wire_pump`/`next_item` conversion; FOLLOW byte-unchanged — addresses CE-AO-1 — TCB: RED/GREEN
    *(no BLUE)*. **Non-goal (S1):** S1 restores peer identity **only**; it MUST NOT alter selection,
    admission, rollback, or evidence-verdict semantics. *(Resolves OQ-SELECT-1.)*
  - **S2 BLUE-safe per-peer candidate aggregation** — invariant: per-peer `CandidateFragment`s built only
    from `validate_and_apply_header` output, never minted; latent until S3 — addresses CE-AO-2 — TCB:
    GREEN/BLUE-candidate pending proof + BLUE-reused. *(Resolves OQ-SELECT-2, incl. the fork-point
    chain_dep question.)*
    - **S2 entry gate (the cluster's load-bearing gate):** **No live call to `select_best_chain` may be
      introduced until candidate construction proves:** (1) peer identity preserved; (2) hash-critical protocol paths use
      preserved original wire bytes, internal candidate comparison/proof surfaces use project-canonical bytes; (3) candidate fragments are derived from Ade validation,
      **not** peer claims; (4) candidate ordering is deterministic; (5) malformed / missing candidate data
      fails closed. *This is the line that keeps the cluster from becoming "look over the peer's
      shoulder."*
  - **S3 live selector dispatch** — invariant: `NeedsForkChoice` routes the candidate set to the existing
    `select_best_chain`; order-independent; tiebreaker-loss = no-op — addresses CE-AO-3 — TCB: RED/GREEN +
    BLUE-reused. *(Resolves the buffer-vs-reselect half of OQ-SELECT-3.)*
  - **S4 selected-range fetch + fork-switch apply (the security-critical slice)** — invariant: **Ade must
    never commit rollback of its current durable chain until the replacement branch's bodies are fetched,
    linked, and validated as a complete candidate branch**; the winner is then adopted via
    `RolledBack(fork_anchor)+ChainSelected×N` through the existing apply arms, `ForkChoiceWin` constructed
    live, the fork anchor canonically bound to Ade's durable stored slot+hash — addresses CE-AO-4 — TCB:
    RED (fetch) + GREEN (sequencing + canonical anchor binding) + BLUE-reused. *(Resolves OQ-SELECT-3;
    isolated for its own focused security review — the H-1 class at fork-choice scale.)*
  - **S5 reselection replay-equivalence + reconcile + forge-fence** — invariant: `ForkChoiceWin`
    reselection WAL-recorded + replay-equivalent; `selector == durable`; no forge across pending
    reselection — addresses CE-AO-5 — TCB: BLUE-reused + GREEN/RED.
  - *(operator pass, post-S5)* **CE-AO-6 live convergence** — addresses CE-AO-6 — operator-gated.

- **Replay obligations:**
  - **No new BLUE canonical type expected** (hypothesis — `RollbackReason::ForkChoiceWin` already exists).
    S1's `NodeSyncItem` change is a transient RED/GREEN feed type (not persisted/hashed) → no canonical-
    type obligation.
  - **NEW replay corpus entry (S5):** a multi-peer reselection sequence — ordered multi-peer receive
    events that replay byte-identically to the same durable tip **including a `ForkChoiceWin` reselection**
    (extends the FOLLOW corpus / `DC-NODE-27`).
  - The §1 BLUE-safety proof obligation (S2/S4) is a determinism/authority gate, not a corpus addition.

- **Rule story (recorded for `/cluster-doc` — NOT declared here; registry untouched):** **flip**
  `CN-CONS-03` (declared→enforced, on the committed CE-AO-6 transcript); **strengthen** `DC-CONS-03` +
  `CN-CONS-01` (live multi-candidate), `DC-NODE-27` (`ForkChoiceWin` replay), `DC-NODE-29` (Ade-initiated
  rollback binding), `DC-CONS-20` / `DC-NODE-25` / `26` / `28` (multi-peer); **new** DC-NODE-family rules
  for peer-identity restoration, BLUE-safe aggregation, live dispatch, and fork-switch apply. Exact
  IDs/count at `/cluster-doc`.

- **Close gates:** the per-cluster security review is a hard close gate; **S4 (FC-6 / never-abandon-until-
  validated / canonical fork-anchor binding) is the most likely HIGH surface** — if found, a remediation
  slice lands (the AI-S6 precedent), not pre-planned here.

---

## Discipline honored

- **Complete-work-only / no carry-forward:** every CE-AO-1…5 is reachable by S1–S5; CE-AO-6 is operator-
  gated per the project's CE-tier doctrine (like CE-AI-6 / `CN-CONS-06`'s live half). The fork-below case
  S4 opens is fenced fail-closed before S4 — a typed scope boundary, not a placeholder (the N-AI
  `NeedsForkChoice`-fail-closed pattern).
- **Hypothesis, not claim:** "near-zero new BLUE" is a hypothesis; the §1 proof obligation is a hard S2
  entry gate, and the candidate aggregator is **not** pre-colored GREEN until S2 proves the authority
  boundary.
- **Authority boundary made explicit:** the dangerous seam (candidate construction) carries the load-
  bearing S2 entry gate; the security-critical seam (own-chain rollback) is isolated in S4 with its own
  blunt never-abandon-until-validated invariant.
