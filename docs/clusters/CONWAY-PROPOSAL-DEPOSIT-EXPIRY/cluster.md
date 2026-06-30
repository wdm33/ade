# CONWAY-PROPOSAL-DEPOSIT-EXPIRY — the missing gov-proposal deposit-refund transition

> **Status:** Planning Artifact (Non-Normative). Describes how the work is organized and sequenced;
> introduces no new requirements beyond the invariant it declares (DC-GOV-01). If this conflicts with a
> normative document, the normative document wins.

**Central rule:** DC-GOV-01 (declared by this cluster). **Builds on:** LIVE-LEDGER-EPOCH-TRANSITION
(the `EpochAccumulator`, the boundary transition, CE-3d), DC-EPOCH-19..22. **Prerequisite for:**
CE-3d byte-exactness → S4 (accumulator-as-authority).

---

## 1. The gap (specific, ground-truthed 2026-06-30)

The CE-3d POST-1342 reward differential is dominated by a single missing **authoritative state
transition**, isolated to the byte via a POST-1340 cardano `db-analyser` extraction:

```
gov proposal submitted (deposit enters the deposit pot, return address recorded)
  → proposal expires unratified at an epoch boundary
  → deposit refunds to its recorded return address (a reward account)
```

Concretely at the 1340→1341 boundary: **5 unique Conway `TreasuryWithdrawals` proposals** (submitted
epoch 1309, `expires_after=1339`, `deposit=100,000 ADA` each) expired unratified. Their deposits
refunded to two reward accounts — **4 → `00ceb134…` (+400,000 ADA), 1 → `00f53256…` (+100,000 ADA)** —
exactly the −400B/−100B lovelace gap (the whole −500,037,651,836 reward differential bar a ~−37.6M
rounding tail). Cardano's **treasury and reserves are untouched** by this (the refund source is the
deposit pot, not a pot in the CE-3d comparison): cardano `ref@1340` vs Ade `acc@1340` differ by only
+231M/+894M (the stable B3c residual), and cardano's treasury Δ(1340→1341) ≈ Ade's within 320k. So the
mechanism is **deposit refund on expiry**, not a treasury withdrawal.

Ade's accumulator carries `gov_state = None` (the bootstrap decoder `skip_item`s the gov-state
Proposals + Committee), so it tracks no proposals/deposits and never refunds. **This transition must
land before the remaining reward differential can be interpreted** — otherwise CE-3d compares an
accumulator that is missing known account balances.

---

## 2. The invariant (blunt)

> **DC-GOV-01.** Ade refunds a removed governance proposal's deposit to its recorded return address
> **only when it can prove, from canonical governance state and the Conway rules, that the proposal
> could not have ratified or enacted.** Otherwise it fails closed (terminal structured failure). The
> refund (deposit-pot debit + return-address credit) is a total, deterministic, replay-equivalent
> boundary transition; the proof is canonical, persisted-or-reproducibly-derivable, and testable.

This is a **negative-proof** invariant. It closes the observed expiry-refund defect **without** claiming
ratification or enactment: Ade never decides that a proposal *ratifies*, only proves when one *cannot*.

### Scope — what this cluster OWNS

- imported pending proposals + their deposit / return-address / expiry identity (manifest-bound);
- newly submitted proposal procedures (live, from canonical tx bytes — field 20);
- exact expiry detection at the boundary;
- exact deposit-pot debit + return-address credit;
- the **negative ratifiability proof** that gates every refund.

### Scope — what this cluster does NOT claim (keeps it honest)

vote *processing* (tally/apply as a live ratification engine) · positive ratification · enactment ·
treasury withdrawals · parameter enactment · committee/DRep transitions. The negative proof READS the
canonical vote maps as input (vote *capture* is in scope by "prove from canonical governance state");
it never enacts.

### The decision table (binding)

| Proposal at boundary | Disposition |
|---|---|
| Unvoted, expiring (`expires_after < ending_epoch`) | **refund** |
| Voted but **provably** unable to ratify (committee quorum and/or DRep threshold structurally unreachable, given real boundary-era membership) | **refund** |
| Voted and **potentially** ratifiable | **terminal structured failure** |
| Already ratified / enacted / postponed / otherwise unresolved | **terminal structured failure** (until that governance transition is modeled) |
| Malformed / unsupported proposal or vote representation | **terminal structured failure** |
| Not expiring, not ratifiable | carry forward (no action) |

