# Slice MEM-OPT-UTXO-DISK S2b step 2 — pre-resolve dependency enumeration

> **Status:** **DONE — built + proven as INFRASTRUCTURE FOR B (`LIVE-LEDGER-APPLY` / `track_utxo=true`).** **Enumerate → prove → wire.** The pre-resolve path (extract EVERY TxIn a tx's validation can read — inputs ∪ collateral ∪ reference — resolve them from the anchor into an in-memory `WorkingSet`, validate over that view only; `32e0da41`/`96118302`, gated by `ci_check_utxo_pre_resolve.sh`) is complete. It is the **B (`track_utxo=true`) live-validation path** — the current live admission runs `track_utxo=false`, so it does NOT use this path; the BA-08 owned-RSS win came from PATH A (see `S2b-2cA-static-utxo.md`). **OWED:** wiring this as the live backend. The dependency table below is **part of the proof**, not just documentation.
> **Cluster:** MEM-OPT-UTXO-DISK · **Prior:** S2b redb anchor (`253ee718`).

## 1. The era-aware UTxO-dependency table (the proof artifact)

For each era: the tx-body fields inspected for UTxO dependencies, the validation rules that consume them, and whether each must be present in the resolved view. The live admission path decodes to **Conway** (`DecodedTx.body: ConwayTxBody`) — the structural superset.

| Era | Tx-body fields read for UTxOs | Validation rules consuming them | Required in resolved view |
|---|---|---|---|
| **Byron** | `inputs` (`Vec<ByronTxIn>`) | input resolution, value conservation | spends |
| **Shelley** | `inputs` (`BTreeSet<TxIn>`) | input resolution, value conservation, witnesses | spends |
| **Allegra** | `inputs` | + validity interval | spends |
| **Mary** | `inputs` | + multi-asset value | spends |
| **Alonzo** | `inputs`, `collateral_inputs` | `check_inputs_present` (BadInputsUTxO over spends ∪ collateral); `sum_collateral` + collateral-% (O-27); Plutus **script context** = `build_resolved_utxos` over spends ∪ collateral | spends **+ collateral** |
| **Babbage** | `inputs`, `collateral_inputs`, `reference_inputs` | `check_inputs_present` over spends ∪ collateral ∪ reference; collateral; Plutus **script context** over spends ∪ collateral ∪ reference; reference scripts | spends **+ collateral + reference** |
| **Conway** | `inputs`, `collateral_inputs`, `reference_inputs` | same as Babbage (governance rules read no UTxO) | spends **+ collateral + reference** |

**Closed-era rule:** the danger is not the obvious spend inputs — it is **script/context construction**, which for Babbage/Conway pulls **reference inputs** and the script-context UTxO view. The extractor MUST include reference inputs for Babbage/Conway; a Shelley-era shape (spends only) applied to a Conway body would skip them. The live extractor is over `ConwayTxBody`, which carries all three classes, and a completeness test fails if any class is dropped.

## 2. The single authority
`collect_required_txins(&ConwayTxBody)` = `inputs ∪ collateral_inputs ∪ reference_inputs` — the EXACT set the Conway validator's `all_inputs` feeds to `check_inputs_present` and the Plutus script-context resolver. The Conway validator is refactored to **use** this function, so the pre-resolve set and the validated set are the same set by construction (no drift, no hidden read).

## 3. Intra-block dependency (a wiring note, not an extractor concern)
A tx can spend an output produced by an earlier tx in the SAME block — that output is NOT in the anchor yet. So block-level pre-resolve seeds the resolved working-set from the anchor, then block application adds each tx's produced outputs to the working-set as it goes (sequential), so a later tx resolves an intra-block output from the working-set. An input that is neither in the anchor NOR produced earlier in the block is genuinely missing → fail closed (BadInputs). This is handled in the **wiring** step (resolve + apply), not the extractor.

## 4. API shape (locked)
- `collect_required_txins(tx_or_block) -> BTreeSet<TxIn>` — era-aware, closed, **deterministic sorted set** (never a HashSet).
- `resolve_required_txins(anchor, required) -> BTreeMap<TxIn, TxOut>` — RED; reads the anchor.
- `validate(block, resolved_view, other_state)` — BLUE consumes ONLY the resolved view.
- **AVOID** `validate(block, store_that_might_hit_disk)` — that reintroduces the boundary violation under a nicer name.

## 5. Acceptance criteria
- [x] one authoritative extractor of all required TxIns (this slice)
- [x] era-aware + closed (Conway includes reference inputs; completeness test)
- [x] returns a deterministic sorted `BTreeSet<TxIn>` (not a HashSet)
- [x] resolved working-set is `BTreeMap<TxIn, TxOut>` (a `UtxoStore`) — `WorkingSet`
- [x] BLUE validation receives only the resolved view — the verdict-equivalence test validates over `WorkingSet`
- [x] no code path where BLUE can call redb / trigger disk I/O — gate asserts no `redb`/`UtxoAnchor` in ade_ledger
- [x] negative tests: missing pre-resolved inputs fail closed (`resolved_view_missing_input_fails_closed`)
- [x] equivalence: resolved-view verdict == full-UTxO verdict (component level; live WAL/block-hash equivalence is the integration step)

## 6. Status
- **Enumeration DONE:** the table + `collect_required_txins` + the Conway validator using it; completeness/determinism tests.
- **Resolved-view wiring DONE (component level):** `WorkingSet` (seed + `apply_tx_acceptance` transitions; impls `UtxoStore`); `UtxoAnchor::resolve_required` (RED — the sole anchor read for validation); verdict-equivalence + intra-block produced-then-spent + missing-fail-closed proofs; `ci_check_utxo_pre_resolve.sh` (incl. the no-`redb`-in-BLUE + no-cache guardrails).
- **NEXT — live integration:** wire `collect_required_txins_block → resolve_required → WorkingSet → validate → anchor.commit_block` into the admission path; one redb write-txn per block; prove WAL post_fp + incremental fp + block-hash agreement unchanged on the live corpus.
- **Cache is a SEPARATE later slice** — not folded here; proven output-invariant under clear/evict/capacity change.
