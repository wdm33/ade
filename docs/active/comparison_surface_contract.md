# Comparison Surface Contract — Phase 2B

**Status**: Normative
**Version scope**: cardano-node 10.6.2 (git rev 0d697f14)
**Era scope**: Byron, Shelley, Allegra, Mary
**Date**: 2026-03-18

## Purpose

This document defines exactly what oracle outputs map to what Ade outputs, where wire-byte vs project-canonical distinction applies, and what "zero divergence" means concretely. No BLUE ledger code may claim correctness without reference to this contract.

---

## True Tier — Single Byte Authority Per Surface

### Hash surface definition

The oracle comparison hash at each block boundary is:

```
Blake2b-256(encodeDiskExtLedgerState(extLedgerState))
```

Where:
- `extLedgerState` is the Haskell `ExtLedgerState` after applying the block
- `encodeDiskExtLedgerState` is the canonical CBOR serialization used by `ouroboros-consensus-cardano` for on-disk storage
- `Blake2b-256` is RFC 7693 with 32-byte output

This is the **single authoritative byte representation** for the aggregate ledger state comparison surface.

### Mismatch localizability

If the aggregate state hash diverges at block N:
1. The pre-state hash (block N-1) identifies whether prior state was already divergent
2. Sub-surface hashes (when available) identify which component diverged first:
   - UTxO set hash
   - Delegation map hash
   - Reward accounts hash
   - Protocol parameters hash

### Wire-byte authority

For transaction ID computation: `Blake2b-256(tx_body_wire_bytes)` where `tx_body_wire_bytes` comes from `PreservedCbor::wire_bytes()` — the original network bytes, never re-encoded.

For block header hash: `Blake2b-256(header_body_wire_bytes)` — same wire-byte authority.

---

## Derived Tier — Oracle Surface Mapping

### Oracle extraction method

| Surface | Oracle tool | Oracle output | Ade computation |
|---------|-------------|---------------|-----------------|
| Aggregate state hash | `db-analyser --store-ledger` | Blake2b-256 of serialized ExtLedgerState | `blake2b_256(ade_ledger_state_canonical_bytes)` |
| Transaction ID | `encodeDiskExtLedgerState` embedded TxId | Blake2b-256 of CBOR tx body | `ade_crypto::transaction_id(tx_body.wire_bytes())` |
| Block header hash | ImmutableDB secondary index | Blake2b-256 of CBOR header body | `ade_crypto::block_header_hash(header.wire_bytes())` |

### Per-era comparison surface differences

**Byron**: ExtLedgerState includes Byron-specific chain state. Byron EBBs produce state hashes but contain no transactions. The UTxO set uses Byron address format.

**Shelley**: ExtLedgerState adds delegation state, reward accounts, protocol parameters. The UTxO set uses Shelley address format. State includes stake distribution snapshots (mark/set/go).

**Allegra**: Same as Shelley with validity interval enforcement. No structural change to ExtLedgerState format.

**Mary**: ExtLedgerState UTxO entries may contain multi-asset values. The Value type changes from `Coin` to `Coin + MultiAsset`.

### PreservedCbor authority

- `PreservedCbor::wire_bytes()` governs all hash-critical paths (transaction IDs, block hashes)
- `PreservedCbor::canonical_bytes()` is project-canonical re-encoding — NEVER used for comparison against oracle
- Opaque substructures (`RawCbor`) preserve original wire bytes for fields not yet decoded

### ChainDB snapshot behavior

The oracle state hashes are computed from forward replay starting at the nearest existing ledger snapshot, not from genesis. This means:
- Byron hashes start from the genesis state (slot 0)
- Shelley hashes start from the Byron→Shelley transition state
- Allegra hashes start from the Shelley→Allegra transition state
- Mary hashes start from the Allegra→Mary transition state

The contiguous corpus captures blocks within a single era. Cross-era transitions are verified separately via HFC translation (S-18).

---

## Release Tier — Zero-Divergence Claim

### Definition

"Zero divergence" means: for every block in the contiguous corpus, `oracle_state_hash == ade_state_hash` where both are computed as Blake2b-256 of their respective canonical ledger state serializations.

### Version scope

All comparisons are scoped to:
- cardano-node 10.6.2 (git rev 0d697f14)
- ouroboros-consensus-cardano 0.26.0.3
- Mithril snapshot epoch 618, immutable 8419

A different oracle version may produce different state hashes. The comparison is NOT version-independent.

### Scope boundary

- **In scope**: Byron (1,500 blocks), Shelley (1,500 blocks), Allegra (1,500 blocks), Mary (1,500 blocks)
- **Out of scope**: Alonzo, Babbage, Conway — deferred to Phase 3

### Hash algorithm

Blake2b-256 (RFC 7693, 32-byte output) governs all comparison surface hashes.

---

## Operational Tier — Extraction Workflow

### Oracle state hash extraction

1. Start `db-analyser` from the nearest existing ledger snapshot
2. Replay forward through contiguous blocks
3. At each block boundary, serialize ExtLedgerState via `encodeDiskExtLedgerState`
4. Compute Blake2b-256 of the serialized bytes
5. Output: `SlotNo N|hex_hash|byte_offset` per line

### Corpus storage

```
corpus/contiguous/
├── byron/
│   ├── blk_00000_chunk00000_idx00001.cbor
│   ├── ...
│   └── blocks.json                          # index with blake2b_256 per block
├── byron_state_hashes.txt                   # 1,502 lines: SlotNo|hash|offset
├── shelley/
│   ├── ...
│   └── blocks.json
├── shelley_state_hashes.txt                 # 1,500 lines
├── allegra/
│   ├── ...
│   └── blocks.json
├── allegra_state_hashes.txt                 # 1,500 lines
├── mary/
│   ├── ...
│   └── blocks.json
└── mary_state_hashes.txt                    # 1,500 lines
```

### State hash file format

Each line: `SlotNo <slot>|<hex_blake2b_256>|<byte_offset>`

- `slot`: absolute slot number
- `hex_blake2b_256`: 64-character lowercase hex Blake2b-256 of serialized ExtLedgerState
- `byte_offset`: byte offset in the ImmutableDB (for reproducibility)

### CI validation

`ci/ci_check_ref_provenance.sh` validates manifest integrity.
Differential comparison runs as integration tests in `ade_testkit`.

---

## Proof Obligation

This contract must be mechanically verified on sample blocks before any BLUE ledger code claims oracle agreement. The verification is:

1. Extract state hash from oracle for a known block
2. Compute state hash from Ade ledger after applying the same block
3. Assert byte-equality of the two hashes

This verification is performed by the differential harness (`diff_ledger_sequence_rich`) using `BlockMeta` and `LedgerHashSequence`.
