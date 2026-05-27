# Unsigned-header pre-image fixtures — OQ-S-A proof obligation

> **Phase:** PHASE4-N-S-A A1 pre-flight capture.
> **Captured:** 2026-05-27.
>
> **Source:** the existing
> `ade_testkit::validity::corpus::ConwayValidityCorpus`
> (`/home/ts/Code/rust/ade/.fixtures/conway_validity_corpus/`),
> which contains real Conway-era block bytes extracted from
> mainnet snapshots. These blocks are already canonical to
> Ade's validator: `header_input::decode_block(block_bytes)`
> succeeds on every entry and produces a structured
> `DecodedBlock` whose `header_input.kes.header_body_bytes`
> field IS the exact CBOR pre-image the KES signature was
> produced over (per the field docstring at
> `crates/ade_core/src/consensus/header_summary.rs:57`).

## §1 Reference authority

The validator-side authority for "unsigned-header pre-image
bytes" is:

```rust
let decoded = ade_ledger::block_validity::header_input::decode_block(block_bytes)?;
let pre_image: &[u8] = &decoded.header_input.kes.unwrap().header_body_bytes;
```

These bytes are the exact CBOR encoding of `ShelleyHeaderBody`
(the first element of the outer `[header_body, kes_signature]`
header array). cardano-node's `verify_kes` consumes them
verbatim (line 106 of `ade_core/src/consensus/kes_check.rs`).

## §2 N-S-A A2 byte-match test contract

The producer-side recipe `unsigned_header_pre_image(...)`
(landing in A2) MUST produce byte-identical output to
`decode_block.header_input.kes.header_body_bytes` for every
block in the corpus.

The byte-match test (`unsigned_header_preimage_matches_decode_block_extraction`)
loads a corpus block at a stable index, decodes via
`header_input::decode_block`, extracts the canonical pre-
image, then calls the producer-side recipe with inputs
derived from the same `DecodedBlock` and asserts byte
equality.

## §3 No committed binary fixture

The corpus blocks are committed at
`.fixtures/conway_validity_corpus/` (the testkit
`load()`-able artifact). The reference pre-image bytes are
derived at test runtime from `decode_block`. A separate
binary fixture file is **not** committed because:

1. The corpus IS the canonical input.
2. `decode_block` IS the validator-side authority.
3. Re-deriving at runtime ensures the test stays in sync
   with any future corpus updates without manual
   re-capture.

## §4 Recipe shape (A2 deliverable)

```rust
pub fn unsigned_header_pre_image(
    slot: SlotNo,
    block_no: BlockNo,
    prev_hash: Hash32,
    vrf_vk: VrfVerificationKey,
    vrf_proof: VrfProof,
    vrf_output: VrfOutput,
    issuer_vkey: [u8; 32],         // cold VK encoded in header
    hot_vkey: [u8; 32],            // KES VK from opcert
    op_cert_counter: u64,
    op_cert_kes_period: u64,
    op_cert_signature: [u8; 64],   // cold-signed sigma
    body_hash: Hash32,
    body_size: u32,
    protocol_version: ProtocolVersion,
) -> UnsignedHeaderPreImage;
```

A2 implementation extracts these inputs from a
`DecodedBlock.header_input` for fixture comparison; the
recipe rebuilds the same CBOR encoding.

## §5 Reference field source

Mapping `DecodedBlock.header_input` → recipe inputs (A2 will
extract these in the byte-match test):

| Recipe input | Source in `HeaderInput` |
|---|---|
| `slot` | `header_input.slot` |
| `block_no` | `header_input.block_no` |
| `body_hash` | `header_input.body_hash` |
| `vrf_vk` | `header_input.vrf_vk` |
| `vrf_proof` + `vrf_output` | `header_input.vrf` (Praos variant) |
| `hot_vkey` | `header_input.kes.unwrap().kes_vkey` (must be 32 bytes) |
| `issuer_vkey` | `header_input.kes.unwrap().issuer_vkey` (must be 32 bytes) |
| `op_cert_counter` | `header_input.op_cert_counter` |
| `op_cert_kes_period` | `header_input.op_cert_kes_period` |
| `op_cert_signature` | `header_input.kes.unwrap().op_cert_signature` (must be 64 bytes) |
| `body_size` | derived from the encoded body byte length (not in HeaderInput; A2's test computes it from the block bytes) |
| `protocol_version` | not in `HeaderInput`; A2's test extracts via re-decoding the inner block CBOR |

If any of these aren't reachable from `HeaderInput` at A2
test time, A2 documents the gap (and N-S-A A4 may need to
extend `HeaderInput` or add a helper accessor — recorded as
a sub-task before A4 close).
