# Cluster PHASE4-B1 — Full Block Validity Agreement

> **Tier**: 1 semantic. The block-validity verdict is observable — disagreeing
> with the reference node on whether a block is valid forks the network.
> Bounty priority #1; dependency root for PHASE4-B2/B3.
> **Status**: opening — slice work begins with B1-S1.
> **Planning trio**:
> - `docs/planning/phase4-b1-invariants.md` (invariant sketch; §8 investigation closed)
> - `docs/planning/phase4-b1-cluster-slice-plan.md` (ratified 7-slice plan)
> - this document — cluster spec

## Primary invariant

> `block_validity(LedgerState, PraosChainDepState, EraSchedule, &LedgerView,
> block_cbor)` is a pure, total function whose verdict equals the reference
> cardano-node verdict on valid **and** invalid blocks.

Anchored by **`DC-VAL-01..06`** (`docs/ade-invariant-registry.toml`), which
strengthen **`DC-LEDGER-02`** (no-false-accept / CE-88 lineage),
**`CN-EPOCH-01`**, **`DC-CONSENSUS-02`**, **`CN-CONS-04`**, **`DC-CRYPTO-01`**,
**`T-ENC-01`**, **`T-DET-01`**.

## Normative anchors

- `docs/ade-invariant-registry.toml` — `DC-VAL-01..06` (declared → enforced by
  this cluster); strengthened: `DC-LEDGER-02`, `CN-EPOCH-01`, `DC-CONSENSUS-02`,
  `CN-CONS-04`, `DC-CRYPTO-01`, `T-ENC-01`, `T-DET-01`.
- `docs/planning/phase4-b1-invariants.md` — sketch + §8 investigation results
  (consensus-input provenance; fail-closed audit).
- `docs/active/CE-79_gate_statement.md` — Tier 1 (must conform).
- Public specs (sketch "Public grounding"): Cardano ledger spec (Shelley→Conway),
  Ouroboros Praos spec, cardano-node reference behavior.

---

## Entry Conditions

- **PHASE4-N-B closed**: `validate_and_apply_header`, `PraosChainDepState`,
  `LedgerView` trait, `EraSchedule`, VRF/leader/nonce/op-cert kernel exist.
- **Phases 1–3 closed**: `ade_ledger::rules::apply_block_with_verdicts` does
  full body validation. Confirmed (sketch §8): Ed25519 witness verification and
  Byron bootstrap verification are fail-closed (`shelley.rs:204-221`,
  `byron.rs:231-238` via `from_bytes` constructors); body/tx hashing uses
  preserved wire bytes (`rules.rs:415`).
- **Consensus-input provenance resolved**: eta0 extracted by a proven tail-scan
  of the snapshot `state` CBOR — eta0(576)=`d4d5f9dc…`, eta0(577)=`19674ecf…`
  captured; ledger-state dumps carry pool VRF keys + set/mark/go stake; asc in
  protocol-params extract; `blocks_epoch577` available.
- **Dependency direction verified acyclic**: `ade_core` depends only on
  `ade_types`/`ade_crypto`/`minicbor`; `ade_ledger` does not yet depend on
  `ade_core` — so `ade_ledger → ade_core` (for `block_validity` + the
  `LedgerView` impl) is safe.
- **KES available**: `ade_crypto::kes::verify_kes` + `verify_opcert`
  (`OperationalCertData`) exist for B1-S5.

---

## Exit Criteria (CI-Verifiable)

