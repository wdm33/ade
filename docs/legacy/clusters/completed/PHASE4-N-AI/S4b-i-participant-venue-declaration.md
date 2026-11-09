# Invariant Slice AI-S4b-i — Participant venue declaration (inert)

> Slice of PHASE4-N-AI. AI-S1/S2/S3/S4a closed + pushed (`30b5727c`). **RED config plumbing;
> TRULY INERT** — recognizes an explicit venue value only; no live behavior flip. The behavior
> flip is AI-S4b-ii.

## 2. Slice Header
- **Slice Name:** Explicit, closed Participant venue declaration (`VenueRole::Participant` via CLI).
- **Cluster:** PHASE4-N-AI. **Status:** Merged.
- **Role:** Resolves **OQ-5** (venue is explicit + closed; `Unknown`/absent fails closed; no
  silent inference). A **precursor** for CE-AI-2 (live, AI-S4b-ii) — not a CE prover; it proves no
  fork-choice/forge behavior.
- **Dependencies:** AI-S2 (the `VenueRole::Participant` variant already exists).

## 4. Intent
Make the venue an **explicit, closed declaration** so the AI-S4b-ii fork-choice path can never be
reached by silent inference: `--participant-venue` → `VenueRole::Participant`;
`--single-producer-venue` → `SingleProducer`; neither → `Unknown` (the conservative
non-fork-choice default); both → fail closed. The declaration changes **nothing else** — it is a
label the live loop does not yet act on.

**Authority model (kept clean):** `Unknown` = undeclared / conservative fail-closed;
`Participant` = an explicitly declared, distinct venue whose fork-choice behavior arrives in
AI-S4b-ii. The two are NOT semantically equivalent — `Participant` only *reaches the same live
fallback* as `Unknown` **until** AI-S4b-ii wires Participant-specific routing, because no live
branch consumes it yet.

## 5. Scope
- **Modules:** RED `ade_node::cli` (the `--participant-venue` flag + mutual-exclusivity
  validation) + `ade_node::node_lifecycle` (`ForgeActivation::declare_participant_venue` + wiring
  the flag).
- **Out of scope (all AI-S4b-ii):** any fork-choice routing, `apply_chain_event` /
  `classify_receive` / `resolve_disposition` / `process_stream_input` call, `in_spine`,
  forge-decision change, the DC-NODE-28 forge gate.

## 6. Execution Boundary (TCB color)
- **RED:** `ade_node::cli`, `ade_node::node_lifecycle` (the setter + the one wiring line). No
  BLUE, no GREEN.

## 7. Invariants Preserved (registry IDs)
`DC-NODE-20` (SingleProducer fence — byte-unchanged), `DC-NODE-15` (Unknown → catch-up gate
unchanged), `DC-NODE-18` (extend fence unchanged), `DC-CONS-03` (fork-choice — untouched; not
reached). Every existing live venue check is `== VenueRole::SingleProducer`, so until AI-S4b-ii a
declared `Participant` reaches the **same existing live fallback** as `Unknown` (no live branch
consumes `Participant` yet).

## 8. Invariants Strengthened / Introduced
- **Resolves OQ-5** — the venue becomes an explicit, closed declaration with a fail-closed
  default and fail-closed on contradictory flags. No new registry rule (it makes
  `VenueRole::Participant`, introduced by AI-S2 for `DC-NODE-24`, operator-declarable). Precursor
  to the AI-S4b-ii live wiring (CE-AI-2 / CE-AI-3 / CE-AI-4).

## 9. Design Summary
- **CLI:** add `--participant-venue` (bool) → `Cli::participant_venue`. **Mutual exclusivity:**
  `--single-producer-venue` + `--participant-venue` together → a typed `CliError` (a venue is
  exactly one role; never both). Neither → `Unknown`.
- **Setter:** `ForgeActivation::declare_participant_venue(&mut self) { self.venue_role = VenueRole::Participant; }`
  (sibling to `declare_single_producer_venue`).
