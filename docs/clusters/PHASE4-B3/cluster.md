# Invariant Cluster — PHASE4-B3

> **Status:** Planning Artifact (Non-Normative). Authority lives in
> `docs/ade-invariant-registry.toml` and the Conway ledger spec. Produced via
> `/cluster-doc PHASE4-B3`. Slice docs (`/slice-doc B3-S<n>`) carry per-slice detail.

---

### Cluster PHASE4-B3 — Full Conway tx value-conservation accounting

**Primary invariant:**
> Every accepted Conway tx satisfies `Σinputs + Σwithdrawals + refunded_deposits ==
> Σoutputs + fee + donation + new_deposits`; imbalance, malformed/unknown cert,
> malformed withdrawals, or an unaccountable state-dependent deposit case is a
> deterministic structured reject — never a silent accept.
> (Strengthens `T-CONSERV-01` / `CN-LEDGER-07`; introduces `DC-TXV-06`, `DC-TXV-07`.)

**Normative anchors:**
- `docs/ade-invariant-registry.toml` — NEW `DC-TXV-06` (closed/total cert-deposit
  classification), `DC-TXV-07` (canonical deposit-param authority); strengthens
  `T-CONSERV-01`/`CN-LEDGER-07`, `DC-VAL-06`, `DC-TXV-03`; anchored by `T-DET-01`, `T-ENC-01`.
- `docs/planning/phase4-b3-invariants.md` — I-1..5, N-1..5, decisions D1–D5.
- `docs/planning/phase4-b3-cluster-slice-plan.md` — slice ordering, CE table, PHASE4-B4 stub.
- External: Cardano Conway ledger spec — `validateValueNotConservedUTxO` (UTXO
  `consumed == produced`); Conway CDDL `certificate` tag set (0–18). Reference oracle
  for the `DC-TXV-06` totality proof.

---

#### Entry Conditions

- **B2 closed:** `tx_validity` composer, `required_signers` closure, `phase1`
  `track_utxo` gate, positive+adversarial corpus harness, fail-closed discipline
  (`DC-TXV-01..05`, `DC-VAL-06`).
- `ade_ledger::conway::check_conway_coin_conservation` exists for the deposit-free
  spend case (B2-S4) — its `if certs.is_some() || withdrawals.is_some() { return Ok(()) }`
  early-out (`conway.rs:174`) is what this cluster removes.
- Conway governance state already present: `governance.rs` (`GovAction`,
  `GovActionState`, ratification/enactment), `state.rs` `vote_delegations` /
  `gov_action_lifetime`, `fingerprint.rs` gov-action serialization, `conway::cert::DRep`.
- Snapshot loader provides cert/pool/DRep registration + reward accounts + era params
  over the Conway-576 window (`project_snapshot_loader_done`).
- `drep_deposit`/`gov_action_deposit` exist **only** in testkit `ConwayGovParams`
  (`snapshot_loader.rs:2473-75`) — B3-S1 promotes them into canonical era-indexed params.

---

#### Exit Criteria (CI-Verifiable)

- [ ] **CE-B3-1** — `cargo test -p ade_ledger --lib pparams` PASS **AND**
  `ci/ci_check_deposit_param_authority.sh` PASS. The CI check proves no BLUE path
  sources `key_deposit`, `pool_deposit`, `drep_deposit`, or `gov_action_deposit` from
  testkit defaults, shell config, ambient `ConwayGovParams`, fallback constants, or
  non-canonical state. Deposit params are canonical **era-indexed**
  `EraProtocolParameters`/`ConwayParams` fields populated from genesis/protocol-param
  state and threaded into `validate_conway_state_backed`; cross-era pparam
  serialization golden tests updated; replay-fingerprint migration explicit.
  *(B3-S1, `DC-TXV-07`)*
- [ ] **CE-B3-2** — `cargo test -p ade_codec --test conway_cert_classification` PASS:
  decoder total over tags `0..18`; each → `NewDeposit|Refund|Neutral|ExplicitReject`;
  Conway-removed tags 5/6 → `ExplicitReject`; unknown tag / malformed cert CBOR →
  structured reject, never `Neutral`; `class_mapping_is_total`. *(B3-S2, `DC-TXV-06`)*
