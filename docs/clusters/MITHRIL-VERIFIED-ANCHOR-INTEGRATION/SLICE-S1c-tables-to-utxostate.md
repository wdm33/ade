# SLICE: MITHRIL-VERIFIED-ANCHOR-INTEGRATION / S1c — tables → authoritative UTxOState materialization

## Claim
The Stage-2 `tables` (MemPack-decoded TxOuts) deterministically materialize into Ade's authoritative
`UTxOState`: each `DecodedTxOut` → ledger `TxOut` with hash-critical datum / reference-script bytes
PRESERVED VERBATIM, u64 output quantities carried through with NO intermediate i64 conversion, in
canonical TxIn order, fail-closed on any unsupported form — and the resulting `UTxOState` commitment is
bound to the SAME manifest point + the Stage-1 non-UTxO commitment + the Stage-2 tables commitment, with
deterministic materialization AND recovery from the persisted form. This is the authority slice that
unblocks the live FirstRun route (step 4); it does NOT touch node_lifecycle or add CLI flags.

## The gap to build — the converter (`crates/ade_ledger`)
A pure `decoded_txout_to_ledger(DecodedTxOut) -> Result<TxOut, TxOutMaterializeError>` (closed error enum):
- **No datum AND no script** → `TxOut::ShelleyMary { address, value: Value { coin, multi_asset } }`, where
  `multi_asset: MultiAsset(BTreeMap<Hash28, BTreeMap<AssetName, OutputAssetQuantity>>)` is built by
  wrapping each Stage-2 `u64` quantity into `OutputAssetQuantity(u64)` — NEVER truncated / saturated /
  i64-cast. (Byron only if the address header byte is Byron.)
- **Datum OR script present** → `TxOut::AlonzoPlus { raw, address, coin }` where `raw` is canonical
  Conway TxOut CBOR (map form, keys ascending):
  - key 0 = address bytes;
  - key 1 = value: `coin` (uint) if ada-only, else `[coin, {policy: {name: qty}}]` with each `qty` a CBOR
    UNSIGNED int (u64) and the policy/name maps in canonical sorted order;
  - key 2 (if datum) = datum_option: `DatumField::Hash(h)` → `[0, h]`; `DatumField::Inline(bytes)` →
    `[1, #6.24(bytes)]` — the inline bytes embedded VERBATIM inside the tag-24 (CBOR-encoded-CBOR), NEVER
    re-decoded/re-encoded;
  - key 3 (if script) = reference script: `#6.24([type, script_bytes])` where `ScriptField::Native` →
    `[0, native_bytes]`, `ScriptField::Plutus{version,bytes}` → `[1+version, plutus_bytes]` (V1→1, V2→2,
    V3→3); the script bytes embedded VERBATIM.
  - `coin` (the AlonzoPlus quick-access field) = the decoded coin; `address` = the decoded address bytes.
- The datum / script bytes are the HASH-CRITICAL identity Cardano hashes; they are embedded verbatim
  (tag-24) and never reconstructed. ade_plutus reads `raw` directly for the ScriptContext.

## Iteration + materialization (`crates/ade_runtime` or `ade_ledger`)
- Iterate the `tables` CBOR map in canonical ASCENDING TxIn order (as `decode_tables_commitment` already
  verifies — assert ascending, terminal otherwise).
- Parse each TxIn key (32-byte txid + 2-byte big-endian index → `TxIn { tx_hash, index }`).
- `read_txout` → `DecodedTxOut` → `decoded_txout_to_ledger` → ledger `TxOut`.
- Accumulate a `BTreeMap<TxIn, TxOut>` in canonical order → `UTxOState::from_map`.
- Era-bound to Conway (from the Stage-1 era). FAIL-CLOSED on any unsupported tag / address form / script
  language / value tag / non-ascending key — a STRUCTURED terminal error, NEVER an opaque keep-bytes
  fallback.

## Commitment binding (point coherence extended to the UTxO)
- The materialized `UTxOState` → `fingerprint_utxo_v2` (the Ristretto255 set commitment).
- Produce a binding record over: the manifest certified point + the Stage-1
  `NativeSnapshotNonUtxoState` commitment + the Stage-2 `decode_tables_commitment` + the UTxO
  fingerprint_v2. A mismatch (wrong point / wrong Stage-1 / wrong Stage-2) is TERMINAL — the UTxO
  authority is visible only when all bind to the one manifest point.

## Cross-checks (slice-entry obligations; required — like Stage-2's PO#1)
- ROUND-TRIP: `DecodedTxOut → AlonzoPlus.raw → (decode the raw via ade_codec babbage/conway TxOut decode)
  → the SAME address / value / datum_option / script_ref` (the re-encode is decodable + lossless).
- BYTE PRESERVATION: the inline-datum and script bytes inside `raw` are byte-identical to the
  `DecodedTxOut`'s `DatumField::Inline` / `ScriptField` bytes.
- cardano-cli ORACLE: a sample of materialized TxOuts (incl. inline-datum, reference-script, and
  multi-asset entries) cross-checked against `cardano-cli query utxo` for the SAME TxIns — address +
  value + datum + script agree (the live preview/preprod node, as in Stage-2).
- REAL-SNAPSHOT: materialize a sample of the real preprod `tables` → `UTxOState`; the fingerprint_v2 is
  DETERMINISTIC (same tables → same commitment) and the binding holds.

## Acceptance
- deterministic: same `tables` + manifest → byte-identical `UTxOState` commitment + binding record;
- u64 > i64::MAX outputs materialize and survive persist → recover exactly;
- datum / script bytes preserved verbatim in `raw` (asserted);
- canonical TxIn ordering (asserted);
- fail-closed negatives: unsupported tag / address / script-language / value-tag / non-ascending key;
- the UTxO commitment binds to the manifest point + Stage-1 + Stage-2 commitments (terminal on mismatch);
- recovery from the persisted form is deterministic (persist `UTxOState` → recover → identical fingerprint).

## Scope fence
Materialization + commitment binding ONLY. NO live CLI flags, NO node_lifecycle changes (step 4, gated
on this). NO Conway per-byte min-UTxO calculator (DC-LEDGER-PARAMS-01). Reuse the value model
(DC-LEDGER-VALUE-01), the ledger `TxOut` / `UTxOState::from_map` / `fingerprint_utxo_v2`, the Stage-2
`read_txout`, and the existing CBOR primitives — do not fork them.
