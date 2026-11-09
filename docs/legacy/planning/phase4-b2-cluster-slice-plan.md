# Cluster/Slice Plan — Ade / PHASE4-B2 (Transaction Validity Agreement)

> **Status**: planning artifact, non-normative. Authority lives in
> `docs/ade-invariant-registry.toml` and `docs/planning/phase4-b2-invariants.md`.
> Produced via `/cluster-plan`.

## Inputs
- `docs/planning/phase4-b2-invariants.md` — B2 invariant sketch (DC-TXV-01..05).
- `docs/ade-invariant-registry.toml` — DC-TXV-01..05 (declared); DC-LEDGER-05,
  CN-LEDGER-09, DC-MEM-*, T-ENC-01, T-DET-01, DC-LEDGER-02 (to strengthen).
- B1 (closed): `block_validity`, `DC-VAL`, fail-closed + adversarial-corpus discipline.
- `~/.claude/methodology/idd.md` Part I §§1–10, Part IV.

## Cluster Index (Dependency Order)
1. **PHASE4-B2** — Transaction validity agreement — primary invariant: a tx fed
   directly is Valid iff phase-1 ∧ phase-2 accept it against the ledger state,
   and that verdict equals the Haskell reference; **no false accept**. *(detailed below)*
2. **PHASE4-B3** — Block production on preprod — *(future)* — depends on B1, B2.
3. **PHASE4-B4** — Sync from genesis/Mithril → tip — *(future)* — depends on B1 + N-D.

Mempool *policy* (eviction/ordering/propagation, N2C/N2N tx-submission plumbing)
is **Tier 5 and out of B2** beyond the Tier-1 admission gate (B2-S5); it lands in
a later mempool-plumbing cluster.

---

## Cluster PHASE4-B2 — Transaction validity agreement

**Primary invariant:**
> `tx_validity(LedgerState, tx_cbor)` is a pure, total function whose Valid/Invalid
> verdict equals the Haskell reference on valid AND invalid transactions — proven
> on a positive corpus (real on-chain txs) and a mandatory adversarial corpus. A
> false-accept is **release-blocking**.

**Anchors:** NEW `DC-TXV-01..05`; strengthens `DC-LEDGER-05`, `CN-LEDGER-09`
(Conway vkey-witness binding — the gap), `DC-MEM-*` (Tier-1 admission),
`DC-VAL-06`, `T-ENC-01`, `T-DET-01`, `DC-LEDGER-02`.

**TCB partition:**
- **BLUE:** `ade_ledger::tx_validity::{required_signers, witness, phase1, transition, verdict}`
  (phase-1 incl. UTxO-resolution-aware required-signer enumeration + fail-closed
  vkey-witness verification + phase-2 dispatch; `TxValidityVerdict`/`TxRejectClass`
  closed taxonomies); `ade_ledger::mempool::admit` (Tier-1 admission gate).
- **GREEN:** `ade_testkit::tx_validity` (positive + adversarial corpus harness +
  tx mutators); `ade_ledger::mempool` policy stub (eviction/ordering — Tier 5,
  must not affect validity).
- **RED:** tx ingest (N2C local-tx-submission / N2N tx-submission2 — later);
  oracle-verdict + tx-corpus loading.

**Entry conditions:**
- B1 closed: `apply_block_with_verdicts` (body validation), `block_validity`,
  preserved-bytes hashing, the fail-closed/adversarial discipline.