The proof reuses the EXISTING `governance.rs::check_ratification`: at a proposal's expiry boundary its
votes are final, so `check_ratification(…, ending_epoch) == false` with COMPLETE canonical inputs is the
proof it cannot ratify. `gov_action_threshold_index` already encodes the per-action required bodies
(TreasuryWithdrawals ⇒ constitutional-committee quorum + DRep[9], no SPO). **Do not** hard-code
"0 committee Yes = impossible"; the evaluator reads the real committee membership + quorum + thresholds.

### Binding contract (refined by user 2026-06-30)

- **Structured canonical verdict.** The evaluator returns a closed sum, never a bare bool:
  `ProvablyUnratifiable { reason }` (e.g. `MissingRequiredCommitteeApproval`) · `PotentiallyRatifiable` ·
  `UnsupportedGovernanceState` · `MalformedGovernanceState`. **Only `ProvablyUnratifiable` may enter the
  expiry-refund path**; every other verdict is terminal structured failure.
- **Identity binding.** A tracked proposal is identified by its `GovActionId`, and its state carries
  `{action_kind, deposit, return_addr, proposed_in, expires_after, canonical vote-map commitments}`.
  **Never match, expire, or refund a proposal by return address or deposit amount alone** — always by
  `GovActionId` identity.
- **Vote tripwire (S3) precedence.** The tripwire must fire on **every** canonical selected-chain vote
  field targeting **any** currently-tracked proposal, BEFORE the boundary evaluator may refund that
  proposal. A post-seed vote on a tracked proposal ⇒ terminal (the imported vote map is no longer
  canonical); no tally/ratification/enactment is claimed.
- **No silent skips in the decoder.** An unknown `GovActionState`/`GovAction` variant or an unsupported
  committee representation is **terminal for this authority path** (`UnsupportedGovernanceState` /
  `MalformedGovernanceState`), NEVER coerced to an empty/default. An absent proposal/committee set is
  **not** semantically equal to an empty one.

---

## 3. What already exists vs. the gap (verified via 3 code maps, 2026-06-30)

| Surface | Status |
|---|---|
| `EpochAccumulator.gov_state: Option<ConwayGovState>` | ✅ field exists (epoch_accumulator.rs:136) |
| Accumulator codec persists `gov_state` | ✅ wire array(11) idx 6 = `encode_gov_state` (v1) |
| `ConwayGovState` (proposals, committee cold→expiry, quorum, drep_expiry, vote_delegations, thresholds, hot_keys, gov_action_lifetime) | ✅ all fields exist (state.rs:81-106) |
| `GovActionState` (3 vote maps, deposit, return_addr, proposed_in, expires_after) | ✅ exists |
| `check_ratification` / `evaluate_ratification` / `gov_action_threshold_index` | ✅ exists (governance.rs) |
| Boundary computes drep_stake + has go/committee/thresholds | ✅ rules.rs:1263-1299 |
| `decode_proposal_procedures` / `decode_gov_action` (tx field 20, typed) | ✅ ade_codec/conway/governance.rs |
| **Bootstrap decoder reads gov-state Proposals (idx 0) + Committee (idx 1)** | ❌ `skip_item`'d (ledgerdb_state.rs:1245) → gov_state None |
| **`NativeSnapshotNonUtxoState` carries proposals + committee + commitment** | ❌ no field; commitment `…-v5` |
| **Ledger-state `Proposals` OMap → `Vec<GovActionState>` decoder** (with votes) | ❌ new (tx-body ProposalProcedure has no votes) |
| **Within-epoch fold captures tx field 20 (proposals) + field 19 (votes)** | ❌ skipped at `read_one_tx_field` default arm |
| **field 19 `voting_procedures` decoder / tripwire** | ❌ opaque, no decoder |
| **Boundary deposit-expiry-refund evaluator** | ❌ none |

---

## 4. Slice decomposition (maps the user's 7-step sequence)

> Ordering reflects dependency + safety. A slice is not complete until it meets the exit criteria
> incrementally and is replay-verifiable.

