# S-29 Flat Decoder Probe Results

> **Status:** Discharge obligation O-29.3 satisfied. Aiken v1.1.21's
> Flat decoder handles every Plutus script in Ade's mainnet corpus
> byte-identically.
>
> **Executed:** see `crates/ade_testkit/tests/plutus_flat_decoder_probe.rs`.
> **Oracle source:** mainnet blocks extracted from cardano-node 10.6.2
> snapshots — `corpus/contiguous/{alonzo,babbage,conway}/`.

## Summary

| Metric | Value |
|--------|-------|
| Total Plutus scripts extracted | **6 899** |
| Scripts that decode via `PlutusScript::from_cbor` | **6 899 / 6 899** |
| Scripts that round-trip (`to_cbor(from_cbor(b)) == b`) | **6 899 / 6 899** |
| Plutus V1 scripts | 6 687 |
| Plutus V2 scripts | 210 |
| Plutus V3 scripts | 2 |

Per-era breakdown:

| Era | Scripts | Decode OK | Round-trip OK | V1 | V2 | V3 |
|-----|---------|-----------|---------------|----|----|----|
| Alonzo | 55 | 55 | 55 | 55 | 0 | 0 |
| Babbage | 5 816 | 5 816 | 5 816 | 5 816 | 0 | 0 |
| Conway | 1 028 | 1 028 | 1 028 | 816 | 210 | 2 |

**Threshold requirement:** O-29.3 commitment was "100+ mainnet Plutus
txs." The probe exercises 6 899 scripts across 4 500 contiguous
mainnet blocks — 69× the minimum bar.

## Properties verified per script

1. **Decode succeeds.** `aiken_uplc::Program::<DeBruijn>::from_cbor`
   accepts the witness-set script bytes without parse errors.
2. **Round-trip is byte-identical.** Encoding the decoded program
   back via `to_cbor` produces bytes equal to the original input.
   This proves aiken's canonical Flat encoding matches the encoding
   actually used by every on-chain Plutus script in the corpus.

Properties NOT verified (scoped to later slices):

- **Hash agreement** (`Blake2b-224(cbor_bytes) == witness_script_hash`).
  Requires correlating the script bytes with the corresponding
  output's `script_data_hash` or script reference, which needs
  tx-body-level parsing not in S-29 scope. Added in S-30 / S-31.
- **Evaluation correctness.** The probe verifies encoding, not
  semantic execution. Evaluation against IOG conformance vectors is
  CE-85 (slice S-30).

## Technical findings from the probe

### Double-CBOR encoding

Initial run of the probe revealed that on-chain Plutus scripts are
stored as **double-CBOR-encoded Flat**: the witness-set map value is
a CBOR bytestring whose CONTENT is itself a CBOR bytestring whose
content is the Flat-encoded UPLC program. The outer bstr is CDDL
`bytes`; the inner bstr is the Cardano Plutus script convention.

Decoding path:

```
witness_set[3|6|7] → bstr content → bstr content → flat bytes → UPLC AST
                     │                │
                     │                └─ aiken_uplc::Program::from_cbor
                     └─ ade_ledger::witness::decode_plutus_scripts_in_witness_set
```

`PlutusScript::from_cbor` in `ade_plutus::evaluator` handles the
inner layer; `decode_plutus_scripts_in_witness_set` in
`ade_ledger::witness` handles the outer layer.

### Type parameter for on-chain scripts

`PlutusScript` internally uses `Program<DeBruijn>` (nameless
index-based representation), NOT `Program<NamedDeBruijn>`. On-chain
scripts are compiled to DeBruijn form; attempting to decode them as
NamedDeBruijn fails because the name payload is absent from the wire
encoding. Aiken's own tx evaluator uses `Program::<DeBruijn>::from_cbor`.

### Stack size

UPLC programs use deeply-nested term trees. Aiken's Flat decoder
recurses over this tree, and mainnet programs exercise recursion
depths that exceed Rust's default 2 MiB test-thread stack. The
probe launches its worker thread with a 32 MiB stack via
`std::thread::Builder::new().stack_size(32 << 20)`; this is
empirically sufficient for every script in the corpus.

Production code paths in `ade_plutus` will need to either use
`#[main]`-time stack sizing (for a binary), spawn worker threads
with adequate stack (for a server), or use an iterative/continuation
rewrite of aiken's decoder (invasive, not recommended). S-32
integration should default to spawning the evaluator on a dedicated
worker thread.

## Artifacts

- Probe implementation: `crates/ade_testkit/tests/plutus_flat_decoder_probe.rs`
- Witness extractor: `ade_ledger::witness::decode_all_plutus_scripts_in_block`
- Wrapper under test: `ade_plutus::evaluator::PlutusScript`

## Re-run instructions

```
cargo test --package ade_testkit --test plutus_flat_decoder_probe
```

No environment variables or flags required. Runs in ~11 seconds on a
warm cache. Any regression in aiken's Flat decoder at a future pin
bump, or any encoding-drift in the mainnet corpus, surfaces as a
test failure with localization to the first failing block + version
+ error message.

## Conclusion

O-29.3 discharged. `ade_plutus::PlutusScript` correctly decodes and
re-encodes every Plutus script produced by cardano-node 10.6.2 on
mainnet through epoch 508. Aiken v1.1.21 at commit
`42babe5d5fcdd403ed58ed924fdc2aed331ede4d` is the authoritative
Flat decoder for Ade; no upstream divergence surfaced by this probe.

S-29 is functionally complete. CE-85 (evaluation + conformance)
remains open pending S-30 work.