- `ade_crypto::verify_ed25519` fail-closed (via `from_bytes`).
- `LedgerState.utxo_state` available for input resolution.
- Known gap (this cluster's #1 obligation): the Conway/late-era body path does
  NOT verify vkey witnesses (`project_conway_body_witness_gap`).

**Cluster Exit Criteria (CI-verifiable):**

| CE | Check | Closed by |
|---|---|---|
| **CE-B2-1** | `cargo test -p ade_ledger --test tx_witness_closure` PASS — Conway phase-1 verifies vkey witnesses fail-closed; `required_signers` is a closed, era-versioned enumeration; each B2-S1 hard gate (missing payment-key / explicit-required-signer / withdrawal / certificate / governance-voting witness → Invalid; wrong-size sig; sig over wrong body; right-key-wrong-body; extra irrelevant witness does not substitute). | B2-S1 |
| **CE-B2-2** | `cargo test -p ade_ledger --lib tx_validity` + `--test tx_validity_compose` PASS — `tx_validity` composes phase-1 ∧ phase-2 into a closed verdict; total evolution (Valid → applied state, Invalid → unchanged + reason); tx_id from preserved bytes. | B2-S2 |
| **CE-B2-3** | `cargo test -p ade_ledger --test tx_validity_positive_corpus` PASS — every real on-chain Conway tx (extracted from the corpus blocks) → Valid; verdict stream byte-identical across two runs. | B2-S3 |
| **CE-B2-4** | `cargo test -p ade_ledger --test tx_validity_adversarial_corpus` PASS — every adversarial tx (forged/missing witness per source, wrong-size sig, sig-over-wrong-body, value imbalance) → Invalid with the spec-defined fail-closed class. **No false accept.** | B2-S4 |
| **CE-B2-5** | `cargo test -p ade_ledger --test mempool_admission` PASS — admission accepts a tx iff `tx_validity(current_state, tx)` is Valid; no false accept into the mempool; eviction/ordering policy provably does not change the validity verdict. | B2-S5 |

**Oracle policy:** positive = on-chain inclusion (a tx in a real block is valid).
Negative = spec-defined invalidity (reference-node confirmation best-effort).
Same as B1.

**Slices:**

- **B2-S1** — Conway phase-1 vkey-witness + required-signer closure (THE GAP) —
  *invariant:* a Conway tx is accepted only if `required_signers(state, tx, era)`
  (closed, era-versioned: resolved input payment creds via UTxO, explicit
  required-signers, certs, withdrawals, governance/voting, collateral) is fully
  enumerated and every required key hash is covered by a valid vkey witness over
  the preserved tx body hash — fail-closed — *addresses:* **CE-B2-1**;
  strengthens DC-LEDGER-05, CN-LEDGER-09; introduces DC-TXV-05 — *TCB:* BLUE.
  Also closes the gap on the B1 block body path (shared phase-1).
- **B2-S2** — `tx_validity` entry point + verdict taxonomy + phase-1∧phase-2
  composition — *invariant:* `tx_validity(LedgerState, tx_cbor)` is a pure, total
  composition; closed `TxValidityVerdict`/`TxRejectClass`; tx_id from preserved
  bytes — *addresses:* **CE-B2-2**; DC-TXV-01/02/04, T-ENC-01 — *TCB:* BLUE.
- **B2-S3** — Positive tx corpus + replay — *invariant:* every real on-chain
  Conway tx → Valid; replay-equivalent — *addresses:* **CE-B2-3**; DC-TXV-03,
  T-DET-01 — *TCB:* GREEN + RED.
- **B2-S4** — Adversarial tx corpus (no false accept) — *invariant:* every
  malformed/forged tx → Invalid with the correct fail-closed class —
  *addresses:* **CE-B2-4**; DC-TXV-03, DC-VAL-06, DC-LEDGER-02 — *TCB:* GREEN + RED.
- **B2-S5** — Mempool admission gate (Tier-1) — *invariant:* admit iff
  `tx_validity` Valid; no false accept; eviction/ordering (Tier 5) cannot affect
  the validity verdict — *addresses:* **CE-B2-5**; DC-MEM-* — *TCB:* BLUE
  (admission) + GREEN (policy stub).

**Forbidden during this cluster** (inherits B1 + global doctrine):
- Skipping vkey-witness verification on any era's tx path (the gap).
- A required-signer source omitted from the closed era enumeration.
- Mempool eviction/ordering feeding back into the validity verdict.
- Fail-open length guards; re-encoding for tx hash; `HashMap`/float/wall-clock in BLUE.
- Positive-only coverage — the adversarial corpus (CE-B2-4) is mandatory.
- **A false accept is release-blocking** — never soften an assertion to pass a tx.

**Replay obligations:**
- New canonical types: `TxValidityVerdict`, `TxRejectClass`, `TxValidityError`,
  `RequiredSigners`, the per-tx verdict surface (CBOR, paralleling B1's
  `VerdictSurface`).
- New authoritative transition: `tx_validity` (+ shared phase-1 used by the B1
  body path).
- New corpus: `corpus/tx_validity/{positive,adversarial}/` (positive = txs
  extracted from the real Conway corpus blocks; adversarial = deterministic
  mutators). Replay byte-identically; anchored by `T-DET-01`, `DC-LEDGER-02`.

**Open items for `/cluster-doc PHASE4-B2`:**
- Confirm the complete per-era required-signer source enumeration (the DC-TXV-05
  proof obligation) against the Conway ledger spec — the load-bearing correctness item.
- Where the shared phase-1 lives so both `tx_validity` and the B1 body path use one
  witness/required-signer implementation (no divergence).
- Phase-2/Plutus CE-88 carve-out policy for B2-S2/S3 (honest reporting, same as B1-S6).
- Mempool accumulating-state shape + intra-block tx dependency (B2-S5).

## Authority reminder
Planning aid only. Authority for rules belongs to `docs/ade-invariant-registry.toml`;
for mechanical acceptance, the named tests/CI. If this plan conflicts with the
registry or normative specs, those win.