- **S1 — Manifest-bound bootstrap proposal + committee import.** Decode gov-state Proposals (idx 0 =
  `array(2)[GovRelation(skip), indefinite-array of GovActionState]`) + Committee (idx 1 = `SJust
  array(2)[map{cred⇒epoch}, UnitInterval]`) in `decode_native_nonutxo_state`; add to
  `NativeSnapshotNonUtxoState` (identity-bound `Vec<GovActionState>` + committee map + quorum); thread
  through assembly into `ConwayGovState` so the accumulator seeds a populated `gov_state`. New
  ledger-state `GovActionState` + `Committee` CBOR decoders (reuse `decode_gov_action` for the
  `procedure.gov_action`). **Unknown variant / unsupported committee repr ⇒ terminal**
  (`UnsupportedGovernanceState`/`MalformedGovernanceState`), never an empty default. Verified by
  decoding the real POST-1340 state byte-exactly: exactly 5 expiring `TreasuryWithdrawals` proposals
  (deposit 100k ADA, return addresses acct1×4 / acct2×1, `proposed_in=1309`, `expires_after=1339`, the
  one `[0,2,0]` vote). Declares **DC-GOV-01**.
- **S2 — Version the seed codec; fail-closed re-bootstrap error. [DONE]** The S1 commitment is already
  v6 (binds the imported proposals + committee). S2 adds the LOAD-PATH gate (absent ≠ empty): a sealed
  Conway+ bootstrap baseline whose `gov_state` is `None` PREDATES the import (a pre-v6 store) and is
  rejected — it is NEVER loaded as "zero proposals." `gov_state = None` is uniquely the no-import case
  (a v6 bootstrap always seals `Some(..)`, even with an empty pending-proposal set). Mechanism:
  `EpochAccumulatorStore::verify_governance_imported` → `AccumulatorReadinessError::
  GovernanceImportRequired`; the warm-start store-open in `node_lifecycle` maps it to the TYPED terminal
  `NodeLifecycleError::AccumulatorPredatesGovernanceImport` (exit `EXIT_NODE_WARM_START_RECOVERY_FAILED`,
  same class as the ECA-2-pre old-sidecar gate; DISTINCT from a corrupt store, which stays non-fatal),
  with a clear startup error. **Release note: a v5/pre-import bootstrap store requires re-bootstrap from
  the certified snapshot to import the gov proposals + committee.** Tests:
  `epoch_accumulator_store::tests::governance_import_gate_rejects_absent_but_allows_empty`.
