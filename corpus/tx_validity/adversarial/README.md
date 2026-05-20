# Adversarial transaction corpus (no false accept) — PHASE4-B2-S4 / CE-B2-4

This directory documents the adversarial transaction corpus that proves the
load-bearing bounty property: **`tx_validity` never accepts a malformed or
forged transaction as `Valid`** (no false accept; a false accept is
release-blocking).

There are **no committed CBOR fixtures here** — every adversarial transaction is
generated deterministically at test time by the GREEN harness
`ade_testkit::tx_validity::adversarial`, then driven through the BLUE
`ade_ledger::tx_validity::tx_validity`. The test
`crates/ade_ledger/tests/tx_validity_adversarial_corpus.rs` asserts that **every**
generated mutation yields `Invalid`, and that the verdict-surface stream replays
byte-identically across two runs.

## Two mutation families

### (A) Witness mutations on REAL corpus transactions (`track_utxo = false`)

Each real on-wire Conway transaction extracted from the committed Conway-576
corpus blocks that carries a **tx-derived required signer** (explicit
`required_signers` k14, withdrawal k5, certificate k4, or governance voter k19)
is corrupted in one targeted way. The mutator decodes the witness-set vkey
array and applies the mutation to **every** witness entry whose
`Blake2b-224(vkey)` covers a tx-derived required key — so the requirement is
genuinely violated even when several witnesses share a key hash.

At `track_utxo = false` the tx-derived required-signer coverage and the
supplied-witness Ed25519 verification both run unconditionally, so these are
valid no-false-accept targets. Real corpus txs with **no** tx-derived
requirement are skipped here (removing their witnesses is correctly not a
required-signer violation in partial mode); they are covered by family (B).

| # | Mutation | Spec rule violated | Expected reject class |
|---|----------|--------------------|-----------------------|
| W1 | Remove every witness covering a required signer | UTXOW required-signer coverage (`getConwayWitsVKeyNeeded`) | `MissingRequiredSigner` |
| W2 | Flip a byte in a covering witness signature | UTXOW Ed25519 verification over the preserved body hash | `WitnessInvalid` |
| W3 | Truncate a covering witness signature (drop 1 byte) | Fixed-size signature field (`Ed25519Signature::from_bytes`) | `WitnessInvalid` (wrong-size sig is a fail-closed `MalformedWitnessField`, routed to `WitnessInvalid`) |
| W4 | Replace a covering witness signature with one over a different message | UTXOW Ed25519 verification (right key, wrong body) | `WitnessInvalid` |

Coverage at HEAD: **103** real Conway txs extracted; **54** carry a tx-derived
required signer and qualify as family-A targets; **216** family-A mutation cases
run (54 × W1–W4). All 216 reject — none yields `Valid`.

### (B) Synthetic adversarial transactions (`track_utxo = true`, controlled UTxO)

Family (B) exercises the UTxO-dependent surface that `track_utxo = false`
defers. Each case is a controlled Conway transaction spending a single UTxO held
by a deterministic key, judged against a `track_utxo = true` Conway ledger at
epoch 576.

| # | Mutation | Spec rule violated | Expected reject class |
|---|----------|--------------------|-----------------------|
| S1 | Value imbalance: `sum(outputs) + fee != sum(inputs)` | UTXO preservation of value (`consumed == produced`) | `Phase1Invalid` |
| S2 | Missing input payment-key witness | UTXOW input payment-credential coverage | `MissingRequiredSigner` |
| S3 | Unresolvable / dangling input (UTxO holds a different input) | UTXO `BadInputsUTxO` (`inputs ⊆ dom utxo`) | `Phase1Invalid` |
| S4 | Forged input payment-key witness (right vkey, signature over a wrong message) | UTXOW Ed25519 verification | `WitnessInvalid` |

## Pre-release finding closed by this slice (S1)

S1 surfaced a genuine BLUE **fail-open gap**: the Conway/Babbage/Alonzo
state-backed validation path verified input presence, collateral, network, and
required signers but **never checked coin-level preservation of value**. A
transaction whose outputs + fee did not equal its inputs was accepted as `Valid`
at `track_utxo = true`.

Fixed in `ade_ledger::conway::check_conway_coin_conservation`, wired into
`validate_conway_state_backed`. The check enforces
`sum(input coins) == sum(output coins) + fee + donation` in its closed,
deposit-free form, applied only when the transaction carries neither
certificates (deposit/refund terms) nor withdrawals (a consumed term). For
deposit/withdrawal-bearing transactions the full preservation-of-value equation
is out of this slice's scope and is conservatively **deferred** — deferral can
never produce a false reject, and the positive corpus (`track_utxo = false`) is
unaffected. Closing the residual deposit/withdrawal accounting is follow-up
work; S1's pure-spend imbalance is now rejected fail-closed.

## Determinism

The harness uses `BTreeMap`/`BTreeSet`/`Vec` only — no `HashMap`, no float, no
wall-clock, no rand. Synthetic key material is seeded deterministically. The
adversarial verdict-surface vector is byte-identical across runs
(`adversarial_replays_identically`), anchored by `T-DET-01` / `DC-LEDGER-02`.
