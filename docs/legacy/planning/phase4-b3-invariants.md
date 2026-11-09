# PHASE4-B3 — Full Conway tx value-conservation accounting — Invariant Sketch

**Concept slug:** `phase4-b3`
**TCB:** BLUE authoritative ledger validation (codec decoders + ledger equation + param threading); GREEN test harness only. No RED in scope.
**Status:** invariants phase (pre-`/cluster-plan`).

## Framing — this is removal of a known false-accept path, not "add two terms"

`check_conway_coin_conservation` (`crates/ade_ledger/src/conway.rs:169`) currently early-outs —
`if body.certs.is_some() || body.withdrawals.is_some() { return Ok(()) }` (conway.rs:174) —
accepting every cert/withdrawal-bearing Conway tx **without any value check**. B3 is merge-safe
only when every such tx is either **fully accounted** (`consumed == produced` under the full
equation) or **deterministically rejected with a structured error**. A silent bounded deferral
would reproduce, in spirit, the very early-out bug B3 retires.

**Target equation (coin level, Conway UTXO rule):**

```
consumed = Σ(resolved input coins) + Σ(withdrawals) + refunded_deposits
produced = Σ(output coins) + fee + donation + new_deposits
accept ⟺ consumed == produced
```

## 1. What must always be true

- **I-1 (conservation, full form).** For every accepted Conway tx — *including* those carrying
  certificates and/or withdrawals — `consumed == produced` under the full equation. Strengthens
  **T-CONSERV-01 / CN-LEDGER-07** beyond the deposit-free spend case closed in B2-S4.
- **I-2 (closed, total per-era cert classification).** Every Conway certificate variant maps to
  exactly one deposit-effect class — `NewDeposit(coin)`, `Refund(coin)`, `Neutral`, or
  `ExplicitReject` — via a closed, era-versioned function. The deposit/refund coin comes from the
  cert itself for Conway explicit-deposit variants (reg/unreg, tags 7/8) and from the protocol
  parameter for legacy variants (tags 0/1). Cert-classification analogue of **DC-TXV-05**.
- **I-3 (deposit params are canonical input).** `key_deposit`, `pool_deposit`, `drep_deposit`,
  `gov_action_deposit` enter the equation only as canonical `ProtocolParameters` values threaded
  into the validator — never read ambiently from testkit `ConwayGovParams`, defaults, fallback
  constants, or any shell source.
- **I-4 (withdrawal sum is total and exact).** Σ(withdrawals) is the sum over a fully-decoded
  `map{reward_account → coin}`; a partially-decoded or truncated withdrawals field is never summed
  as if complete.
- **I-5 (integer-exact, deterministic).** All accumulation is `i128` (or checked `u64`),
  `BTreeMap`/`BTreeSet` iteration order, no float, no `HashMap`.

## 2. What must never be possible

- **N-1 (no false accept via deferral).** The `if body.certs.is_some() || body.withdrawals.is_some()
  { return Ok(()) }` early-out becomes impossible — a value-imbalanced cert/withdrawal-bearing tx
  accepted unchecked is a release-blocking false accept (`feedback_tx_validity_priority`).
- **N-2 (no silent skip on unknown cert).** An unrecognized cert tag, malformed cert CBOR, or
  undecodable withdrawals map must **REJECT** with a structured error — never be classified
  `Neutral` and skipped. The fail-open precedent in `rules.rs:1430-1439` ("non-fatal during replay")
  must **not** be inherited. Strengthens **DC-VAL-06**.
- **N-3 (no Shelley-decoder coverage gap).** Conway cert bytes must not be decoded with the
  6-variant Shelley decoder with unmatched Conway variants silently dropped. Either the decoder is
  Conway-complete, or any unmatched variant is an N-2 reject.
- **N-4 (no ambient deposit value).** A deposit/refund term computed from a default/testkit param
  while the canonical chain uses a different value — a determinism/replay break — is unrepresentable.
- **N-5 (no double-counted / mis-charged pool deposit).** A `PoolRegistration` updating an
  *already-registered* pool charges **no** new deposit; charging one is a false reject. State-dependent
  (see decision D1).

## 3. What must remain identical across executions

- The decoded cert classification list and decoded withdrawals map for fixed input bytes.
- The `(consumed, produced)` pair for a fixed `(tx_body, resolved UTxO, cert/pool/DRep state, protocol params)`.
- The structured error and its reason class on imbalance / unknown cert / decode failure — byte-stable per **T-DET-01**.

## 4. What must be replay-equivalent

- Replaying the Conway-576 positive corpus through the value equation yields byte-identical verdicts
  (extends the B2-S3 tx-validity replay surface, **DC-TXV-03**).
- The mandatory **adversarial negative corpus** — value-imbalanced cert/withdrawal txs, unknown-cert-tag
  txs, truncated-withdrawals txs, and state-dependent cases that hit `ExplicitReject` — replays
  byte-identically to `Invalid` / structured-reject every time (`feedback_fail_closed_validation`).

