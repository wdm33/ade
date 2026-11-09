# EPOCH-CONSENSUS-VIEW — S3f-4 (scope): the live activation flip (DC-EPOCH-04..07)

> **Status:** SCOPED (2026-06-21, pre-code, user-directed). The ONE slice that changes live consensus behaviour. A SINGLE atomic, WAL-backed activation path — NOT a feature-flagged alternate consensus mode. No runtime flag, no seed-view fallback after the boundary, halt on activation failure or mismatch. Durable-before-visible, replay-identical.

## The activation sequence (the only path)
1. derive the candidate N+1 `EpochConsensusView` (window driver DC-EVIEW-10 → `form_mark_snapshot` → `EpochConsensusView::bind` with the N+1 context);
2. verify ALL bindings against the selected chain (DC-EVIEW-07 `matches`);
3. write the durable activation WAL record;
4. atomically publish the active `EpochConsensusView`;
5. ALL N+1 header validation + leadership read ONLY that active view.

## 1. WAL activation record
A distinct `WalEntry::EpochConsensusViewActivated` recording the ENTIRE activation identity (not just hash + point): `target_epoch`, `network`, `era`/protocol-version context, `transition chain point`, `source checkpoint commitment`, `snapshot phase`, `nonce commitment`, `stake-view canonical hash`, `full EpochConsensusView canonical hash`. The durable proof that THIS exact view became authoritative for epoch N+1 at THIS exact selected-chain transition.

Do NOT weaken `DuplicateProvenance` broadly. Activation idempotence is EXPLICIT:
- same target epoch + byte-identical activation record → replay / idempotent;
- same target epoch + ANY differing binding/hash → structured conflict, fail closed.

## 2. Leadership feed
The promoted `PoolDistrView` FULLY REPLACES the recovered seed view at the epoch wall. NO runtime flag (a flag = two consensus semantics for one canonical history → violates replay-first). The safe gate is the ACTIVATION PREDICATE, not a flag: `all bindings verify + WAL activation durable + selected-chain point correct + epoch transition eligible → promote; otherwise → no promotion`. Before promotion epoch-N seed view is authoritative; after, epoch-N+1 promoted view is authoritative. No "choose old or new by config" state.

## 3. Fail-closed posture (terminal, never fallback)
- WAL-record failure → HALT before promotion.
- Any post-promotion mismatch → HALT. Do NOT fall back to the seed view (it is known epoch-WRONG; "no leadership" is insufficient if header validation / follow could observe stale consensus inputs).
- Structured terminal states: `EpochViewActivationFailed`, `EpochViewActivationConflict`, `EpochViewPostPromotionMismatch`.

## Crash recovery
- crash before durable WAL → old epoch remains active;
- crash after durable WAL but before publication → recovery replays and publishes the SAME view;
- crash after publication → the recovered active view must match the WAL EXACTLY.

## Required invariants
- **DC-EPOCH-04** — for a target epoch, AT MOST ONE canonically bound `EpochConsensusView` may activate.
- **DC-EPOCH-05** — epoch N+1 validation and leadership may NOT observe epoch-N seed inputs.
- **DC-EPOCH-06** — activation is durable-before-visible and replay-identical.
- **DC-EPOCH-07** — a missing / stale / conflicting / mismatched candidate view causes TERMINAL fail-closed behaviour, NEVER fallback consensus.

## Decomposition (fail-closed order)
- **S3f-4a — the WAL activation record** (DC-EPOCH-04, DC-EPOCH-06 substrate): the `EpochConsensusViewActivated` variant + canonical encode/decode + the explicit idempotence-vs-conflict rule (same epoch byte-identical = idempotent; differing = conflict). Hermetic.
- **S3f-4b — the activation predicate + atomic publish** (DC-EPOCH-05, DC-EPOCH-07): the pure predicate (bindings verify + point correct + transition eligible), the atomically-published active-view container, the terminal `EpochViewActivation*` states. Hermetic.
- **S3f-4c — durable-before-visible + crash recovery** (DC-EPOCH-06): the WAL-before-publish ordering + the recovery replay (republish the same view; post-publication recovered == WAL). Hermetic crash tests.
- **S3f-4d — the live wiring** (the flip): block-source ChainDb → window driver → bind → predicate → publish → leadership reads the active view at the wall. GATED on the two live cardano-node proofs (the boundary-aligned stake oracle + the leadership-schedule proof) — the boundary is spent PROVING, not implementing.

## Where it sits
S3f-1 (consume point) + S3f-2-pre (cert import) + S3f-2 (window driver) + S3f-3 (rebind seam) are all built, fail-safe, inert live. S3f-4 is the atomic flip that makes the rebind seam (S3f-3) fire — a real normal-node-style epoch transition, not a test mode that can quietly stay disabled.