| CE | Check | Closed by |
|---|---|---|
| **CE-B1-1** | `cargo test -p ade_ledger --lib consensus_view` + `cargo test -p ade_ledger --test ledger_view_corpus` PASS — production `LedgerView` returns correct `(σ, total_active_stake, vrf_key, asc)` for Conway 577 from a loaded `LedgerState`; no `HashMap`, no rederivation. | B1-S2 |
| **CE-B1-2** | `cargo test -p ade_ledger --lib block_validity` PASS — composition is `header ∧ body`, header-before-body fail-fast ordering, body-hash binding wired (proven by `body_hash_mismatch_rejected`, a negative test). | B1-S4 |
| **CE-B1-3** | `cargo test -p ade_ledger --test block_validity_positive_corpus` PASS — every block in `corpus/validity/conway_epoch577/` → Valid (oracle = on-chain inclusion); verdict stream byte-identical across two runs. | B1-S6 |
| **CE-B1-4** | `cargo test -p ade_ledger --test block_validity_adversarial_corpus` PASS — every block in `corpus/validity/adversarial/` → Invalid with the spec-defined fail-closed reason class (bad witness/key/sig size, fabricated witness, bignum overflow, altered body-vs-header-hash, future slot, era-mismatched VRF, bad KES sig, missing required signer). | B1-S5 + B1-S7 |
| **CE-B1-5** | `cargo test -p ade_ledger --test block_validity_positive_corpus -- evolution` PASS — Valid → evolved `(LedgerState', PraosChainDepState')`; Invalid → unchanged states + reason; asserted by `invalid_block_leaves_state_unchanged`. | B1-S4 + B1-S6 |

**Oracle policy**: positive = on-chain inclusion (no node query); negative =
spec-defined invalidity (reference-node confirmation best-effort — bundled
cardano-node 8.12.2 cannot run Conway).

---

## Expected Slice Types

- **B1-S1** — RED extractor + GREEN corpus: snapshot `state` CBOR nonce
  tail-scan, un-skip pool VRF keys in the loader, assemble Conway-577 corpus.
- **B1-S2** — BLUE projection: `LedgerView` impl over `LedgerState`.
- **B1-S3** — BLUE canonical-type slice: verdict/error closed taxonomies + CBOR.
- **B1-S4** — BLUE state-transition slice: `block_validity` composition.
- **B1-S5** — BLUE header-completeness: KES verify + era-correct VRF domain +
  fail-closed field checks (`expect_size`).
- **B1-S6** — GREEN+RED replay slice: positive agreement corpus + replay.
- **B1-S7** — GREEN+RED adversarial slice: negative agreement corpus + mutators.

---

## TCB Color Map (FC/IS Partition)

| Module | Color | Constraint |
|---|---|---|
| `ade_ledger::consensus_view` (NEW) | **BLUE** | Pure/total projection of `LedgerState` → `LedgerView`. `BTreeMap` only; no I/O, no rederivation. |
| `ade_ledger::block_validity` (NEW) | **BLUE** | Composition transition + `BlockValidityVerdict`/`BlockValidityError`/`FieldError` closed taxonomies + canonical CBOR. Depends on `ade_core` (acyclic). |
| `ade_core::consensus::header_validate` (extend) | **BLUE** | KES sig verification + era-correct VRF domain wired into header validity. |
| `ade_crypto::kes` (consume) | **BLUE** | `verify_kes`/`verify_opcert` — already BLUE. |
| `ade_ledger::rules` (consume) | **BLUE** | `apply_block_with_verdicts` — already BLUE; B1-S5 adds an `expect_size` helper if any fail-open found. |
| `ade_testkit::validity` (NEW) | **GREEN** | Positive + adversarial corpus harness + block mutators. Non-authoritative. |
| `ade_testkit::harness::snapshot_loader` (extend) | **RED** | Un-skip pool VRF field; load ledger-state dumps. |
| `ade_ledger::consensus_input_extract` (NEW) or testkit tool | **RED** | Snapshot `state` CBOR nonce tail-scan; S3/disk loading. |

Rules: no RED behavior in BLUE; GREEN must not affect authoritative outputs.

---

## Forbidden During This Cluster

- Fail-open length/size guards (`if X.len()==K {check} else {skip}`) on any
  authority path — `DC-VAL-06`.
- Defined-but-unwired checks; tautological/no-op guards.
- Re-encoding for body/tx hashing instead of preserved wire bytes — `T-ENC-01`.
- `LedgerView` rederiving a stake snapshot; mark/go instead of set —
  `DC-CONSENSUS-02`, `CN-EPOCH-01`.
