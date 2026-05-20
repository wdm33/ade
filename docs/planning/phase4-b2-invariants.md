# PHASE4-B2 — Invariant Sketch (Transaction Validity Agreement / Mempool)

> **Status**: planning artifact, non-normative. Produced via `/invariants` (IDD Part I); approved with tightenings 2026-05-20.
> **Tier**: 1 for the accept/reject *validity* verdict (chain-split / forged-spend class); 5 for eviction/ordering policy.
> **Bounty priority #2** — the challenger wins if they find "a transaction for which your node and the Haskell node disagree on validity." B2 targets exactly that surface. **A false-accept is release-blocking and constitution-breaking** (false reject is bad; false accept is catastrophic).
> **B2 leads with the Conway vkey-witness gap (B2-S1).**
> **Public grounding only**: Cardano ledger spec (Shelley→Conway tx + witness/required-signer rules), cardano-node reference behavior, IDD doctrine. Builds on B1 (`block_validity`, `DC-VAL`, fail-closed + adversarial-corpus discipline).

## Can it be expressed as `canonical input → canonical output`? **Yes.**

```
tx_validity(LedgerState, tx_cbor) -> TxValidityVerdict        // Valid | Invalid{class,error}
```
Canonical input: `LedgerState` (UTxO for input resolution + pparams + cert/pool/gov state) + raw tx bytes. Canonical output: a verdict required to equal the reference's. Mempool admission is a thin Tier-1 gate over this verdict plus accumulating state.

## 1. What must always be true