- [ ] **CE-B3-3** — `cargo test -p ade_codec --test conway_withdrawals` PASS: decode
  to `BTreeMap<RewardAccount, Coin>`; truncated/malformed → structured reject; Σ exact
  `i128`, no overflow false-accept. *(B3-S3)*
- [ ] **CE-B3-4** — `cargo test -p ade_ledger --test conway_conservation_full` PASS:
  full equation enforced for cert/withdrawal-bearing txs; early-out removed;
  state-dependent unaccountable cases → `UnsupportedStateDependentDepositAccounting`;
  **CE-B2-3 and CE-B2-4 remain green** (no regression, no new false-reject on the real
  corpus). *(B3-S4, strengthens `T-CONSERV-01`/`CN-LEDGER-07`, `DC-VAL-06`)*
- [ ] **CE-B3-5** — `cargo test -p ade_ledger --test conway_conservation_positive_corpus`
  PASS: every real cert/withdrawal-bearing Conway tx → Valid at `track_utxo=true` with
  real resolved UTxO + cert/pool/DRep state + canonical params; verdict stream
  byte-identical across two runs. *(B3-S5, strengthens `DC-TXV-03`, `T-DET-01`)*
- [ ] **CE-B3-6** — `cargo test -p ade_ledger --test conway_conservation_adversarial`
  PASS: value-imbalanced (deposit/refund/withdrawal-mutated), unknown-cert-tag,
  removed-tag (5/6), truncated-withdrawals, and mis-charged-pool-deposit txs →
  Invalid/structured-reject with the correct class. **No false accept.**
  *(B3-S6, strengthens `DC-TXV-03`, `DC-VAL-06`)*

> No human review may substitute for these checks.

---

#### Candidate cert classification mapping *(pending B3-S2 proof — NOT proven fact)*

The cluster plans around the table below. It is a **candidate** Conway CDDL tag→effect
mapping; it does not become fact until B3-S2 mechanically confirms tag `0..18` totality
against the Conway CDDL / reference oracle (see B3-S2 hard entry-proof obligation).

| tag | constructor (candidate) | conservation effect | state-dependence |
|---|---|---|---|
| 0 | stake_registration (legacy) | NewDeposit(key_deposit) — implicit | independent |
| 1 | stake_deregistration (legacy) | Refund(recorded deposit) | **state-dependent** (deposits map) |
| 2 | stake_delegation | Neutral | independent |
| 3 | pool_registration | NewDeposit(pool_deposit) iff pool new, else update | **state-dependent** (pools map) |
| 4 | pool_retirement | Neutral (refund at POOLREAP, not tx-time) | independent |
| 5, 6 | genesis_key_delegation / MIR | **ExplicitReject** (removed in Conway) | independent |
| 7 | reg_cert (explicit deposit) | NewDeposit(cert.deposit) | independent |
| 8 | unreg_cert (explicit refund) | Refund(cert.refund) | independent |
| 9, 10 | vote_deleg / stake_vote_deleg | Neutral | independent |
| 11, 12, 13 | (stake/vote) reg+deleg (explicit deposit) | NewDeposit(cert.deposit) | independent |
| 14, 15 | auth_committee_hot / resign_committee_cold | Neutral | independent |
| 16 | reg_drep (explicit deposit) | NewDeposit(cert.deposit) | independent |
| 17 | unreg_drep (explicit refund) | Refund(cert.refund) | independent |
| 18 | update_drep | Neutral | independent |

**D1 state-dependent boundary:** the only state-dependent cases are legacy dereg refund
(tag 1) and pool reg new-vs-update (tag 3); accountable iff state carries the
per-credential deposits map / pools map, else `UnsupportedStateDependentDepositAccounting`.
Explicit-deposit certs (7,8,11–13,16,17) are accountable from the cert itself (matching
the cert amount against the recorded deposit is a separate validity rule, out of B3).

