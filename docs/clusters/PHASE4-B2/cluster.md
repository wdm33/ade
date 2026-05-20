# Cluster PHASE4-B2 — Transaction Validity Agreement

> **Tier**: 1 for the validity verdict (forged-spend / chain-split class; a
> false-accept is **release-blocking**); 5 for mempool eviction/ordering.
> **Status**: opening — slice work begins with B2-S1 (the Conway vkey-witness gap).
> **Bounty priority #2** — the challenger wins on "a transaction for which your
> node and the Haskell node disagree on validity."
> **Planning trio**:
> - `docs/planning/phase4-b2-invariants.md` (sketch; DC-TXV-01..05)
> - `docs/planning/phase4-b2-cluster-slice-plan.md` (ratified 5-slice plan)
> - this document — cluster spec

## Primary invariant

> `tx_validity(LedgerState, tx_cbor)` is a pure, total function whose
> Valid/Invalid verdict equals the Haskell reference on valid AND invalid
> transactions. No false accept.

Anchored by **`DC-TXV-01..05`**; strengthens **`DC-LEDGER-05`**, **`CN-LEDGER-09`**
(Conway vkey-witness binding — the gap), **`DC-MEM-*`** (Tier-1 admission),
**`DC-VAL-06`**, **`T-ENC-01`**, **`T-DET-01`**, **`DC-LEDGER-02`**.

## Normative anchors

- `docs/ade-invariant-registry.toml` — DC-TXV-01..05 (declared → enforced).
- `docs/planning/phase4-b2-invariants.md`.
- `docs/active/CE-79_gate_statement.md` (Tier 1).
- Public specs: Cardano ledger spec — Conway UTXOW (witness + required-signer
  rules), UTXO (value/fee), and the per-era required-signer sources;
  cardano-node reference behavior.

---

## Entry Conditions

- **B1 closed**: `block_validity`, `apply_block_with_verdicts` (body validation),
  preserved-bytes hashing, the fail-closed + adversarial-corpus discipline,
  `corpus/validity/conway_epoch576/` (14 real Conway blocks → txs to extract).
- `ade_crypto::verify_ed25519` + `Ed25519VerificationKey::from_bytes` /
  `Ed25519Signature::from_bytes` are fail-closed.
- `LedgerState.utxo_state` supports input resolution; the Conway tx decoder
  (`ade_codec::conway::tx`) extracts `required_signers` (k14), `withdrawals`
  (k5), certificates, `voting_procedures` (k19), `proposal_procedures` (k20),
  `collateral_inputs` (k11), inputs (k0).