| # | Invariant | Anchor |
|---|---|---|
| 1.1 | `tx_validity` is a pure function of `(LedgerState, tx_cbor)` — no wall-clock, arrival order, `HashMap` iteration, float | NEW `DC-TXV-01` |
| 1.2 | A tx is Valid iff **phase-1** (structural + UTxO rules + witnesses) **and phase-2** (Plutus, if scripts present) both accept | NEW `DC-TXV-02` |
| 1.3 | **(THE GAP)** Phase-1 verifies Ed25519 vkey witnesses **fail-closed for every era incl. Conway**: every required key hash must be covered by a valid vkey witness over the preserved tx body hash; a wrong-size sig, a signature over the wrong body, or a witness for the right key but wrong body is Invalid | **DC-LEDGER-05**, **CN-LEDGER-09** strengthened (Conway path) |
| 1.4 | Ade's Valid/Invalid verdict for a tx equals the reference verdict (positive corpus of real on-chain txs + mandatory adversarial corpus) | NEW `DC-TXV-03` (agreement) |
| 1.5 | Mempool **admission** accepts a tx iff `tx_validity(current_state, tx)` is Valid; **no false accept of an invalid tx** | **DC-MEM-*** strengthened (Tier 1) |
| 1.6 | Every crypto-input/field-size check on the tx path is fail-closed (no silent skip) | **DC-VAL-06** lineage |
| 1.7 | Total: a Valid tx yields an applied `LedgerState'` (mempool's accumulating view); an Invalid tx leaves state unchanged + a structured reason | NEW `DC-TXV-04` |
| 1.8 | The tx id/hash is computed from preserved wire bytes, never re-encoded | **T-ENC-01** |
| 1.9 | **Required-signer enumeration is closed and era-versioned**: `required_signers(state, tx, era)` is an explicit, closed function over every era's signer sources; a signer source not represented in the era's closed enumeration is impossible to silently omit | NEW `DC-TXV-05` |

> 1.9 is distinct from 1.3 by design. 1.3 says enumerated witnesses are *verified*; the more dangerous bug is *incomplete enumeration* — if `required_signers` misses one source (e.g. a Conway governance/voting witness), every enumerated signer is covered and the tx is **falsely accepted**. Enumeration completeness gets its own named, tested invariant.

## 2. What must never be possible

| # | Forbidden | Anchor |
|---|---|---|
| 2.1 | Accepting an invalid tx (forged/missing witness, value imbalance, etc.) as Valid — **release-blocking** | DC-TXV-03, 1.5 |
| 2.2 | Skipping vkey-witness verification on **any** era's tx path (the Conway gap) | 1.3 |
| 2.3 | A required-signer source omitted from the era's closed enumeration (incomplete-enumeration false accept) | 1.9 |
| 2.4 | Mempool eviction/ordering/prioritisation (Tier 5) influencing the accept/reject **validity** verdict | DC-MEM, 1.5 |
| 2.5 | A verdict depending on wall-clock, arrival order, `HashMap`, or float | DC-TXV-01 |
| 2.6 | Re-encoding for the tx id/hash instead of preserved bytes | T-ENC-01 |
| 2.7 | Partial ledger mutation on an invalid tx | DC-TXV-04 |
| 2.8 | An "unknown Conway witness source ignored" / fallback witness interpretation | 1.9, 2.2 |
| 2.9 | An extra irrelevant witness substituting for a missing required witness | 1.3 |

## 3. What must remain identical across executions
- `tx_validity(state, tx)` → same verdict + reason class.
- Mempool admission over the same ordered tx stream + same starting state → same accept/reject sequence and same resulting mempool state.

## 4. What must be replay-equivalent
Two corpora: **positive** (real on-chain Conway txs — oracle = on-chain inclusion → Valid) and **mandatory adversarial** (forged vkey witness, missing required signer per source, wrong-size sig, signature over wrong body, value imbalance → Invalid with the spec-defined class). Ordered tx-stream replay → identical verdict sequence + mempool state. Anchored by `T-DET-01`, `DC-LEDGER-02`.

## 5. State transitions in scope
```rust
TxValidity   (LedgerState, tx_cbor) -> Result<TxValidityVerdict, TxValidityError>
MempoolAdmit (MempoolState, LedgerState, tx_cbor)
             -> Result<(MempoolState, AdmitOutcome), TxValidityError>   // Tier-1 gate only

// closed taxonomies, paralleling B1's DC-VAL surface:
TxValidityVerdict = Valid { tx_id, applied: LedgerState } | Invalid { class: TxRejectClass, error: TxValidityError }
TxRejectClass = Phase1Invalid | WitnessInvalid | MissingRequiredSigner | Phase2Invalid | MalformedField | ...

// B2-S1 BLUE surface:
required_signers(state: &LedgerState, tx_body, era) -> RequiredSigners   // closed, era-versioned (DC-TXV-05)
verify_vkey_witnesses(tx_body_hash, &RequiredSigners, witnesses) -> Result<(), TxValidityError>  // fail-closed (1.3)
tx_validity_phase1(state, tx_cbor) -> Result<Phase1Ok, TxValidityError>
```
Eviction/ordering/prioritisation is a **separate Tier-5 transition** that must not feed back into validity — documented, but **out of scope for B2-S1**.

## 6. TCB color hypothesis
- **BLUE**: `tx_validity` phase-1 (incl. `required_signers` enumeration + `verify_vkey_witnesses` + UTxO input payment-credential resolution) + phase-2 dispatch; `TxValidityVerdict`/`TxRejectClass` closed taxonomies; mempool **admission** authority (the Valid/Invalid gate + accumulating-state application).
- **GREEN**: mempool **policy** (eviction order, prioritisation, queue position) — deterministic, non-authoritative, must not affect validity; corpus harness.
- **RED**: tx ingest from N2N tx-submission2 / N2C local-tx-submission; oracle-verdict loading.
- **Unresolved**: whether the mempool's accumulating `LedgerState` view is BLUE state or a GREEN cache over BLUE — open question (does not block B2-S1, which is pure `tx_validity` phase-1).

## 7. Open questions (resolve before `/cluster-plan`)
1. **Required-signer enumeration (proof obligation — the gap's core, B2-S1)**: enumerate *every* required-signer source per era — resolved input payment-key-hashes (UTxO lookup), explicit `required_signers`, certificate key hashes, withdrawal key hashes, Conway governance/voting witnesses, collateral-input signer implications. Missing one = a fail-open hole. Must be a **closed enum** (DC-TXV-05).
2. **Tx-validity oracle**: positive = on-chain inclusion (a tx in a real block is valid). Negative = spec-defined invalidity (reference-node confirmation best-effort; bundled node can't run Conway). Same policy as B1.
3. **Phase-2 / Plutus**: the CE-88/aiken limitation may block full eval of some script txs — same honest carve-out policy as B1-S6 (report, don't fake). **Out of B2-S1** (phase-1 only).
4. **Family**: new **`DC-TXV`** confirmed (parallels `DC-VAL`; tx validity has its own bounty-observable contract — not buried under generic `DC-VAL`). `DC-MEM` already exists for the admission gate.
5. **Mempool state shape** + intra-block dependency (a tx spending another mempool tx's output) — deferred past B2-S1.
6. **Convergence**: B2's per-tx phase-1 and B1's body-path per-tx validation must share one phase-1 (the body path currently skips Conway vkey witnesses — B2-S1 closes it for both call sites).

## B2-S1 (lead slice) — Conway phase-1 vkey-witness required-signer closure
**Intent**: a Conway tx is accepted only if every required-signer source is fully enumerated (closed, era-versioned) and covered by a valid vkey witness over the preserved tx body hash.
**In scope**: preserved tx body hash; Conway tx body decode; `required_signers` (input payment creds via UTxO resolution, explicit required-signers, withdrawals, certificates, governance/voting, collateral implications); `verify_vkey_witnesses`; structured Invalid verdicts.
**Out**: eviction/ordering, mempool prioritisation, Plutus phase-2, block production, broad N2C/N2N tx-submission behavior, performance tuning.
**Hard gates (acceptance)**: valid Conway on-chain corpus stays Valid; each of {missing payment-key witness, missing explicit required signer, missing withdrawal witness, missing certificate witness, missing governance/voting witness, wrong-size signature, signature over wrong body hash, witness for correct key but wrong body, extra irrelevant witness not substituting} → Invalid; `tx_id` from preserved bytes; invalid tx leaves ledger/mempool accumulating state unchanged.

## Proposed registry entries (DC-TXV family — NEW)
`DC-TXV-01..05` (below); **strengthen** `DC-LEDGER-05`, `CN-LEDGER-09` (Conway vkey-witness), `DC-MEM-*` (Tier-1 admission), `DC-VAL-06`, `T-ENC-01`, `T-DET-01`, `DC-LEDGER-02`.

## Authority reminder
Authority for invariants belongs to `docs/ade-invariant-registry.toml`. If this sketch conflicts with the registry or the normative specs, those win.
