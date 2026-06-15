# Slice MEM-OPT-OPS S2 — streaming seed import (remove the import peak, byte-identical UTxO)

> **Status:** Merged — streaming import (byte-identical, hermetic + live) + the `seed_import` VmHWM tap. Import peak 6.56 → 3.25 GiB (−50.5%), `initial_ledger_fp` == S1's, replay `agreed`.
> **Cluster:** MEM-OPT-OPS (primary invariant `OP-MEM-02`; core equivalence `DC-MEM-06`, `DC-WAL-03`)
> **Cluster doc:** `docs/clusters/MEM-OPT-OPS/cluster.md` · **Grounding:** `docs/planning/mem-opt-grounding.md` · **Prior:** S1 (`861757f4`)

## 2. Slice Header

### Cluster Exit Criteria Addressed
- [x] **CE-OPS-2** (`OP-MEM-02`, IMPORT): a committed run with streaming import shows a **reduced import peak** — the **`seed_import` measurement point's `VmHWM`** (captured right after `import()`, before the snapshot write) strictly below the whole-buffer import peak — the imported UTxO **fingerprint byte-identical** to the non-streaming import (a hermetic replay test), replay verdict `agreed`. **MET 2026-06-15** — `docs/evidence/mem-opt-ops-s2-import-preprod-memory.{jsonl,md}` (sha256 `70151e45…`): seed_import VmHWM 3,405,288 < whole-buffer 6,874,028 kiB (−50.5%); `initial_ledger_fp_hex` == S1's `fb7cb12a…`; `replay_verdict=agreed`, 0 diverged, 9 admitted. **Metric correction:** the *run-end* VmHWM is confounded by the later `chain.db` snapshot serialization (~8 GiB), so the import peak is measured at the dedicated `seed_import` tap (see §9 / the snapshot finding).

Out of scope: the owned-footprint `smaps_rollup` sampler + RSS ceiling (S3); on-disk UTxO; compact TxOut.

---

## 3. Implementation Instruction
Replace whole-buffer JSON ingestion with **streaming** ingestion in the sole seed-import authority. Reuse the EXACT per-entry conversion (`parse_txin_key` + `build_canonical_tx_out`); the streamed canonical `BTreeMap<TxIn,TxOut>` must be byte-identical to the whole-buffer result. No BLUE/semantic change. No best-effort recovery. No seed-format change.

---

## 4. Intent
Remove the seed-import memory **peak** by replacing whole-buffer JSON ingestion (`fs::read` the 3.8 GB file + `serde_json::from_slice` into the intermediate `RawUtxoMap`, both held simultaneously → ~6.8 GB peak) with **streaming** ingestion: `serde_json::Deserializer::from_reader` over a `BufReader<File>`, converting each `(TxIn, TxOut)` and inserting into the canonical `BTreeMap` *as it is parsed* — never materializing the file buffer or the intermediate map. S1 *returned* the retained peak after the fact; S2 prevents the spike from happening.

## 5. Scope
- **Modules / crates:** `crates/ade_runtime/src/seed_import/importer.rs` (RED) — the file entry `import_cardano_cli_json_utxo` switches to a new streaming path (`CanonicalUtxoSink`, a `DeserializeSeed`/`Visitor`); the whole-buffer `import_cardano_cli_json_utxo_from_bytes` is **retained** as the equivalence oracle + in-memory test helper. Evidence: `crates/ade_node/src/admission_log/event.rs` + `writer.rs` + `convergence_evidence.rs` add the run high-water `rss_hwm_kib` to `MemorySummary` (GREEN, captures the import peak). New gate `ci/ci_check_mem_opt_s2_import_peak.sh`.
- **State machines affected:** none.
- **Persistence impact:** none — the imported `UTxOState` / WAL / checkpoint are byte-identical (the obligation).
- **Network-visible impact:** none.
- **Out of scope:** S3's owned sampler/ceiling; the on-disk UTxO; any change to the accepted seed JSON format.

## 6. Execution Boundary
- **BLUE:** none. The UTxO type, `parse_txin_key`, `build_canonical_tx_out`, `compute_utxo_fingerprint`, and the WAL/checkpoint are untouched — the streaming path only changes *how bytes are fed* into the unchanged conversion.
- **GREEN:** the evidence comparison (the hermetic streamed-vs-whole-buffer fingerprint test) + the `rss_hwm_kib` evidence field + the replay-fingerprint validation.
- **RED:** the streaming file I/O + JSON ingestion mechanics (`BufReader<File>`, the `serde` streaming `Visitor`), and the `/proc` VmHWM read (existing `rss_sampler::sample_vm_hwm_kib`).