## 5. State transitions in scope

All pure; the conservation check is a read-only guard (no state mutation):

1. **Cert decode + classify** — `(era, raw_cert_bytes) → Result<Vec<ClassifiedCert{effect: NewDeposit(Coin)|Refund(Coin)|Neutral}>, CertDecodeError | UnknownCertTag>`
2. **Withdrawals decode** — `(raw_withdrawals_bytes) → Result<BTreeMap<RewardAccount, Coin>, WithdrawalDecodeError>`
3. **Deposit accounting** — `(classified_certs, pool_state, drep_state, protocol_params) → Result<(new_deposits: Coin, refunded_deposits: Coin), DepositAccountingError | UnsupportedStateDependentDepositAccounting>`
4. **Conservation guard** — `(tx_body, resolved_utxo, withdrawals_sum, new_deposits, refunded_deposits) → Result<(), ConservationError>`

Composer `validate_conway_state_backed` gains the deposit-param input (I-3) and the required
cert/pool/DRep state (D1), and replaces the deferral branch with 1→4.

## 6. TCB color hypothesis

| Component | Color | Note |
|---|---|---|
| Conway cert decoder / classifier (codec + ledger) | **BLUE** | Closed grammar; unknown tag → reject |
| Withdrawals decoder | **BLUE** | |
| Deposit-accounting + conservation guard (`conway.rs`) | **BLUE** | |
| `ProtocolParameters` deposit-param threading | **BLUE** | I-3 canonicalization |
| Promotion of `drep_deposit`/`gov_action_deposit` into canonical `ProtocolParameters` | **BLUE** | D3 — leaves testkit-only `ConwayGovParams` |
| Adversarial + positive corpus harness (`ade_testkit`) | **GREEN** | Evidence only; no authority |

## 7. Resolved scope decisions (from review)

- **D1 (Q1 — state threading).** B3 threads the required cert/pool/DRep state **now**. State-dependent
  cert cases that cannot be accounted are **not** silently deferred — they reject with a structured
  `UnsupportedStateDependentDepositAccounting` error. (Covers N-5 / pool re-registration.)
- **D2 (Q2 — withdrawal-equals-balance).** **Out of B3's conservation equation.** Conservation needs
  only Σ(withdrawals). The "a withdrawal must equal the full reward-account balance" rule needs
  reward-account state and is a **separate adjacent ledger-verdict invariant slice** — not mixed into
  B3 unless B3 already threads reward state.
- **D3 (Q3 — deposit param home).** Promote `drep_deposit` and `gov_action_deposit` into canonical
  `ProtocolParameters` (they are authoritative ledger protocol parameters). Do **not** thread the
  testkit-shaped `ConwayGovParams` object. Deposit params enter only as canonical validator inputs (I-3 / N-4).
- **D4 (Q4 — decoder completeness, mechanical proof obligation).** Enumerate Conway certificate tags
  `0..18` against the Conway CDDL / reference oracle; require each tag to be classified as
  `NewDeposit | Refund | Neutral | ExplicitReject`. Unknown or malformed tags must never become
  `Neutral`. Directly supports N-2 / N-3 and prevents Shelley-decoder coverage gaps.
- **D5 (Q5 — donation vs treasury_value).** `donation` (key 22) stays a **produced** term in the
  equation. `treasury_value` (key 21) carries **no conservation weight** — marked a
  **reference-confirmation item** before enforcement (confirm via reference differential evidence).

## 8. Tier classification

| Item | Tier | Reason |
|---|---|---|
| Conservation itself | **true** | UTxO/value conservation is already true-tier as T-CONSERV-01 / CN-LEDGER-07. |
| Conway cert classification | **derived** | Cardano/Conway-specific mapping of cert variants to deposit effects (DC-TXV-06). |
| Canonical deposit-param threading | **derived** | Cardano-specific parameter surface enforcing true determinism (DC-TXV-07). |
| Adversarial corpus | **release** | Test gate / evidence, not runtime law. |
| Testkit harness | **GREEN** | Evidence only; no authority. |

## 9. Registry actions

**New entries appended at sketch time** (status `declared`, cluster `PHASE4-B3`):

- **DC-TXV-06** — closed, total, era-versioned cert-deposit classification; unknown/malformed → deterministic reject.
- **DC-TXV-07** — canonical deposit-parameter authority; never testkit/shell/default/ambient sources. Kept
  independent of N-4 so it is separately CI-checkable.

**Planned strengthenings (deferred to `/cluster-close`, not appended at sketch time** — repo convention is
that `strengthened_in` lists clusters whose enforcement has *shipped*; appending before code/tests exist
would be a premature flip per `feedback_hard_closure_gates`):

- **T-CONSERV-01 / CN-LEDGER-07** — extend conservation to deposit/refund/withdrawal terms; flips toward
  `enforced` for the Conway tx path.
- **DC-VAL-06** — the deferral early-out is a `skip` pattern; B3 removes it.
- **DC-TXV-03** — adds the B3 adversarial negative corpus.
