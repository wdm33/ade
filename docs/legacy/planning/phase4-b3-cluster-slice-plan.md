# Cluster/Slice Plan — Ade / PHASE4-B3 (Full Conway tx value-conservation accounting)

> **Status**: planning artifact, non-normative. Authority lives in
> `docs/ade-invariant-registry.toml` and `docs/planning/phase4-b3-invariants.md`.
> Produced via `/cluster-plan`. Distinct from the phase-level
> `docs/active/phase_4_cluster_plan.md`, which is **not** modified by this doc.

## Inputs
- `docs/planning/phase4-b3-invariants.md` — B3 sketch (I-1..5, N-1..5, decisions D1–D5).
- `docs/ade-invariant-registry.toml` — NEW `DC-TXV-06`, `DC-TXV-07` (declared);
  strengthens `T-CONSERV-01`/`CN-LEDGER-07`, `DC-VAL-06`, `DC-TXV-03`; anchored by `T-DET-01`.
- B2 (closed): `tx_validity` composer, `required_signers` closure, `phase1` `track_utxo`
  gate, positive+adversarial corpus harness, fail-closed discipline;
  `check_conway_coin_conservation` deposit-free case (B2-S4).
- `~/.claude/methodology/idd.md` Part I §§1–10, Part IV.

## Cluster Index (Dependency Order)
1. **PHASE4-B3** — Full Conway tx value-conservation accounting — primary invariant:
   every accepted cert/withdrawal-bearing Conway tx satisfies the full
   `consumed == produced` equation, or is deterministically rejected with a
   structured error; **the deferral early-out becomes impossible**. *(detailed below)*
2. **PHASE4-B4** — Conway cert-state accumulation fail-closed path — *(future)* —
   removes the `rules.rs:1419-1440` fail-open; **consumes** the shared
   Conway-complete cert decoder produced by B3-S2. *(stub below)*

---

## Cluster PHASE4-B3 — Full Conway tx value-conservation accounting

**Primary invariant:**
> For every accepted Conway tx — *including* those carrying certificates and/or
> withdrawals — `Σinputs + Σwithdrawals + refunded_deposits == Σoutputs + fee +
> donation + new_deposits`. An imbalance, an unknown/malformed cert or withdrawals
> field, or a state-dependent case that cannot be accounted is a deterministic
> structured reject — **never a silent accept**. A false-accept is release-blocking.

**Anchors:** NEW `DC-TXV-06` (closed/total cert-deposit classification), `DC-TXV-07`
(canonical deposit-param authority); strengthens `T-CONSERV-01`/`CN-LEDGER-07`
(conservation extended to deposit/refund/withdrawal terms), `DC-VAL-06` (removes the
`skip` early-out), `DC-TXV-03` (adds B3 negative corpus); anchored by `T-DET-01`.

**TCB partition:**
- **BLUE:** `ade_codec::conway::{cert,withdrawals}` (NEW Conway-complete decoders);
  `ade_ledger::conway` (cert classification + deposit accounting + full conservation
  guard, replacing the early-out); `ade_ledger::pparams` (NEW era-indexed
  `EraProtocolParameters` / `ConwayParams` carrying `drep_deposit`/`gov_action_deposit`);
  `validate_conway_state_backed` signature (deposit params + cert/pool/DRep state
  threaded in); `ade_ledger::tx_validity::phase1` (pass-through of params/state).
- **GREEN:** `ade_testkit` positive + adversarial conservation corpus harness +
  value/cert/withdrawal mutators.
- **RED:** corpus + ledger-state loading (snapshot-derived cert/pool/DRep state,
  resolved UTxO, canonical params); reference-oracle verdicts.

**Entry conditions:**
- B2 closed (composer, required-signers, phase1 `track_utxo`, corpus harness, fail-closed).
- `check_conway_coin_conservation` exists (deposit-free case) — the early-out at
  `conway.rs:174` is the thing this cluster removes.
- Snapshot loader exposes ledger state with cert/pool/DRep registration + reward
  accounts + era params over the Conway-576 window (`project_snapshot_loader_done`) —
  required for D1 state-dependent accounting and the `track_utxo=true` positive corpus.
- `drep_deposit`/`gov_action_deposit` currently testkit-only in `ConwayGovParams` —
  S1 promotes them into canonical era-indexed params.

**Cluster Exit Criteria (CI-verifiable):**

