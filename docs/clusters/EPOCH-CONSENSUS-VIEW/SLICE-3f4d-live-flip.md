# EPOCH-CONSENSUS-VIEW — S3f-4d (scope): the live activation flip (DC-EPOCH-08)

> **Status:** SCOPED (2026-06-21, pre-code, user-directed). The ONE slice that changes live consensus. Established design: orchestrate at the existing epoch-boundary apply site; drive ONLY from durable ChainDB blocks for the completed source window; HALT on any terminal `EpochViewActivation*`. The live behaviour flip is gated on the two live cardano-node proofs.

## The only authority path
- **Boundary apply site:** the epoch transition already has ONE semantic moment — orchestrate there. NO second detector, NO async activation loop (two boundary authorities = timing ambiguity).
- **Durable ChainDB source only:** `checkpoint commitment + canonical durable ChainDB range = candidate EpochConsensusView`. NO peer/network read, NO files/caches/reconstructed range, NO wall-clock, NO async side channel may influence candidate contents.
- **Halt on terminal:** an activation failure ⇒ a structured terminal node state ⇒ no further N+1 admit, no further forge, NO stale-view follow. Past the wall, following/header-validation itself depends on the correct N+1 view; continuing under an unresolved activation failure risks operating on stale/missing consensus state.

## Required constraints (binding)
1. The completed source window is PINNED to the selected ChainDB lineage and the source epoch.
2. The range is COMPLETE, ORDERED, and BOUNDED; missing / duplicate / out-of-window blocks fail closed.
3. Candidate binding happens BEFORE WAL activation.
4. WAL activation succeeds BEFORE active-view publication (DC-EPOCH-06).
5. Recovery reconstructs the same active view ONLY from the activation WAL + the bound artifacts.
6. NO peer/network read, wall-clock read, or async side channel influences candidate contents.
7. Existing same-epoch behaviour remains BYTE-IDENTICAL.

## The snapshot-timing nuance (LOAD-BEARING — avoid the Mark/Set/Go off-by-one)
The durable ChainDB range MUST NOT be called "epoch N" generically. The Cardano snapshot lag means the window that produces leadership for a TARGET epoch is NOT the target epoch's own blocks. Use NAMED ROLES throughout the code + spec:
- `source_epoch` — the completed epoch whose admitted blocks the window drives over;
- `source_window_start` / `source_window_end` — the bounded, ordered durable ChainDB range;
- `snapshot_phase` — the phase the window produces (Mark, which becomes Set for leadership);
- `target_epoch` — the epoch whose LEADERSHIP reads the activated view.

**PROOF OBLIGATION (slice-entry, not a footnote):** the exact `target_epoch = f(source_epoch, snapshot_phase)` relationship (the Mark→Set leadership lag) is a typed, explicit mapping that MUST be pinned by the Cardano snapshot-timing proof + confirmed by the live leadership-schedule proof. The code carries `target_epoch` as a typed field derived by that explicit mapping — never an inline `source + k`.

## Decomposition
- **S3f-4d-1 — the source window + named roles (DC-EPOCH-08 substrate; HERMETIC, safe):** the `ActivationSourceWindow{source_epoch, source_window_start, source_window_end, snapshot_phase, target_epoch, lineage_pin}` type + the durable-ChainDB-range extraction/validation (pinned to the lineage; complete + ordered + bounded; missing/duplicate/out-of-window fail closed) + the EXPLICIT source→target mapping (the lag a proof obligation). No live behaviour.
- **S3f-4d-2 — the candidate derivation (HERMETIC):** source window → `drive_window_aggregate` (S3f-2) → `form_mark_snapshot` → `EpochConsensusView::bind` with the target-epoch context. Candidate binding BEFORE WAL (constraint 3).
- **S3f-4d-3 — the live orchestration + the flip (the live change; GATED):** at the boundary apply site, run derive→`activation_predicate`→`activate_durable_before_visible` (WAL→publish), HALT on terminal; the wall feeds `decide_epoch_rebind(admission, Some((active.promoted(), bindings)))`; warm-start `recover_active_view`. GATED on the two live proofs (the boundary-aligned stake oracle + the leadership-schedule proof) — entered only after they pass.

## Where it sits
S3f-1/S3f-2-pre/S3f-2/S3f-3/S3f-4a/S3f-4b/S3f-4c are all built + hermetic. S3f-4d wires them into a real normal-node-style epoch transition. The hermetic substrate (S3f-4d-1/2) is built now; the live flip (S3f-4d-3) and the two oracle proofs use the next real Preview boundary.