## 7. Invariants Preserved (the proof obligations)
The streaming import must produce, on the **same seed file**:
1. the **same recovered anchor** (the seed-point slot/hash is unchanged — the import only builds the UTxO),
2. the **same imported UTxO semantics** (same per-entry conversion, reused verbatim),
3. the **same canonical-key fingerprint** — `compute_utxo_fingerprint(streamed) == compute_utxo_fingerprint(whole_buffer)` — THE core check (`DC-MEM-06`: the fingerprint is over canonically-encoded keys, never store/iteration/parse order),
4. the **same WAL/checkpoint fingerprint** (derived from the byte-identical `UTxOState`),
5. the **same replay verdict** (`DC-WAL-03`: `agreed`),
6. a **lower peak RSS during import** (`VmHWM` strictly below the whole-buffer peak).

The BTreeMap is keyed by `TxIn` and ordered by canonical key, so the result is independent of JSON textual order and of insertion order — streaming and whole-buffer converge to the identical map.

## 8. Invariants Strengthened
- **`OP-MEM-02`** (toward): removes the avoidable import spike. **Stays `declared`** — S2 measures `VmHWM`/`VmRSS`, NOT the owned `Private_Dirty`/`RssAnon` metric (S3); no OP-MEM-02 flip from VmRSS alone.
- **`DC-MEM-06`** (exercised live): the streamed import is proven to compute the identical canonical fingerprint — the "no reorder/drop/duplicate/normalize/reinterpret" guarantee, now on the streaming path.