| CE | Check | Closed by |
|---|---|---|
| **CE-B3-1** | `cargo test -p ade_ledger --lib pparams` PASS — deposit params (`key_deposit`/`pool_deposit`/`drep_deposit`/`gov_action_deposit`) are canonical, **era-indexed** `ProtocolParameters` fields populated from genesis/protocol-param state and threaded into `validate_conway_state_backed`; cross-era pparam serialization golden tests updated; replay-fingerprint migration is explicit; `ci_check_forbidden_patterns.sh` (extended) shows no testkit/default/ambient deposit-param source in the BLUE path. | B3-S1 |
| **CE-B3-2** | `cargo test -p ade_codec --test conway_cert_classification` PASS — decoder is **total** over Conway cert tags `0..18`; each classified `NewDeposit \| Refund \| Neutral \| ExplicitReject`; unknown tag / malformed cert CBOR → structured reject, never `Neutral`; `class_mapping_is_total`. | B3-S2 |
| **CE-B3-3** | `cargo test -p ade_codec --test conway_withdrawals` PASS — withdrawals decode to `BTreeMap<RewardAccount, Coin>`; truncated/malformed → structured reject; Σ exact (`i128`, no overflow false-accept). | B3-S3 |
| **CE-B3-4** | `cargo test -p ade_ledger --test conway_conservation_full` PASS — full equation enforced for cert/withdrawal-bearing txs; early-out removed; state-dependent unaccountable cases → `UnsupportedStateDependentDepositAccounting`; **CE-B2-3 and CE-B2-4 stay green** (no regression / no new false-reject on the real corpus). | B3-S4 |
| **CE-B3-5** | `cargo test -p ade_ledger --test conway_conservation_positive_corpus` PASS — every real cert/withdrawal-bearing Conway tx → Valid under full accounting at `track_utxo=true` with real resolved UTxO + cert/pool/DRep state + canonical params; verdict stream byte-identical across two runs. | B3-S5 |
| **CE-B3-6** | `cargo test -p ade_ledger --test conway_conservation_adversarial` PASS — value-imbalanced (deposit/refund/withdrawal-mutated), unknown-cert-tag, truncated-withdrawals, and mis-charged-pool-deposit txs → Invalid/structured-reject with the correct class. **No false accept.** | B3-S6 |

**Oracle policy:** positive = on-chain inclusion (real cert/withdrawal-bearing corpus
txs are valid). Negative = value-conservation violation per the Cardano UTXO rule
(`validateValueNotConservedUTxO`); reference-node confirmation best-effort. Same
posture as B1/B2.

**Slices:**

- **B3-S1** — **Era-indexed canonical deposit parameters** (state-shape slice, *not*
  small plumbing) — *invariant:* all four deposit params are canonical, era-indexed
  `ProtocolParameters` values threaded into the validator; no ambient/testkit/default
  source; canonical encoding version explicit; cross-era replay fingerprints
  intentionally migrated — *addresses:* **CE-B3-1**; introduces `DC-TXV-07`;
  touches `T-ENC-01`/`T-DET-01` (canonical-state shape) — *TCB:* BLUE.

  **Additional acceptance (binding):**
  - `ProtocolParameters` canonical encoding version is **explicit**.
  - Pre-Conway eras either encode absence explicitly or use **era-indexed**
    `ProtocolParameters` (preferred — see design below).
  - Existing replay fingerprints are **intentionally migrated**, not silently changed.
  - **No Conway-only parameter is defaulted into non-Conway semantic paths.**
  - Cross-era pparam serialization **golden tests are updated in the same slice**.

  **Design (preferred):** era-indexed params, not one flat struct with meaningless
  Conway fields in older eras —
  ```rust
  enum EraProtocolParameters {
      Byron(ByronParams),
      Shelley(ShelleyParams),
      Allegra(AllegraParams),
      Mary(MaryParams),
      Alonzo(AlonzoParams),
      Babbage(BabbageParams),
      Conway(ConwayParams),
  }

  struct ConwayParams {
      key_deposit: Coin,
      pool_deposit: Coin,
      drep_deposit: Coin,
      gov_action_deposit: Coin,
      // ...
  }
  ```
  This keeps the canonical-state change explicit and prevents
  "field exists but is semantically irrelevant" ambiguity.

- **B3-S2** — Conway cert decoder + closed classification (**shared decoder**, also
  consumed by PHASE4-B4) — *invariant:* total over tags `0..18`; each variant →
  `NewDeposit|Refund|Neutral|ExplicitReject`; unknown/malformed → reject, never
  neutral; Conway-complete (no Shelley-decoder coverage gap) — *addresses:*
  **CE-B3-2**; introduces `DC-TXV-06`; strengthens `DC-VAL-06` — *TCB:* BLUE.
- **B3-S3** — Conway withdrawals decoder + Σ — *invariant:* full
  `map{reward_account→coin}` decode; truncated → reject; exact integer sum —
  *addresses:* **CE-B3-3** — *TCB:* BLUE.
- **B3-S4** — Full conservation guard + deferral removal — *invariant:* full equation
  wired into `validate_conway_state_backed`; early-out gone; state-dependent
  unaccountable → `UnsupportedStateDependentDepositAccounting`; existing corpora stay
  green — *addresses:* **CE-B3-4**; strengthens `T-CONSERV-01`/`CN-LEDGER-07`,
  `DC-VAL-06` — *TCB:* BLUE.
- **B3-S5** — Positive conservation corpus + replay — *invariant:* every real
  cert/withdrawal-bearing Conway tx → Valid under full accounting; replay-equivalent —
  *addresses:* **CE-B3-5**; strengthens `DC-TXV-03`, `T-DET-01` — *TCB:* GREEN + RED.
