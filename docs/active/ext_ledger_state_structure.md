# ExtLedgerState CBOR Structure Analysis

**Source**: Raw dumps from `encodeDiskExtLedgerState` (cardano-node 10.6.2)
**Date**: 2026-03-18

## Top-Level Structure

```
array(2) [LedgerState, HeaderState]
```

### LedgerState — Telescope Encoding

The HFC combinator uses a "Telescope" encoding. The telescope length = 1 + number of completed past eras.

```
array(1 + past_eras) [
  Past(era_0)?,     -- if past: array(2) [Bound_start, Bound_end]
  Past(era_1)?,     -- ...
  Current(era_N)    -- array(2) [Bound_start, era_specific_state]
]
```

**Bound**: `array(3) [epoch: uint, slot: uint, relative_time: uint]`

For Byron (era 0): telescope = `array(1) [Current]`
For Shelley (era 1): telescope = `array(2) [Past(Byron), Current(Shelley)]`
For Allegra (era 2): telescope = `array(3) [Past(Byron), Past(Shelley), Current(Allegra)]`
For Mary (era 3): telescope = `array(4) [Past(Byron), Past(Shelley), Past(Allegra), Current(Mary)]`

### HeaderState

```
array(2) [WithOrigin(AnnTip), ChainDepState(Telescope)]
```

**WithOrigin**: `array(0)` for Origin, indefinite-array `[value]` for At
**AnnTip**: NS (sum) encoding = `array(2) [eraIndex: uint, payload]`
**ChainDepState**: Same Telescope encoding as LedgerState but carrying per-era consensus state

## Byron Era Specifics

### Byron LedgerState (at era-specific position)

```
array(3) [
  WithOrigin(BlockNo),   -- array(0) | array(2) [1, blockNo]
  ChainState,            -- array(5) [...]
  Transition             -- transition trigger state
]
```

### Byron ChainState

```
array(5) [
  uint(1),               -- version tag (1 byte)
  array(2),              -- [slot_count(?), prev_hash(?)] (36 bytes)
  map(N),                -- UTxO set: TxIn -> TxOut (14,505 entries at slot 1 = 1.3MB)
  array(11),             -- update state / protocol parameters (139 bytes)
  array(2)               -- delegation state (880 bytes)
]
```

### Byron UTxO map entry format

Key: `array(2) [array(4) [u64,u64,u64,u64], uint]` — TxIn as (TxId decomposed into 4 u64s, index)
Value: `array(2) [bytes(43), uint]` — TxOut as (address CBOR, coin lovelace)

## Size Summary (Byron slot 1)

| Component | Offset | Size |
|-----------|--------|------|
| LedgerState | 1..1,363,491 | 1,363,490 bytes |
| - Telescope overhead | 1..7 | 6 bytes |
| - Byron tipBlockNo | 8..10 | 3 bytes |
| - Byron ChainState | 11..1,363,139 | 1,363,128 bytes |
|   - UTxO map (14,505 entries) | 49..1,363,120 | 1,363,071 bytes |
|   - Update state | 1,363,120..1,363,259 | 139 bytes |
|   - Delegation state | 1,363,259..1,364,139 | 880 bytes |
| - Transition | ~1,364,139 | ~1 byte |
| HeaderState | 1,363,491..1,364,257 | 766 bytes |
| **Total** | | **1,364,257 bytes** |

## Implications for Ade

To produce byte-identical hashes, Ade must serialize:
1. The full UTxO set in the same map encoding (key format, value format, ordering)
2. The update/delegation state
3. The transition trigger state
4. The full HeaderState including consensus protocol state (AnnTip, ChainDepState)

The UTxO set dominates the state size. The key encoding uses decomposed TxId (4 x u64) rather than raw 32-byte hash, which is a Haskell-specific serialization convention.

## Shelley+ Structure (TBD — inspect slot_10800019.bin)

Shelley uses versioned encoding: `encodeVersion 2 (array(3) [WithOrigin(tip), NewEpochState, Transition])`

The `NewEpochState` is the bulk of the state and includes:
- EpochState (accounting snapshots, ledger state)
- StakeDistribution
- PoolDistr
- UTxO set (grows to ~44MB by Shelley start)
