# Shelley Bootstrap Contract — Phase 2B

**Status**: Draft
**Date**: 2026-03-18

## Problem Statement

To run Shelley UTxO-aware replay, we need a starting state at the Shelley era boundary. Unlike Byron (where genesis UTxO is well-defined), the Shelley starting state is the result of applying ~4.8 million Byron blocks plus the HFC transition — we cannot derive it from first principles in reasonable time.

## Import Strategy

**Approach**: Load Shelley starting state from the oracle's ExtLedgerState dump, the same way Byron's genesis UTxO was loaded from `slot_0.bin`.

**Source**: `corpus/ext_ledger_state_dumps/shelley/slot_10800019.bin` (44MB, state after first Shelley block)

**Alternative**: If a pre-first-block dump becomes available, use that instead for exact boundary state.

## What Is Imported (Authoritative for Replay)

| Component | Source location in dump | Used for |
|-----------|----------------------|----------|
| UTxO set (84,609 entries) | NewEpochState → EpochState → LedgerState → UTxOState | Input resolution, conservation check |
| Protocol parameters | NewEpochState → EpochState → PParams | Fee calculation, min UTxO |
| Epoch number | NewEpochState.epochNo (222) | Epoch boundary tracking |

These are the components that affect transaction validation verdicts.

## What Is Oracle-Only Context (Not Authoritative)

| Component | Why not authoritative |
|-----------|---------------------|
| Stake distribution snapshots (mark/set/go) | Not needed for tx validation verdicts |
| Reward accounts | Not needed for basic tx validation |
| Pool distribution | Not needed for tx validation |
| HeaderState (AnnTip, ChainDepState) | Consensus-layer, not ledger-layer |
| Transition trigger | HFC mechanism, not tx validation |

These components exist in the dump but are NOT loaded into Ade's LedgerState. They are outside the comparison surface for Phase 2B.

## Shelley UTxO Key Format

The oracle's on-disk UTxO map uses Haskell's compact encoding:
- Key: `array(2) [uint(output_index), bytes(28)]` — 28-byte key is Blake2b-224 of the full TxId
- Value: `array(4) [address_struct, slot, coin, optional_datum_hash]`

This differs from the transaction-level TxIn encoding (`[bytes(32), uint]`). The compact form saves ~4 bytes per entry at the cost of needing a mapping table.

**Implication**: Direct entry-by-entry UTxO set equality (as done for Byron) requires understanding this compact key encoding. The 28-byte key is NOT the same as `ade_types::TxIn { tx_hash: Hash32, index: u16 }`.

## Proof Obligation

Before Shelley replay counts as closed, we must verify:
1. The imported UTxO count matches the oracle dump
2. Verdict agreement on all 1,500 Shelley blocks
3. UTxO set equality at checkpoint(s) where dumps are available

## Translation Rules

No translation is performed. The dump is loaded as-is. The imported state is treated as an opaque starting point whose correctness is guaranteed by the oracle. This is the "Haskell-side extractor as oracle" model (Option 3).

## Evidence Artifacts

- Oracle manifest: `corpus/contiguous/oracle_manifest.toml`
- Starting state dump: `corpus/ext_ledger_state_dumps/shelley/slot_10800019.bin`
- Comparison dump: `corpus/ext_ledger_state_dumps/shelley/slot_10800027.bin`
- Contiguous blocks: `corpus/contiguous/shelley/` (1,500 blocks)
- State hashes: `corpus/contiguous/shelley_state_hashes.txt` (1,500 entries)
