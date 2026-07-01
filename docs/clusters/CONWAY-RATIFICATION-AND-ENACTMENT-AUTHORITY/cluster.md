# CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY — full deterministic Conway governance vs the oracle

> **Status:** Planning Artifact (Non-Normative). User-directed 2026-07-01 as the MANDATORY successor to
> CONWAY-PROPOSAL-DEPOSIT-EXPIRY: the S3 live-vote tripwire and the S4 potentially-ratifiable terminal are
> CONTINUITY BLOCKERS (a real cardano node never halts on a ratifiable proposal / live vote), to be
> REPLACED — not bypassed — with full deterministic ratify-then-enact. See
> `~/.claude/.../feedback_consensus_terminal_is_a_continuity_blocker.md`.

## 1. Central invariant (the compatibility obligation)

> For EVERY supported Conway governance action and canonical vote/proposal history, Ade produces the SAME
> ratification, enactment, deposit, governance-state, parameter, and future-epoch result as the cardano
> ledger oracle. The epoch boundary is atomic, replayable, and has exactly one authoritative result.

A derived Cardano-compatibility requirement (not a new law) under the constitution's atomic/replayable
epoch-boundary rule. It REPLACES the CPDE terminals: the vote tripwire becomes authoritative vote capture;
the potentially-ratifiable terminal becomes deterministic ratify-then-enact.

## 2. What EXISTS vs. the gaps (surveyed 2026-07-01)

Much machinery is already present in `ade_ledger::governance` — the cluster is FEED + UNBLOCK + COMPLETE +
PROVE, not build-from-scratch.

| Surface | Status |
|---|---|
| `check_ratification` gates, `gov_action_threshold_index`, `evaluate_ratification` | ✅ exists |
| `enact_proposals` — effects for ALL 7 action kinds (treasury, param, hard fork, no-confidence, update-committee, constitution, info) | ✅ exists |
| `apply_committee_enactment` (committee write-back) | ✅ exists |
| Canonical inputs — DRep/SPO thresholds (curPParams 22/23), DRep stake (vote_delegations × stake), committee_hot_keys, drep_expiry | ❌ seeded EMPTY (mithril_native_assembly.rs:376-381) — the gates are STARVED |
| Live vote capture (field-19 → proposals' vote maps) | ❌ S3 tripwire TERMINALS instead |
| Deterministic ratify at the boundary (ordering, delays, prev-action protection) | ❌ S4 TERMINALS instead |
| Enactment APPLICATION (effects → ledger; per-account treasury credit, param/committee/constitution/hard-fork) | ⚠️ effects computed, application incomplete (CE-3d note: per-account treasury credits ignored at rules.rs) |

## 3. Slice decomposition (ordered)

- **S0 — Oracle ground-truth harness (read-only). [STARTED]** Decode cardano's governance state across a
  corpus of oracle states + report the canonical lifecycle (per-epoch proposals/votes/committee +
  per-boundary ratify/enact/expire/deposit/param transitions) — the ground truth every later slice is
  gated against. `cre_oracle_govstate_lifecycle.rs` does this for the CE-3d window. **FINDING (2026-07-01):
  the local CE-3d window 1340–1342 has ONLY expiry events (the 5 TW refunds; 1341→1342 is static) — NO
  ratify/enact, NO new submissions, NO vote changes.** ⇒ a corpus spanning a REAL successful governance
  action (committee/DRep/SPO approval → ratify → enact, + a parameter/committee/constitution change) MUST
  be extracted (offline, via the AWS reference node per CLAUDE.md) before the ratify/enact slices can be
  ground-truthed. THIS IS THE GATING NEXT STEP.
- **S1 — Import the full ratification authority** (DRep/SPO thresholds + DRep stake distribution + hot keys
  + drep_expiry — commitment-bound). **Part 1 DONE (threshold IMPORT, not activation):** `read_conway_pparams`
  captures curPParams 22/23 into `ImportedGovState`, fail-closed on degenerate UnitIntervals, bound in
  commitment **v8**, ground-truthed against the real POST-1340 (pool = 5×0.51, drep = 10 CIP-1694 fractions
  via `cre_oracle_govstate_lifecycle.rs`). **The thresholds are DELIBERATELY NOT threaded into the live
  `ConwayGovState` gate.** The per-slice IDD review caught that threading them would *activate* the SPO
  ratification gate on the authoritative boundary — `check_ratification`'s SPO arm has NO active-stake guard
  (only `voted_stake > 0`, `governance.rs:299`) and its inputs (the `go` pool stake + `spo_votes`) are
  already present at bootstrap, so a go-stake undercount could flip a near-boundary ratio into a false
  rejection (the CE-3d window was safe only because it carries zero SPO votes — an accident of the corpus,
  not an invariant). So S1 imports the AUTHORITY; the ratify SEMANTIC activates deliberately in **S4** with
  oracle verification. **Part 2a DONE (vote_delegations):** `read_dstate` captures the DState-UMap drep
  field (`read_native_drep`, robust to the DRep arity variance) → `ImportedGovState.vote_delegations`,
  commitment **v9**; ground-truthed at 58,525 real delegations (all 4 DRep variants). **Part 2b DONE (the
  VState):** `read_vstate` decodes `array(3)[vsDReps, vsCommitteeState, vsNumDormant]` (was skipped) →
  `drep_expiry` (`DRepState[0]`, 8940 real DReps) + `committee_hot_keys` (variant-0 MemberAuthorized,
  inverted hot→cold, 8 real members), commitment **v10**. **S1 IS COMPLETE** — all four inputs imported +
  commitment-bound + ground-truthed, and ALL kept OUT of the live gate (import-not-activate); the ratify
  SEMANTIC activates deliberately in **S4** with oracle verification.
- **S2 — Live vote capture [DONE].** `apply_field19_votes` (epoch_accumulator.rs) replaces the CPDE S3
  detect-and-halt tripwire: it decodes the full field-19 `{ voter => { gov_action_id => voting_procedure } }`
  and applies each vote to the tracked proposal's committee/DRep/SPO vote map (voter_type 0/1→committee,
  2/3→DRep, 4→SPO; 0/1/2 = No/Yes/Abstain). Untracked-proposal votes ignored; re-votes replace; byte-aligned
  (every entry fully decoded regardless of tracked-ness); fail-closed on malformed / unknown voter-or-vote
  discriminant on a tracked proposal. The `VoteOnTrackedProposal` terminal is removed — **a live vote no
  longer halts the node** (the first CPDE continuity blocker retired). CAPTURE, not ratification: DRep/SPO
  votes inert (S1 gates inert); a captured committee vote feeds the CPDE-live committee gate → the CPDE
  potentially-ratifiable terminal (fail-safe, never a wrongful refund) until S4. Safe on the proven CE-3d
  window (no votes on tracked proposals there → no-op; the −500B closure unchanged). Live-delta ground truth
  (real vote windows) is available via a local `dba.sh` preview dump, same as S4.
