# Slice MEM-OPT-UTXO-DISK S1 — owned-`utxo_lookup` BLUE interface change (CE-UD-1)

> **Status:** Scoped — **BLUE interface-semantics slice (HIGH-RISK, PROOF-HEAVY).** Change the authoritative `utxo_lookup` to return an **owned `Option<TxOut>`** behind a `UtxoStore` seam, so S2 can later swap in an on-disk backend. The BTreeMap stays the ONLY backend; NO redb, NO on-disk state. Proven to NOT alter any verdict, fingerprint, or failure shape. **Interface-prep — NOT a memory victory (no `DC-MEM-07` flip).**
> **Cluster:** MEM-OPT-UTXO-DISK (primary invariant `DC-MEM-05` + `DC-MEM-07`)
> **Cluster doc:** `docs/clusters/MEM-OPT-UTXO-DISK/cluster.md` · **Prior:** S0 (verdict `bootstrap_transient_but_admission_live_working_set` — the admission footprint is live ⇒ on-disk backend is the lever)

## 2. Slice Header

### Cluster Exit Criteria Addressed
- [ ] **CE-UD-1** (S1 INTERFACE): `utxo_lookup` returns owned `Option<TxOut>`; the BTreeMap is the ONLY backend (no redb); ALL ledger-validity tests green; the replay corpus byte-identical; the UTxO fingerprint identical before/after for the same state; structured errors unchanged; no new clone-heavy path in block admission; registry marks this interface-prep (**no `DC-MEM-07` flip**).

### Intent
S0 proved the active-admission footprint is a **live working set** ⇒ the structural lever is the on-disk UTxO backend (S2). But the current authoritative lookup `utxo_lookup(&UTxOState, &TxIn) -> Option<&TxOut>` (`ade_ledger/src/utxo.rs:116`) returns a **borrow into the BTreeMap** — an on-disk backend cannot hand out a borrow into redb without leaking storage lifetimes everywhere or inventing guard semantics that complicate validation. S1 changes the lookup to return an **owned `Option<TxOut>`** — the cleaner authority boundary — and **proves the change is authoritative-output-invariant** (verdicts, fingerprints, failure shapes unchanged). The BLUE interface change is ISOLATED from the storage swap (S2) so it can be replay-proved on its own.

