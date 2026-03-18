# Byron Era Closure Report — Phase 2B

**Status**: Closed at UTxO evidence level
**Date**: 2026-03-18
**Commit**: bb03c4b

## Evidence Summary

| Surface | Result | Test |
|---------|--------|------|
| Block decode | 1,500/1,500 | `contiguous_corpus_decode::byron_contiguous_blocks_decode` |
| Tx body decode | 0 txs (early mainnet) | `contiguous_corpus_tx_decode::byron_contiguous_tx_decode` |
| Verdict agreement | 1,500/1,500 | `differential_replay_all_eras::byron_replay_all_1500` |
| UTxO-aware replay | 1,500 blocks, 14,505 UTxO | `differential_byron_utxo::byron_replay_with_genesis_utxo` |
| UTxO set equality | 14,505/14,505 across full window | `differential_byron_utxo_full::byron_utxo_equality_full_1500_blocks` |
| Replay determinism | 2 runs identical | `differential_byron_utxo_full::byron_full_replay_determinism` |
| First divergence | None (zero mismatches) | — |

## Corpus Identity

- **Blocks**: `corpus/contiguous/byron/` — 1,500 contiguous blocks from chunk 0
- **Slot range**: 0–1,500
- **State hashes**: `corpus/contiguous/byron_state_hashes.txt` — 1,502 entries
- **Oracle dumps**: `corpus/ext_ledger_state_dumps/byron/slot_0.bin`, `slot_1.bin`
- **Genesis**: `corpus/contiguous/mainnet-byron-genesis.json` (14,505 AVVM entries)

## Oracle Manifest

- cardano-node 10.6.2 (git rev 0d697f14)
- ouroboros-consensus-cardano 0.26.0.3
- Mithril snapshot epoch 618, immutable 8419
- Comparison surface: Blake2b-256 of encodeDiskExtLedgerState
- Extraction date: 2026-03-18

## Proof Claims

**True tier**: Same genesis UTxO + same 1,500 Byron blocks = identical UTxO state, deterministically, across two independent runs.

**Derived tier**: Ade's post-replay UTxO set is entry-by-entry identical to the oracle's ExtLedgerState UTxO component (14,505 TxIn keys, 14,505 address+coin values, zero mismatches).

**Release tier**: Byron verdict agreement and UTxO equality proven on contiguous mainnet corpus, version-scoped to cardano-node 10.6.2.

**Operational tier**: Genesis UTxO loaded from ExtLedgerState binary dump via `genesis_loader::load_genesis_utxo`. Oracle UTxO extracted via CBOR navigation of the same dump format.

## Scope Limitations

- Early Byron blocks (slots 0–1,500) contain 0 transactions. UTxO invariance is expected.
- Byron transaction validation (witness verification, fee checks) is implemented but not exercised by this corpus window.
- Full Byron transaction-level validation requires a corpus window containing non-empty blocks (later Byron epochs).
- Aggregate ExtLedgerState hash comparison deferred per Option 5 (Ade-canonical + oracle-external dual-surface).

## Template for Future Eras

Each subsequent era (Shelley, Allegra, Mary) should close in this order:
1. Block decode
2. Tx body decode
3. Verdict agreement
4. UTxO-aware replay
5. UTxO set equality
6. Replay determinism

This report format is the standard for era closure.