---

#### Expected Slice Types

- **B3-S1 — Era-indexed canonical-parameter introduction.** `EraProtocolParameters`
  enum + `ConwayParams`; fingerprint/replay migration; cross-era golden tests; dedicated
  `ci/ci_check_deposit_param_authority.sh`. *State-shape slice, not plumbing.*
- **B3-S2 — Closed-grammar cert decoder + classification.** Conway-complete decoder over
  tags 0–18; total `ClassifiedCert` sum type. Produces the **shared decoder** that
  PHASE4-B4 consumes.

  **Hard entry-proof obligation (non-optional):** before any classification logic is
  implemented, enumerate Conway certificate tags `0..18` from the Conway CDDL / reference
  oracle and record, per tag: (a) tag number, (b) constructor name, (c) encoded fields
  relevant to deposits/refunds, (d) conservation effect, (e) state-independent vs
  state-dependent, (f) reference citation / fixture proving the mapping. **No
  implementation branch may be accepted from a guessed table.** The candidate table above
  is planning input only.
- **B3-S3 — Closed-grammar withdrawals decoder.** `map{reward_account→coin}`, exact Σ.
- **B3-S4 — Authoritative-guard wiring.** Deposit accounting + full conservation equation
  into the composer; early-out removal; `UnsupportedStateDependentDepositAccounting`.
- **B3-S5 — Positive replay corpus** and **B3-S6 — Adversarial corpus + mutators** under
  `corpus/tx_validity/conservation/{positive,adversarial}/`.

---

#### TCB Color Map (FC/IS Partition)

- **BLUE (deterministic, authoritative):**
  - `ade_codec::conway::cert` (NEW Conway-complete cert decoder + classification)
  - `ade_codec::conway::withdrawals` (NEW withdrawals decoder)
  - `ade_ledger::conway` (cert→deposit accounting, full conservation guard, early-out removal)
  - `ade_ledger::pparams` (NEW `EraProtocolParameters`/`ConwayParams`)
  - `ade_ledger::fingerprint` (param-shape migration — canonical encoding)
  - `ade_ledger::tx_validity::phase1` (params/state pass-through to the guard)
  - `crates/ade_types/src/conway` (new `ClassifiedCert` / error types)
- **GREEN (deterministic glue, non-authoritative):**
  - `ade_testkit` conservation corpus harness + value/cert/withdrawal mutators (positive + adversarial).
- **RED (nondeterministic shell):**
  - Snapshot/corpus/ledger-state loading (resolved UTxO, cert/pool/DRep state, canonical
    params); reference-oracle verdicts.

**Color classification: resolved.** `ade_ledger::fingerprint` is BLUE; the param-shape
change must preserve canonical determinism (a CE-B3-1 acceptance item), not introduce
shell behavior.

---

#### Forbidden During This Cluster

- Re-introducing any early-out / silent skip on cert/withdrawal-bearing txs (the bug being removed).
- Classifying an unknown or malformed cert as `Neutral`; admitting Conway-removed tags 5/6 as anything but `ExplicitReject`.
- Decoding Conway certs with the 6-variant Shelley decoder and dropping unmatched variants.
- Sourcing any deposit param from testkit/default/shell/ambient `ConwayGovParams` in the BLUE path.
- Guessing a state-dependent deposit/refund instead of structured-reject; defaulting a Conway-only param into a non-Conway semantic path.
- Accepting a classification implementation branch from a guessed (unproven) tag table.
- Positive-only corpus coverage — the adversarial corpus (CE-B3-6) is mandatory.
- `HashMap`/float/wall-clock in BLUE; re-encoding for any hash; silent fingerprint change (migration must be explicit).
- **A false accept is release-blocking.**

---

## Authority Reminder

This document is a planning aid only. All correctness rules live in
`docs/ade-invariant-registry.toml` and the Cardano ledger spec; for mechanical
acceptance, the named tests/CI. If there is ever a disagreement:

> **Normative documents + CI enforcement win.**