## 9. Design Summary
- `CanonicalUtxoSink { utxos: &mut BTreeMap<TxIn,TxOut>, conv_err: &mut Option<JsonSeedError> }` implements `DeserializeSeed` → `deserialize_map`; its `Visitor::visit_map` loops `next_key::<String>()` + `next_value::<RawUtxoEntry>()`, converts via the unchanged `parse_txin_key` + `build_canonical_tx_out`, and `insert`s — discarding each raw key+entry. Only ONE `RawUtxoEntry` is alive at a time.
- **Fail-closed:** a conversion error (`BadTxInKey` / value / script) is stashed into `conv_err` and halts the parse with a serde error; the caller surfaces the stashed `JsonSeedError` (no swallowing). After the map, `Deserializer::end()` rejects trailing data. Malformed JSON → `serde_json::Error` propagated. No best-effort recovery.
- **Unique-outref enforcement (M1, from the per-slice IDD review):** distinct JSON key strings can collide on one canonical `TxIn` (uppercase vs lowercase hex; `#0` vs `#00`) — and the whole-buffer (String-sorted) and streaming (textual-order) paths would otherwise pick *different* survivors → different fingerprint. **Both paths now reject ANY duplicate `TxIn` fail-closed** (`JsonSeedError::DuplicateTxIn`), so the import is **byte-identical-or-rejected**, never an order-dependent survivor. A UTxO dump has unique outrefs by construction; this is a no-op on the canonical cardano-cli seed (verified live: fp == S1's).
- `import_cardano_cli_json_utxo(path)` = `File::open` → `BufReader` → `Deserializer::from_reader` → `CanonicalUtxoSink` → `compute_utxo_fingerprint`. Same `(UTxOState, UtxoFingerprint)` return; all 3 production call sites (node_lifecycle, produce_mode, admission/bootstrap) inherit the peak reduction.
- **Evidence (corrected after the live run):** the import peak is captured at a dedicated **`seed_import` measurement point** — `rss_hwm_kib` = the `VmHWM` sampled in bootstrap RIGHT AFTER `import()` returns, **before** the chain.db snapshot write. Both `MemoryMeasure` and `MemorySummary` gain `rss_hwm_kib` (GREEN closed field). **Why a dedicated tap (not the run-end summary):** the run-end `memory_summary.rss_hwm_kib` is confounded — it records a LATER, larger transient (the UTxO→`chain.db` snapshot serialization, ~8 GiB; `seed_to_snapshot` holds ~4 GB serialized + the live state, and the redb `chain.db` then mmaps into gross `VmRSS`). That snapshot transient is NOT the import; it is a **separate finding / next target** (likely MEM-OPT-UTXO-DISK), and the reason the *owned* footprint (S3) is the metric that matters, not gross `VmRSS`.

## 10. Changes Introduced
- **Types:** `CanonicalUtxoSink` (RED, private). `MemorySummary` += `rss_hwm_kib: u64` (GREEN closed field). No BLUE/canonical/persisted type changes.
- **State transitions / persistence:** none.

## 11. Replay, Crash, and Epoch Validation
- **Hermetic equivalence test** (the core): for representative fixtures (multi-entry, multi-asset, inline datum, reference script), `import_cardano_cli_json_utxo_streaming` and `import_cardano_cli_json_utxo_from_bytes` produce the **identical `UtxoFingerprint`** (and identical `UTxOState`). A textual-reorder fixture proves order-independence. A malformed fixture proves both paths fail (closed).
- **Live (CE-OPS-2):** a committed `--mode admission` preprod run with the streaming binary on the same seed shows `bootstrap_complete.initial_ledger_fp_hex` == S1's `fb7cb12a…` (byte-identical import), `memory_summary{replay_verdict=agreed}`, 0 diverged, and `rss_hwm_kib` strictly below the whole-buffer import peak (the A2/S1 ~6.87 GB footprint).

## 12. Mechanical Acceptance Criteria
- [x] `cargo test -p ade_runtime` green incl. the streamed-vs-whole-buffer fingerprint-equivalence (`streaming_matches_whole_buffer_across_fixtures`, 10 fixtures) + the malformed/trailing/conversion-error fail-closed negative tests (`seed_import` module: 31 passed).
- [x] `cargo test -p ade_node` green (the `rss_hwm_kib` field + `seed_import` point round-trip through the writer; closed-vocab gates green; 309 lib + all binaries).
- [x] `ci/ci_check_mem_measure_evidence.sh` green (validates the committed S2 transcript: closed vocab incl. `seed_import`, replay `agreed`, sha256-bound).
- [x] **CE-OPS-2 (live):** committed preprod transcript — `initial_ledger_fp_hex` == S1's `fb7cb12a…`, `replay_verdict=agreed`, 0 diverged, **`seed_import` VmHWM 3,405,288 < whole-buffer 6,874,028 kiB**; `ci/ci_check_mem_opt_s2_import_peak.sh` green.

## 13. Failure Modes
- Conversion error mid-stream (`BadTxInKey`/value/script): stashed + surfaced as the exact `JsonSeedError`; fail-closed, no partial state returned.
- Malformed JSON / trailing data: `serde_json::Error` propagated (`Deserializer::end()` rejects trailing); fail-closed.
- A streamed fingerprint that differs from the whole-buffer fingerprint: a CONSENSUS change, not a memory win — the hermetic test fails the slice.

## 14. Hard Prohibitions
Inherits the cluster's "Forbidden During This Cluster". Slice-specific (per the S2 mandate):
- **No partial streaming that silently changes semantics** — every entry is processed; the result is byte-identical or the slice is wrong.
- **No best-effort recovery from malformed JSON** — fail closed; no skipped/defaulted entries.
- **No map-iteration dependence in fingerprints** — the fingerprint is over canonical keys (`DC-MEM-06`), never parse/iteration order.
- **No change to the accepted seed format** unless old + new are explicitly versioned AND replay-proved (not in scope here — the format is unchanged).
- **No `OP-MEM-02` flip from VmRSS/VmHWM alone** — same discipline as S1; the owned metric is S3.
- No BLUE/semantic change to the conversion.

## 15. Explicit Non-Goals
- Not the owned `smaps_rollup` sampler / RSS ceiling (S3).
- Not the on-disk UTxO (MEM-OPT-UTXO-DISK) or compact TxOut (MEM-OPT-COMPACT).
- No seed-format evolution; no new CLI flag.

## 16. Completion Checklist
- [x] Streamed UTxO fingerprint == whole-buffer fingerprint (hermetic, 10 fixtures incl. negatives).
- [x] Fail-closed on malformed/trailing/conversion error (negative tests).
- [x] Live: `initial_ledger_fp_hex` == S1's, replay `agreed`, `seed_import` VmHWM (3.25 GiB) strictly below the whole-buffer import peak (6.56 GiB).
- [x] BLUE untouched; `OP-MEM-02` stays `declared`. Snapshot/`chain.db`-serialization transient logged as the next target.