- **S3 — Capture live proposal procedures + a vote tripwire + the expiry-lifetime authority. [DONE]**
  A DEDICATED within-epoch governance pass `apply_block_governance` in `epoch_accumulator::apply_within_
  epoch` (NOT an extension of the fee scan / `read_one_tx_field`, the planning shorthand: forming a live
  proposal's `GovActionId` needs the tx-body hash, the block epoch, and the `gov_state` to merge into —
  none of which `TxScan` carries; the pass mirrors `process_block_certificates` as its own tx-body walk,
  leaves the fee scan + the SHARED `rules.rs` untouched, and reuses the already-decoded phase-2-`invalid`
  set as the authority-effect gate). Per VALID tx, in tx order: (1) field 20 (`decode_proposal_
  procedures`) → each becomes a tracked `GovActionState` with `action_id = (transaction_id(body),
  proc_index)` (`block.tx_bodies` are raw wire bytes ⇒ the txid matches cardano byte-for-byte),
  deposit/return-addr/gov-action verbatim, `proposed_in = ctx.block_epoch`, `expires_after = proposed_in
  + gov_action_lifetime`, EMPTY vote maps; (2) field 19 → `extract_voted_action_ids` (the inner-map
  GovActionId keys, mirroring `required_signers::collect_voter_keys`) — any targeting a tracked proposal,
  incl. one just submitted this tx, ⇒ terminal `VoteOnTrackedProposal` (no tally/apply). Fail-closed
  terminals: invalid-tx carrying field 19/20 → `InvalidTxCarriesAuthorityEffect`; malformed field 19/20 /
  unknown gov-action → `MalformedGovernanceField`; `gov_state = None` ⇒ governance untracked, pass skipped.
  **Expiry-lifetime authority (closes S3's own timing authority):** `expires_after` is persisted future
  refund authority, so `gov_action_lifetime` must NOT be a default. It is now IMPORTED from the certified
  `curPParams` (index 26, previously skipped) into `ImportedGovState.gov_action_lifetime`, SEEDED into the
  accumulator's `gov_state` (was a hardcoded `0`), and the capture path refuses a `0` (un-imported)
  lifetime → terminal `GovActionLifetimeUnproven` rather than fabricate `expires_after = proposed_in`. The
  bootstrap commitment binds it (`v6`→`v7`) as FRESH-BOOTSTRAP tamper-evidence — a tampered lifetime in the
  certified snapshot flips the digest; it is NOT a warm-start load gate (a pre-S3 durable store recovers
  `0` via the unchanged accumulator codec and fail-closes at the runtime capture guard, which fires exactly
  where the lifetime is consumed — it is consumed nowhere else). DESIGN NOTE (per-slice review): S3 adds a
  NEW deterministic-halt surface to the proven follow path — a real selected-chain block that votes on a
  seed-imported proposal now halts the advance with a structured `Err` (not a panic); operators/runbooks
  should expect the deterministic halt, not read it as a regression. Tests: 13 capture/tripwire/guard cases
  in `epoch_accumulator` + the import-link (hermetic `imported_gov.gov_action_lifetime == 6`), the seed-link
  (assembly), the v7 commitment-binding, and the lifetime-0 terminal. Gate: `ci/ci_check_gov_proposal_
  capture.sh`. Per-slice IDD/security review: no BLOCK. Full field-19 vote capture remains a documented
  escalation if CE-3d fails closed at S5.
- **S4.0 — Ratification census (read-only evidence; admissibility gate for the narrow S4). [DONE]**
  Before any S4 mutation exists, prove over the WHOLE tracked set at the exact 1340→1341 boundary whether
  Ade's CURRENT (committee-only) ratification authority is sufficient — i.e. every tracked proposal is in
  a SAFE terminal category (`PresentGateFailed` = provably unratifiable via a present, failed gate, or
  `InfoActionNeverEnacts`) and NONE is `PotentiallyRatifiable` (would need un-imported DRep/SPO threshold +
  stake) or `Malformed`. A new observe-only `governance::proposal_ratification_observation` exercises the
  REAL `check_ratification` (shares the extracted `active_drep_stake_filtered` preamble with
  `evaluate_ratification` — meaning-preserving, regression-pinned; the observer's verdict comes solely from
  `check_ratification`, never a second implementation) — GREEN/test authority, no mutation, no runtime
  dependency. The census (`ade_testkit/tests/cpde_s4_0_ratification_census.rs`, `#[ignore]`) emits a
  canonical, GovActionId-sorted, state-bound report retained as committed evidence
  (`cpde-s4-0-ratification-census.txt`, reproduced byte-for-byte). **RESULT — CENSUS CLEAN:** all 50
  proposals resolve (46 PresentGateFailed + 4 InfoActionNeverEnacts; 0 PotentiallyRatifiable; 0 Malformed);
  the committee is active 3/8 at 1340 (gate fires); no proposal reaches the DRep/SPO gates ⇒ the empty
  thresholds are never consulted ⇒ **NO threshold/DRep-stake import gap is needed for CE-3d**; the 5
  expiring `TreasuryWithdrawals` are exactly the refund set. Per-slice BLUE review: no BLOCK. ⇒ the narrow
  S4 committee-gate evaluator is ADMISSIBLE.
- **S4 — Boundary deposit-expiry-refund evaluator (the transition). [DONE]** A whole-set PURE planner
  `governance::plan_deposit_refunds` returning `Result<RefundPlan, RefundVerdict>` — for EVERY proposal it
  composes expiry (`expires_after < new_epoch-1`) with the REAL `check_ratification` (the SEAM: refund only
  an EXPIRED **and** provably-unratifiable proposal; never the observer's expiry-free flag). Any
  potentially-ratifiable / malformed / unsupported proposal ⇒ the terminal `RefundVerdict`; the planner
  mutates nothing. `epoch_accumulator::apply_gov_deposit_refunds` (wired into `cross_epoch_boundary` BEFORE
  the boundary view — it has the `Result` channel) builds the canonical inputs from `gov_state` + the
  pre-rotation snapshots, runs the planner, and — ONLY on a fully-safe plan — applies it ATOMICALLY: a
  REGISTERED return-address credential is credited in `cert_state.delegation.rewards`, a DEREGISTERED one
  routes to TREASURY (cardano's unredeemed path, matching POOLREAP + reward distribution — never an orphan
  `rewards` entry), and the proposal is removed, in GovActionId order. Implicit pot (removal IS the debit;
  Σcredits == Σremoved). A non-safe verdict ⇒ `LedgerTransitionError::GovDepositRefundTerminal` with ZERO
  mutation. Removing the expired set here makes the boundary fn's own step-4b expired-drop a no-op (no
  double-refund / double-drop). InfoAction never enacts ⇒ refunded on expiry only if it carried a deposit.
  Total, deterministic, replay-equivalent. **Per-slice BLUE review:** one BLOCK (unregistered return
  account credited unconditionally → treasury divergence) FIXED via the POOLREAP treasury-routing above;
  else clean. **Operational envelope (documented, not a bug):** Ade does not model enactment, so the FIRST
  genuinely potentially-ratifiable governance proposal halts the boundary deterministically (zero mutation)
  — the fail-closed posture, surfaced for runbooks. **Deferred to S5:** the refund credit enters the stake
  *distribution* only at the next boundary's mark (the reward-account balance is immediate, correct for the
  reward differential); the exact SNAP-vs-refund interleave is confirmed by the CE-3d byte-exact rerun.
  Tests: 8 planner decision-table + 4 boundary (refund-to-rewards, deregistered→treasury, terminal-zero-
  mutation, replay-equivalent). Enforces DC-GOV-01.