- **B3-S6** — Adversarial conservation corpus (no false accept) — *invariant:* every
  imbalanced/unknown-cert/truncated-withdrawal/mis-charged-deposit tx → structured
  reject — *addresses:* **CE-B3-6**; strengthens `DC-TXV-03`, `DC-VAL-06` —
  *TCB:* GREEN + RED.

**Forbidden during this cluster** (inherits B1/B2 + global doctrine):
- Re-introducing any early-out / silent skip on cert/withdrawal-bearing txs (the bug being removed).
- Classifying an unknown or malformed cert as `Neutral`.
- Decoding Conway certs with the 6-variant Shelley decoder and dropping unmatched variants (N-3).
- Sourcing any deposit param from testkit/default/shell/ambient `ConwayGovParams` in the BLUE path (N-4).
- Guessing a state-dependent deposit/refund instead of structured-reject (D1).
- Positive-only coverage — the adversarial corpus (CE-B3-6) is mandatory.
- `HashMap`/float/wall-clock in BLUE; re-encoding for any hash.
- **A false accept is release-blocking.**

**Replay obligations:**
- New canonical types: `ClassifiedCert` (deposit-effect sum type), decoded withdrawals
  map, `DepositAccountingError` + `UnsupportedStateDependentDepositAccounting`, any new
  `ConservationError` fields.
- **Canonical-state shape change (S1):** `ProtocolParameters` becomes era-indexed and
  Conway gains `drep_deposit`/`gov_action_deposit` — touches `fingerprint`/state
  serialization across **all** eras. S1 must keep encoding canonical, version it
  explicitly, and migrate replay fingerprints intentionally (a `T-ENC-01`/`T-DET-01`
  concern), or fingerprint replay breaks.
- New authoritative behavior: full conservation guard now runs on cert/withdrawal txs —
  changes verdicts only by rejecting previously-false-accepted imbalances; real corpus
  unaffected (CE-B3-4 guards this).
- New corpus: `corpus/tx_validity/conservation/{positive,adversarial}/`. Replay
  byte-identically; anchored by `T-DET-01`.

**Open items for `/cluster-doc PHASE4-B3`:**
- **(load-bearing)** Confirm the Conway CDDL cert tag set `0..18` and the exact
  deposit-effect of each: which carry **explicit** deposits (Conway `reg_cert`/
  `unreg_cert` tags 7/8, `reg_drep`/`unreg_drep`) vs **legacy implicit** (tags 0/1,
  param-sourced) vs **neutral** (delegation, pool retirement, committee auth/resign,
  vote-deleg). This is the `DC-TXV-06` proof obligation.
- **Precise D1 boundary:** which effects are state-dependent →
  `UnsupportedStateDependentDepositAccounting` vs accountable. Candidates: pool
  re-registration (no new deposit if pool already registered, N-5); legacy stake-dereg
  **refund** = deposit recorded at registration (needs the deposits map from state);
  Conway explicit-unreg refund must *match* the recorded deposit.
- How the snapshot loader populates the promoted era-indexed params (genesis vs
  protocol-param-update provenance) without breaking fingerprint/replay.
- `treasury_value` (key 21) no-conservation-weight reference-confirmation (D5).
- Confirm the snapshot loader exposes resolved UTxO + cert/pool/DRep state for the
  Conway-576 window at `track_utxo=true` (CE-B3-5 prerequisite).

---

## Cluster PHASE4-B4 — Conway cert-state accumulation fail-closed path *(future)*

> **Superseded for planning by `docs/planning/phase4-b4-cluster-slice-plan.md`.**
> Authority remains `docs/ade-invariant-registry.toml` and the PHASE4-B4
> cluster/slice docs. This stub is retained for history only — do not follow it
> for B4 scope.

> Registered now so the `rules.rs:1419-1440` fail-open is not lost as a loose note.
> Scoped **out** of B3: B3 owns value-conservation accounting; this path is
> cert-state accumulation. Folding it into B3 would blur authority and make B3-S4
> less mergeable. B4 **consumes** the shared Conway-complete cert decoder from B3-S2 —
> no duplicated decoder work.

**Primary invariant:**
> A malformed, unknown, or Conway-incomplete certificate path may not be swallowed
> during cert-state accumulation. Cert-state construction either produces complete
> canonical state or a deterministic structured reject.

**Scope:**
- `rules.rs:1419-1440` fail-open removal (no "non-fatal during replay" swallow).
- Shared Conway-complete cert decoder from B3-S2 (consumed, not re-implemented).
- No Shelley 6-variant decoder on Conway cert-state paths.
- Replay corpus for malformed/unknown Conway certs.

**Depends on:** B3-S2 (shared decoder). Anchors TBD at `/cluster-doc PHASE4-B4`
(likely strengthens `DC-VAL-06`, a new cert-state-closure rule paralleling `DC-TXV-06`).

---

## Authority reminder
Planning aid only. Authority for rules belongs to `docs/ade-invariant-registry.toml`;
for mechanical acceptance, the named tests/CI. If this plan conflicts with the registry
or normative specs, those win.