- **S3 — DRep/SPO voting-stake derivation** (the InstantStake-equivalent distribution authority).
- **S4 — Deterministic RATIFY** (remove the S4-CPDE terminal): full ordering, delays, previous-action /
  reference-hash protections.
- **S5 — Deterministic ENACT** (complete the application) for every action kind + deposit accounting on
  ratification (not just expiry).
- **S6 — Oracle differential** (the byte-exact gate): per supported action + canonical history, Ade == the
  cardano oracle.

## 4. Exit

Every CE green in CI + the S6 oracle differential byte-exact for the extracted ratify/enact corpus + both
CPDE terminals (vote tripwire, potentially-ratifiable) REMOVED and replaced by authoritative semantics.

**Entry:** CPDE's −500B reward gap closed (done) + the continuity-blocker correction recorded.

## 5. Enactment census (S6 ground truth) + the unbound-observable seam

`crates/ade_testkit/tests/cre_enactment_census.rs` (`#[ignore]`, local db-analyser artifacts) is the
byte-exact ratify→enact ground truth for the Preview param-update action `69c948cd..#0` ("Increase Tx/Block
Memory Units pt1"), across epochs 1087–1103. Extracted via LOCAL `db-analyser --store-ledger` on the preview
ChainDB (NetworkMagic 2), NOT explorer metadata. The census brackets the full lifecycle from the ledger
itself: submit (action_present false→true @ **1089**) → carried/voted (1089–1095) → **enact @ 1096**, where
four facts coincide at one boundary and the scaffold asserts them together:
`maxTx/maxBlock` exec-mem 14M/62M → **16.5M/72M**; `prevPParams` still 14M/62M (flip is AT this boundary);
target leaves the proposal map; and the enacted-authority root `prevGovActionIds.pgaPParamUpdate` flips
`602d..#0` → **`69c948cd..#0`** — the ledger's OWN record attributing the enactment to the target action.

To surface that last two, the decoder (`ledgerdb_state.rs`) now reads two previously-skipped ConwayGovState
regions: prevPParams (field 4) and the `GovRelation` PParamUpdate root (Proposals field 0, element 0,
`array(4)[StrictMaybe GovActionId]`). They land on `NativeSnapshotNonUtxoState` as
`prev_max_tx_ex_units_mem` / `prev_max_block_ex_units_mem` / `enacted_pparam_update`.

**SEAM (unbound observables — record before any promotion).** These three fields are census OBSERVABLES,
NOT bootstrap authority, so they are DELIBERATELY not bound in `commit_native_nonutxo_state` (v10) — matching
the sibling `max_block_ex_units_mem` / `gov_deposit_pot`, and unlike the S1 authority inputs
(`pool_voting_thresholds` / `vote_delegations`), which ARE commitment-bound precisely because they feed the
ratify gate. If a later slice (S4/S5) ever promotes any of these from census-observable to a live
ratify/enact authority input, it MUST first commitment-bind it (bump the tag) — else it inherits an unbound,
tamper-mutable input. This is the same import-not-activate discipline S1 followed.

**Scope guard (unchanged):** the census is GROUND TRUTH ONLY. A *passed* action is a required negative
control — it has LEFT the proposal map by enactment (present→absent), so S4 must recognize it as
non-refundable/terminal, NOT "handle" it by growing into an enactment engine.
