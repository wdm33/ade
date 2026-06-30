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
  oracle verification. **Part 2 (VState import) NEXT:** `vote_delegations` (DState UMap field 3, skipped at
  `ledgerdb_state.rs:795`) + `committee_hot_keys` + `drep_expiry` (the VState, skipped at `:731`) — the
  inputs S4 needs to activate the DRep gate; same commitment binding.
- **S2 — Live vote capture** (replace the S3 tripwire): decode field-19 voting_procedures into the
  proposals' committee/DRep/SPO vote maps, persisted, discriminant-correct voter resolution.
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