- A "trust the body / skip header" path in the authoritative verdict (the
  follow-bridge's RED peer-trusted shortcut must not leak here) — `DC-VAL-02`.
- `HashMap`/`HashSet`, float, wall-clock in BLUE.
- Positive-only test coverage — the adversarial corpus (CE-B1-4) is mandatory.
- TODO/placeholder reason variants — every reject is a structured
  `BlockValidityError`.

---

## Slices

| ID | Name | TCB | Closes |
|---|---|---|---|
| **B1-S1** | Consensus-input extractor + Conway-577 corpus | RED + GREEN | CE-B1-1 inputs, CE-B1-3 corpus |
| **B1-S2** | Production `LedgerView` projection | BLUE | **CE-B1-1** |
| **B1-S3** | `BlockValidity` verdict/error taxonomies + encoding | BLUE | substrate |
| **B1-S4** | `block_validity` composition transition | BLUE | **CE-B1-2**, CE-B1-5 |
| **B1-S5** | Header-validity completeness (KES + VRF domain + fail-closed) | BLUE | CE-B1-4 (header half) |
| **B1-S6** | Positive agreement corpus + replay | GREEN + RED | **CE-B1-3**, CE-B1-5 |
| **B1-S7** | Adversarial/negative agreement corpus | GREEN + RED | **CE-B1-4** |

Slice docs (`B1-S1.md` … `B1-S7.md`) land here via `/slice-doc B1-SN`.

---

## Engineering Surface (Forward-Looking)

```
crates/ade_ledger/
  Cargo.toml                       # + ade_core dep (acyclic)
  src/
    consensus_view.rs              # B1-S2 BLUE LedgerView impl
    block_validity.rs              # B1-S3/S4 BLUE verdict + composition
    consensus_input_extract.rs     # B1-S1 RED nonce tail-scan (or testkit)
crates/ade_core/src/consensus/
    header_validate.rs             # B1-S5 KES + VRF-domain extension
crates/ade_testkit/src/
    validity/                      # B1-S6/S7 GREEN harness + mutators
    harness/snapshot_loader.rs     # B1-S1 RED: un-skip VRF field
crates/ade_ledger/tests/
    ledger_view_corpus.rs          # CE-B1-1
    block_validity_positive_corpus.rs    # CE-B1-3, CE-B1-5
    block_validity_adversarial_corpus.rs # CE-B1-4
corpus/validity/
    conway_epoch577/               # positive
    adversarial/                   # negative
ci/
    ci_check_no_fail_open_in_validation.sh   # DC-VAL-06 grep gate
```

---

## Replay Obligations (Cluster-Level)

- **New canonical types**: `BlockValidityVerdict`, `BlockValidityError`,
  `FieldError`, production `LedgerView` impl, B1 corpus schema. Round-trip-tested
  in B1-S3.
- **New authoritative transition**: `block_validity` (composes existing
  authorities; no new ambient state).
- **New replay corpus**: `corpus/validity/conway_epoch577/` (positive) and
  `corpus/validity/adversarial/` (negative). Both replay byte-identically;
  anchored by `T-DET-01`, `DC-LEDGER-02`.

---

## Invariants Strengthened By This Cluster

| Slice | Strengthens / introduces |
|---|---|
| B1-S1 | `DC-VAL-01` |
| B1-S2 | `DC-CONSENSUS-02`, `CN-EPOCH-01`, `DC-VAL-01` |
| B1-S3 | `DC-VAL-02/04/05/06` (closed taxonomies) |
| B1-S4 | `DC-VAL-02`, `DC-VAL-03`, `DC-VAL-05`, `CN-CONS-04` |
| B1-S5 | `DC-VAL-06`, `DC-CRYPTO-01`, `T-ENC-01` |
| B1-S6 | `DC-VAL-04`, `T-DET-01`, `DC-LEDGER-02` |
| B1-S7 | `DC-VAL-04`, `DC-VAL-06`, `DC-LEDGER-02` (no-false-accept) |

`DC-VAL-01..06` flip `declared → enforced` as their closing slice lands;
`DC-VAL-04` (agreement) closes at B1-S7.

---

## Authority Reminder

Planning aid. Authority for invariants belongs to
`docs/ade-invariant-registry.toml`; for mechanical acceptance, the named
tests/CI. If this doc conflicts with the registry or normative specs, those win.