- **Wiring:** in `node_lifecycle`, alongside the existing `if cli.single_producer_venue { … }`,
  add `if cli.participant_venue { activation.declare_participant_venue(); }`. Nothing else in the
  loop branches on `Participant`.
- **Inertness (the key property):** every existing live consumer (`venue_policy`,
  `single_producer_forge_decision`, `warm_start_forge_mode`, the loop's `== SingleProducer`
  fences) tests `== SingleProducer` — so **until AI-S4b-ii**, `Participant` reaches the same live
  fallback as `Unknown`. The slice adds **no** `Participant` branch that routes/forges. *(The
  AI-S2 `resolve_disposition` already maps `Participant → NeedsForkChoice`, but it is GREEN +
  hermetic — not called by the live loop until AI-S4b-ii.)*

## 10. Changes Introduced
`Cli::participant_venue` + its parse arm + the both-venues `CliError`;
`ForgeActivation::declare_participant_venue`; one wiring line in `node_lifecycle`. No new live
branch on `Participant`.

## 11. Replay / Crash / Epoch
Not applicable — config plumbing; no durable / authoritative state, no determinism surface.

## 12. Mechanical Acceptance Criteria
- [ ] `cli_participant_venue_sets_role` — `--participant-venue` → `Cli::participant_venue == true`
  → `declare_participant_venue` → `VenueRole::Participant`.
- [ ] `cli_both_venues_fails_closed` — `--single-producer-venue --participant-venue` → `CliError`
  (a venue cannot be both).
- [ ] `cli_no_venue_flag_is_unknown` — neither flag → `VenueRole::Unknown` (no default inference).
- [ ] `participant_venue_is_inert_before_live_wiring` — for `single_producer_forge_decision`,
  `venue_policy`, and `warm_start_forge_mode`, `Participant` returns the **same** result as
  `Unknown` across representative inputs (proves no behavior change *before live wiring* — not a
  semantic-equivalence claim).
- [ ] New gate **`ci/ci_check_participant_venue_inert.sh`**: `declare_participant_venue` exists +
  is wired from `cli.participant_venue`; the both-venues `CliError` exists (fail closed at CLI
  parse/startup); absence of both flags yields `VenueRole::Unknown` (no default inference);
  `node_lifecycle` contains **no** `VenueRole::Participant` arm calling `apply_chain_event` /
  `classify_receive` / `resolve_disposition` / `process_stream_input` (those are AI-S4b-ii);
  `declare_single_producer_venue` / `DC-NODE-20` path unchanged.
- [ ] `cargo test -p ade_node` green.

## 13. Failure Modes
Contradictory venue flags → typed `CliError` (fail closed at startup). No runtime failure surface.

## 14. Hard Prohibitions
No fork-choice routing; no `apply_chain_event` / `classify_receive` / `resolve_disposition` /
`process_stream_input` call; no `in_spine`; no forge-decision change; no DC-NODE-28 forge gate; no
new live branch on `VenueRole::Participant` beyond the setter; `SingleProducer` (DC-NODE-20) +
`Unknown` (DC-NODE-15) behavior byte-unchanged; no silent inference of venue from traffic or from
absence of flags; until AI-S4b-ii, `Participant` must reach the same existing live fallback as
`Unknown` (no `Participant`-specific live branch) — a temporal property, not a semantic
equivalence.

## 15. Explicit Non-Goals
Live fork-choice routing, detector-on-blocks, `in_spine`, the forge gate / pending-reselection
state (all AI-S4b-ii); the operator pass (AI-S5).

## 16. Completion Checklist
- [ ] `--participant-venue` + mutual-exclusivity `CliError` + `declare_participant_venue` + one
  wiring line.
- [ ] Inertness proven (`Participant` reaches the same live fallback as `Unknown` before live
  wiring).
- [ ] 4 tests + `ci_check_participant_venue_inert.sh` + `cargo test -p ade_node` green.
- [ ] No routing / forge / SingleProducer change; no live `Participant` branch.