- **S5 — CE-3d re-run → byte-exact reward total.** Re-run the CE-3d differential; the +400k/+100k
  refunds land, the reward total matches cardano (bar the separate −343B B3c go-stake and the ~−37.6M
  rounding tail). Hands off to the B3c UTxO investigation (separate).

---

## 4b. First commit scope (S1 + the bootstrap-authority correction it rides on)

The S1 commit is ONE coherent bootstrap-authority correction: the certified native bootstrap now imports
**all** historical boundary inputs the node needs after the seed point — the certified fee pot, the
bootstrap RUPD, and the pending governance proposal/committee state — and seeds the `EpochAccumulator`
with canonical, commitment-bound state. Because these inputs interlock at the seed-adjacent boundary,
the change spans `ledgerdb_state.rs`, the native assembly, accumulator seeding, and bootstrap-RUPD
handling; that span is evidence of ONE correction, not unrelated work. The commit bundles: the fee-pot
decode + threading; the leader/owner-stake (`op_share`) full-go-stake revert; the bootstrap-RUPD
application/binding; the S1 governance import + v6 commitment + fixture/fail-closed tests; this doc.

**Zero-double-count invariant (stated + tested).** The imported fee pot (epoch (seed)'s fees) and the
bootstrap RUPD (epoch (seed-1)'s reward + `deltaF`) are DISTINCT historical epochs' fee contributions;
the seed-adjacent boundary may never count the same fee twice. At the seed boundary the native reward
draws ZERO fees, the pots move by EXACTLY the RUPD's deltas, and the imported pot rotates to
`prev_epoch_fees` intact — consumed exactly once at the NEXT boundary. Test:
`epoch_accumulator::tests::imported_fee_pot_not_double_counted_with_bootstrap_rupd_at_seed_boundary`.

**What this commit does NOT establish (explicit).** It does **not** complete CE-3d and does **not** make
S4 (accumulator-as-authority) eligible. It establishes COMPLETE, canonical, bound imported bootstrap
inputs. Still open: S2 (the absent≠empty re-bootstrap codec gate), S3 (live proposal capture + the
field-19 vote tripwire), S4 (the boundary deposit-expiry-refund evaluator), S5 (the CE-3d reward
byte-exact re-run), and — separately — the −343B B3c base-UTxO stake residual.

## 5. Exit criteria (CE) — the cluster is NOT closed until ALL are mechanically green

- **CE-1 (import):** a bootstrap from a real preview Conway `state` populates `gov_state` with the exact
  pending proposals (count, deposits, return addresses, expiries) + committee; a hermetic test asserts
  the imported set matches the decoded reference, and the v6 commitment is manifest-bound + restart
  byte-identical.
- **CE-2 (codec fail-closed):** a v5/v1 store is rejected at load with the terminal re-bootstrap error;
  a round-trip test proves v6 encode/decode byte-identity.
- **CE-3 (negative proof, total):** the evaluator is total over the decision table — unit tests for each
  row (unvoted-expire→refund; voted-but-committee-unreachable→refund; voted-potentially-ratifiable→
  terminal; ratifiable→terminal; malformed→terminal), including the discriminated committee/DRep
  resolution. The proof for a refund is reproducible from the persisted canonical state.
- **CE-4 (transition, replay-equivalent):** a hermetic multi-block + boundary sequence refunds the
  expiring deposits and is byte-identical on re-run; the deposit-pot debit + reward credit conserve.
- **CE-5 (CE-3d byte-exact reward):** the live CE-3d differential's reward total matches the
  cardano-node at the 1340→1341 (and 1341→1342) boundaries — the +400k/+100k accounts byte-exact, the
  remaining gap confined to the documented B3c go-stake + rounding tail.

**Entry:** CE-3d gap ground-truthed to deposit refunds (done, 2026-06-30).
**Exit:** CE-1..CE-5 green in CI + the CE-3d reward total byte-exact.