- **Known gap (this cluster's #1 obligation)**: `verify_shelley_witnesses`
  validates *supplied* witnesses' signatures but does NOT check required-signer
  *coverage*; and the Conway/late-era body path does not call it at all
  (`project_conway_body_witness_gap`).

---

## Required-signer enumeration (DC-TXV-05 — resolved for cluster-doc)

`required_signers(state, tx, era)` is a **closed, era-versioned** function. For
Conway it enumerates exactly these sources of required `Hash28` key hashes:

1. **Resolved input payment credentials** — for each spent input (k0), resolve
   against `utxo_state`; if the output address's payment credential is a
   key-hash, that hash is required (script-hash credentials are phase-2 / native
   script, not vkey).
2. **Explicit `required_signers`** (k14).
3. **Withdrawal reward-account credentials** (k5) — key-hash stake credentials.
4. **Certificate key hashes** — per cert kind (stake dereg/deleg stake key,
   pool register/retire cold key, DRep register/update/unregister DRep key,
   committee hot/cold keys, etc.).
5. **Governance voting credentials** (k19) — each voter's key-hash credential.
6. **Collateral input payment credentials** (k11) — key-hash credentials, when
   the tx carries scripts.

A signer source not in this closed enumeration is **impossible to silently
omit** (the enum is exhaustive; new sources require an explicit, versioned
addition). Coverage check: every enumerated key hash must be covered by a
witness whose Ed25519 signature over the **preserved tx body hash** verifies
fail-closed; an extra irrelevant witness never substitutes for a missing one.

> Open verification at B2-S1 entry: pin the exact per-cert-kind key-hash rules
> and the voter-credential extraction against the Conway ledger spec; validate
> against the real corpus txs (they must stay Valid) + the adversarial corpus.

---

## Exit Criteria (CI-Verifiable)

| CE | Check | Closed by |
|---|---|---|
| **CE-B2-1** | `cargo test -p ade_ledger --test tx_witness_closure` PASS — `required_signers` is closed/era-versioned; Conway vkey witnesses verified fail-closed; hard gates: missing {payment-key, explicit-required-signer, withdrawal, certificate, governance-voting} witness → Invalid; wrong-size sig → Invalid; sig over wrong body → Invalid; right-key-wrong-body → Invalid; extra irrelevant witness does NOT substitute. | B2-S1 |
| **CE-B2-2** | `cargo test -p ade_ledger --test tx_validity_compose` + `--lib tx_validity` PASS — `tx_validity` composes phase-1 ∧ phase-2; closed `TxValidityVerdict`/`TxRejectClass`; total evolution (Valid → applied, Invalid → unchanged + reason); `tx_id` from preserved bytes. | B2-S2 |
| **CE-B2-3** | `cargo test -p ade_ledger --test tx_validity_positive_corpus` PASS — every real on-chain Conway tx (extracted from corpus blocks) → Valid; replay byte-identical across two runs. | B2-S3 |
| **CE-B2-4** | `cargo test -p ade_ledger --test tx_validity_adversarial_corpus` PASS — every adversarial tx → Invalid with the spec-defined fail-closed class. **No false accept.** | B2-S4 |
| **CE-B2-5** | `cargo test -p ade_ledger --test mempool_admission` PASS — admit iff `tx_validity(current_state, tx)` Valid; no false accept into the mempool; eviction/ordering provably cannot change the validity verdict. | B2-S5 |

**Oracle policy**: positive = on-chain inclusion; negative = spec-defined
invalidity (reference confirmation best-effort). Same as B1.

---

## Expected Slice Types

- **B2-S1** — BLUE phase-1 closure: `required_signers` enumeration + fail-closed
  witness coverage (the gap; shared with the B1 body path).
- **B2-S2** — BLUE composition + canonical types: `tx_validity` + verdict taxonomy.
- **B2-S3** — GREEN+RED positive corpus + replay.
- **B2-S4** — GREEN+RED adversarial corpus (no false accept).
- **B2-S5** — BLUE admission gate + GREEN policy stub (Tier 5 out of validity).

---

## TCB Color Map (FC/IS Partition)

| Module | Color | Constraint |
|---|---|---|
| `ade_ledger::tx_validity::required_signers` (NEW) | **BLUE** | Closed, era-versioned enumeration; pure over `(LedgerState, tx_body)`. |
| `ade_ledger::tx_validity::witness` (NEW) | **BLUE** | Fail-closed Ed25519 coverage check over preserved body hash. Shared by the B1 body path. |
| `ade_ledger::tx_validity::{phase1, transition, verdict}` (NEW) | **BLUE** | phase-1 ∧ phase-2 composition; closed `TxValidityVerdict`/`TxRejectClass`. |
| `ade_ledger::mempool::admit` (NEW) | **BLUE** | Tier-1 admission gate (admit iff Valid). |
| `ade_ledger::mempool` policy (NEW) | **GREEN** | Eviction/ordering — Tier 5; must not affect validity. |
| `ade_ledger::rules` (consume/extend) | **BLUE** | Conway body path calls the shared phase-1 witness closure (gap fix). |
| `ade_testkit::tx_validity` (NEW) | **GREEN** | Corpus harness + tx mutators. |
| corpus/oracle loading | **RED** | Tx extraction + verdict loading. |

Rules: no RED in BLUE; GREEN must not affect authoritative outputs.

---

## Forbidden During This Cluster

- Skipping vkey-witness verification on any era's tx path (the gap).
- A required-signer source omitted from the closed era enumeration (DC-TXV-05).
- Verifying supplied witnesses without checking required-signer **coverage**.
- Mempool eviction/ordering feeding back into the validity verdict.
- Re-encoding for the tx id/hash instead of preserved bytes.
- Fail-open length guards; `HashMap`/`HashSet`/float/wall-clock in BLUE.
- Positive-only coverage — the adversarial corpus (CE-B2-4) is mandatory.
- Softening any assertion to make a tx pass — **a false accept is release-blocking**.

---

## Slices

| ID | Name | TCB | Closes |
|---|---|---|---|
| **B2-S1** | Conway vkey-witness + required-signer closure (THE GAP) | BLUE | **CE-B2-1** |
| **B2-S2** | `tx_validity` entry + verdict taxonomy + phase-1∧phase-2 | BLUE | **CE-B2-2** |
| **B2-S3** | positive tx corpus + replay | GREEN+RED | **CE-B2-3** |
| **B2-S4** | adversarial tx corpus — no false accept | GREEN+RED | **CE-B2-4** |
| **B2-S5** | mempool admission gate (Tier-1) | BLUE+GREEN | **CE-B2-5** |

Slice docs (`B2-S1.md` … `B2-S5.md`) land here via `/slice-doc B2-SN`.

---

## Engineering Surface (Forward-Looking)

```
crates/ade_ledger/src/
  tx_validity/
    mod.rs
    required_signers.rs    # B2-S1 BLUE (closed era enumeration)
    witness.rs             # B2-S1 BLUE (fail-closed coverage; shared w/ body path)
    verdict.rs             # B2-S2 BLUE (TxValidityVerdict/TxRejectClass)
    phase1.rs              # B2-S2 BLUE
    transition.rs          # B2-S2 BLUE (tx_validity)
  mempool/
    mod.rs
    admit.rs               # B2-S5 BLUE (Tier-1 gate)
    policy.rs              # B2-S5 GREEN (Tier-5 eviction/ordering stub)
crates/ade_testkit/src/tx_validity/   # B2-S3/S4 GREEN harness + mutators
crates/ade_ledger/tests/
    tx_witness_closure.rs              # CE-B2-1
    tx_validity_compose.rs             # CE-B2-2
    tx_validity_positive_corpus.rs     # CE-B2-3
    tx_validity_adversarial_corpus.rs  # CE-B2-4
    mempool_admission.rs               # CE-B2-5
corpus/tx_validity/{positive,adversarial}/
```

---

## Replay Obligations (Cluster-Level)

- **New canonical types**: `TxValidityVerdict`, `TxRejectClass`, `TxValidityError`,
  `RequiredSigners`, per-tx verdict surface (CBOR, paralleling B1's `VerdictSurface`).
- **New authoritative transition**: `tx_validity` + the shared phase-1 witness
  closure (also reached by the B1 body path).
- **New replay corpus**: `corpus/tx_validity/{positive,adversarial}/` (positive =
  txs extracted from the real Conway corpus blocks; adversarial = deterministic
  mutators). Replay byte-identically; anchored by `T-DET-01`, `DC-LEDGER-02`.

---

## Invariants Strengthened By This Cluster

| Slice | Strengthens / introduces |
|---|---|
| B2-S1 | `DC-TXV-05` (NEW), `DC-LEDGER-05`, `CN-LEDGER-09`, `DC-VAL-06` |
| B2-S2 | `DC-TXV-01/02/04`, `T-ENC-01` |
| B2-S3 | `DC-TXV-03`, `T-DET-01` |
| B2-S4 | `DC-TXV-03`, `DC-VAL-06`, `DC-LEDGER-02` |
| B2-S5 | `DC-MEM-*` |

`DC-TXV-01..05` flip `declared → enforced` as their closing slice lands.

---

## Authority Reminder

Planning aid. Authority for invariants belongs to the registry; for mechanical
acceptance, the named tests/CI. If this doc conflicts with the registry or
normative specs, those win.
