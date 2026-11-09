# S4 — Deterministic RATIFY: the activation boundary

## Goal

Turn the ratification gate from INERT to LIVE on the authoritative native boundary, and REPLACE the CPDE
`PotentiallyRatifiable` terminal with a deterministic ratify-then-enact decision — oracle-verified against
the enactment census. This is the deliberate reversal of import-not-activate ([[feedback_imported_data_is_not_activation]]):
S1 imported the authority (commitment-bound, oracle-ground-truthed); S4 threads it into the live gate WITH
oracle verification.

## The live boundary (surveyed 2026-07-01)

The native Mithril node's boundary is the **EpochAccumulator** path, NOT `rules.rs::evaluate_ratification`
(that is the full-ledger replay). `epoch_accumulator.rs::apply_gov_deposit_refunds` (:441) calls
`governance::plan_deposit_refunds` (:467) with `acc.gov_state`'s gate fields — which
`mithril_native_assembly.rs:381-386` seeds EMPTY (import-not-activate). So the gates are skipped for absent
inputs, and any voted proposal returns `Err(RefundVerdict::PotentiallyRatifiable)` → the whole boundary halts
(the CPDE continuity blocker). `plan_deposit_refunds` refunds ONLY a proposal that is EXPIRED **and**
provably-unratifiable; a potentially-ratifiable one halts.

## Decomposition (ordered; the activation flip is gated on the oracle anchor)

- **S4.1 — `num_dormant` offset (safe prerequisite).** `ConwayGovState` has no `num_dormant` field; add it
  (seeded from `imported_gov.num_dormant_epochs`), and change `active_drep_stake_filtered` to test
  `drep_expiry + num_dormant >= current_epoch` (cardano's active-DRep rule). INERT until S4.2 threads a
  non-empty `drep_expiry`, so it flips nothing on its own. Also dedup the second inline DRep-stake loop in
  `epoch_accumulator.rs:456-461` onto S3's `derive_drep_voting_stake`.
- **S4.4-first — the ORACLE ANCHOR (build BEFORE flipping).** The enactment census proves 69c948cd..#0
  ratified→enacted at the 1095→1096 boundary. The census states carry each proposal's accumulated
  committee/DRep/SPO votes + the real thresholds (curPParams 22/23) + `vote_delegations` + the mark snapshot.
  So decode the epoch-1095 state, derive the DRep stake, run the REAL `check_ratification` on 69c948cd..#0,
  and assert it decides RATIFIED (and 1094 = not-yet). This anchors the activation: if Ade's gate does not
  reproduce the oracle's ratify decision, S4.2/S4.3 do NOT flip until the gate is correct.
- **S4.2 — thread the imported authority into the live gate.** `mithril_native_assembly.rs`: seed
  `gov_state.{drep_expiry, vote_delegations, pool_voting_thresholds, drep_voting_thresholds,
  committee_hot_keys, num_dormant}` from `imported_gov.*` instead of empty. THIS is the activation.
- **S4.3 — replace `PotentiallyRatifiable` with ratify-then-enact.** A ratified proposal is REMOVED and its
  deposit refunded on ratification (from the deposit pot to the return addr) — not a halt. **SPO-sequencing
  (critical):** the SPO gate folds No-stake into the denominator (NON-MONOTONE), so a captured SPO No must
  NOT drive a wrongful refund. The refund path must remain "expired ∧ provably-unratifiable"; a ratified
  proposal takes the enact path, never the refund path. The effect APPLICATION (param/committee/constitution
  write-back) is S5; S4 delivers the ratify DECISION + removal + deposit-return.

## Invariants (registry candidates)

- Ade's ratify decision == the cardano oracle for every proposal in the census window (the ratify gate is
  correct BEFORE it is activated).
- `active_drep_stake_filtered` applies the `num_dormant` offset (denominator matches cardano's active-DRep set).
- No wrongful refund: a proposal that is potentially ratifiable (any gate, incl. the non-monotone SPO gate)
  is NEVER refunded; only EXPIRED ∧ provably-unratifiable proposals are.
- Determinism: the whole boundary is a pure function of canonical inputs; ratify order is `GovActionId` order.
- The activation is deliberate + oracle-verified (not a silent flip); import-not-activate is reversed only
  with the oracle anchor green.

## Acceptance (CE)

- S4.1: `num_dormant` on `ConwayGovState`, applied in `active_drep_stake_filtered`; accumulator uses
  `derive_drep_voting_stake`; byte-identical live path (still-empty inputs); `cargo test -p ade_ledger` green.
- S4.4: the census-anchored ratify test decides 69c948cd..#0 ratified@1095 / not-yet@1094 with the REAL gates.
- S4.2/S4.3: the live gate is fed the imported authority; `plan_deposit_refunds` (or successor) enacts a
  ratified proposal (removal + deposit-return) instead of the `PotentiallyRatifiable` terminal; the
  no-wrongful-refund invariant holds; oracle-verified on the census; whole crate green.

## Deferred

- Byte-exact effect application (param/committee/constitution/treasury write-back) + full deposit accounting → **S5**.
- The full byte-exact oracle differential across all action kinds + canonical history → **S6**.