## 3. The Change (precise)
- **`crates/ade_ledger/src/utxo.rs`:** `utxo_lookup` signature `-> Option<&TxOut>` → `-> Option<TxOut>` (clone the resolved output). Introduce a minimal **`UtxoStore`** trait (the seam for S2) with `get(&self, &TxIn) -> Option<TxOut>`; `UTxOState` (BTreeMap) is the SOLE impl.
- **Input-resolution call sites** (consume the owned `TxOut`; the per-lookup `&`→owned is a single-`TxOut` clone, NEVER a map clone): `rules.rs`, `shelley.rs:resolve_shelley_inputs` (162), `mary.rs:resolve_mary_inputs` (57/125), `alonzo.rs`/`babbage.rs`/`conway.rs` tx validators, `phase.rs:apply_phase_2_failure` (160), `tx_validity/phase1.rs` (`.utxos.get()` point lookups).
- **Unchanged in S1:** `utxo_insert`/`utxo_delete` (the pure clone-per-mutation model — S2's overlay replaces it), `fingerprint_utxo` (`fingerprint.rs:130`, iterates the BTreeMap — untouched), `encode_utxo_state`/the snapshot encoder, the WAL, and every `TxOut`/`TxIn` type.

## 4. Tier Classification (per the slice mandate)
- **true:** the change MUST be authoritative-output-invariant — same verdicts, same fingerprints (`fingerprint_utxo` untouched ⇒ same UTxO state → same fingerprint), same failure shapes. Proven by the replay corpus + the validity + adversarial corpora running byte-identical across the change.
- **derived (`DC-MEM-05` precondition):** the owned-lookup interface is the PRECONDITION for a swappable backend (S2). S1 adds no backend; it makes the interface backend-agnostic.
- **operational:** none. **NO `OP-MEM-02` movement, NO `DC-MEM-07` flip** — S1 is interface-prep, not a memory win (the BTreeMap is still fully in memory).
- **release:** the existing validity + replay CI stays green; a CI gate asserts no borrow-returning authoritative UTxO lookup remains (the interface is owned).

## 5. Scope
- **`ade_ledger/src/utxo.rs` (BLUE):** the `utxo_lookup` signature change + the `UtxoStore` seam (trait + the BTreeMap impl). `utxo_insert`/`utxo_delete`/`len`/`is_empty` keep their signatures + pure semantics.
- **The validity rules (BLUE):** every input-resolution site updated to the owned lookup (§3). Mechanical; no logic change — the resolved `TxOut` is used by value instead of by reference.
- **Proof corpora (existing, reused):** `crates/ade_ledger` validity tests (shelley/mary/alonzo/babbage/conway); `crates/ade_node/tests/admission_adversarial_corpus.rs`; `crates/ade_runtime/tests/wal_replay_from_anchor.rs` (DC-WAL-03) + `crates/ade_node/tests/admission_replay_equivalence.rs`.
- **New gate** `ci/ci_check_utxo_lookup_owned.sh`: asserts the authoritative lookup returns owned (`Option<TxOut>`, not `Option<&TxOut>`) on the BLUE path; vacuous-safe + `--self-test`.
- **Out of scope:** ANY redb / on-disk state / second backend (S2); the overlay/cache/changelog (S2); the incremental fingerprint (S2 entry gate, OQ-UD-3); any memory reduction.

## 6. Execution Boundary
- **BLUE:** `ade_ledger` `utxo_lookup` + the `UtxoStore` seam + the input-resolution sites in the validity rules. **This IS a BLUE interface-semantics change** — hence high-risk + proof-heavy.
- **GREEN:** none new.
- **RED:** none.

## 7. Invariants Preserved (the proof obligations — HARD, not assumed)
1. **Verdict invariance:** every ledger-validity test + the adversarial corpus produces identical `BlockVerdict` / `AdmissionExitCode` across the change.
2. **Fingerprint invariance:** `fingerprint_utxo` is untouched; the same UTxO state yields the identical fingerprint. A test asserts the UTxO/ledger fingerprint is unchanged for a fixed corpus before vs after.
3. **Replay-equivalence (`DC-WAL-03`):** `replay_from_anchor` two-run byte-identical + `admission_replay_equivalence` green; the tail fingerprint unchanged.
4. **Failure-shape invariance:** the structured validity error enums are unchanged — same variant + fields for every negative/adversarial case.
5. **No clone regression:** the owned lookup clones exactly ONE `TxOut` per resolved input (necessary + bounded); a check confirms NO full-map clone is introduced in the admission/validation hot path.

## 8. Invariants Introduced
- **`DC-MEM-09` (NEW — `declared`, interface-prep):** the authoritative UTxO lookup interface returns OWNED values (`Option<TxOut>`), never a borrow into storage; this is the precondition for the swappable backend (`DC-MEM-05`) and MUST NOT alter any verdict, fingerprint, or failure shape. **Explicitly NOT a `DC-MEM-07` flip** — the BTreeMap is still fully in memory; this is interface-prep, not a bounded-storage memory win. (Tests: the §7 invariance corpora; CI: `ci_check_utxo_lookup_owned.sh`.)

## 9. Design Summary
- A minimal **`UtxoStore`** trait: `fn get(&self, tx_in: &TxIn) -> Option<TxOut>` (owned). `UTxOState` implements it by `self.utxos.get(tx_in).cloned()`. The trait is the SEAM S2's redb backend implements; in S1 the BTreeMap is the only impl and `LedgerState` holds it directly (no dynamic dispatch needed yet).
- `utxo_lookup(state, tx_in)` becomes `state.get(tx_in)` (owned). Call sites that did `if let Some(out) = utxo_lookup(&state, &tx_in)` now bind an owned `out` — a single `TxOut` clone, identical value.
- **Why owned is verdict-neutral:** validation reads the resolved `TxOut` BY VALUE (address, coin, value, datum) — never by identity/address-of. A clone yields the identical value, so every downstream check (balance, witnesses, script context, fingerprint) sees identical inputs. The clone is the only change; the *values* are invariant.
- **Proof strategy:** run the full validity + adversarial + replay corpora; they are byte/verdict-identical iff the change is value-equivalent. Add a focused before/after fingerprint-identity test over a fixed UTxO corpus.

## 10. Mechanical Acceptance Criteria
- [ ] `utxo_lookup` returns owned `Option<TxOut>` (the `UtxoStore::get` seam; BTreeMap-only impl).
- [ ] The BTreeMap backend remains the ONLY backend (no redb, no on-disk state, no second impl).
- [ ] ALL ledger-validity tests green (shelley/mary/alonzo/babbage/conway + the adversarial corpus) — verdict invariance.
- [ ] The replay corpus byte-identical (`wal_replay_from_anchor` two-run + `admission_replay_equivalence`) — `DC-WAL-03`.
- [ ] The UTxO fingerprint identical before/after for the same state (focused identity test).
- [ ] Structured errors unchanged (failure-shape invariance over the negative corpus).
- [ ] No extra clone-heavy path in block admission (single-`TxOut` clone per lookup; no full-map clone).
- [ ] Registry: `DC-MEM-09` added as **interface-prep** — **NO `DC-MEM-07` flip**, NO `OP-MEM-02` movement.
- [ ] `ci/ci_check_utxo_lookup_owned.sh` green (+ `--self-test`); `cargo test` green.

## 11. Hard Prohibitions
- **No redb / no on-disk state / no second backend** — S1 is the interface change ONLY; the BTreeMap stays.
- **No behavioral change** — the owned lookup MUST NOT alter any verdict, fingerprint, or failure shape (proven, not assumed).
- **No `DC-MEM-07` flip / no `OP-MEM-02` movement** — S1 is not a memory win; the registry must not claim one.
- **No full-map clone introduced** in the hot path (the owned lookup is a single-`TxOut` clone).
- **No change to `utxo_insert`/`utxo_delete` semantics, `fingerprint_utxo`, the snapshot encoder, or the WAL** — those are S2's concern or untouched.

## 12. Explicit Non-Goals
- Not the redb anchor / bounded overlay / read cache (S2).
- Not the incremental fingerprint (S2 entry gate — OQ-UD-3 below).
- Not a memory reduction (interface-prep only).
- Not the compact `TxOut` (MEM-OPT-COMPACT, `DC-MEM-08`).

## 13. Entry Proof Obligations (carried to S2 — GATES)
- **OQ-UD-3 (per-block `post_fp` — S2 ENTRY GATE, RESOLVED-AT-ENTRY = YES):** block admission computes `post_fp = fingerprint(&next_ledger).combined` PER BLOCK (`crates/ade_node/src/admission/runner.rs:437`), and `fingerprint(state)` runs `fingerprint_utxo` (`crates/ade_ledger/src/fingerprint.rs:90,130`) which **iterates the FULL UTxO map**. So **post_fp is computed by full-UTxO iteration per block.** An on-disk backend (S2) ALONE would turn that into a per-block disk scan — catastrophic. **S2 CANNOT START until an incremental-fingerprint plan exists** (compute `post_fp` from the per-block UTxO delta, not a full scan). S1 records this; it does NOT solve it.
- **S2 key-order guardrail (carried):** S2's "redb key = canonical CBOR `[hash, index]` ⇒ redb order == `TxIn` order" must be PROVEN by a test-vector gate, not assumed. The current `Ord for TxIn` is lexicographic `(tx_hash, index)`; CBOR array encoding must preserve that for the full `u16` index domain. If any integer-width/array-prefix encoding perturbs ordering, use an explicit fixed-width key **`32-byte txid ++ BE-u32 index`** (safer for storage order + fingerprint iteration).

## 14. Completion Checklist
- [ ] `utxo_lookup` → owned + the `UtxoStore` seam (BTreeMap-only).
- [ ] All input-resolution call sites updated (single-`TxOut` clone, no logic change).
- [ ] The 5 §7 proof obligations green (verdict / fingerprint / replay / failure-shape / no-clone).
- [ ] `DC-MEM-09` registered (interface-prep; **no `DC-MEM-07` flip**); `ci_check_utxo_lookup_owned.sh` + `--self-test`.
- [ ] OQ-UD-3 + the key-order guardrail recorded as the S2 entry gates.
