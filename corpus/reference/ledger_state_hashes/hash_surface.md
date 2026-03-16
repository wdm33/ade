# Ledger State Hash Surface Documentation

## Oracle Evidence Discipline

State hash reference artifacts are **version-scoped oracle evidence**, not timeless
semantic truth. Comparison is valid **only** against the pinned oracle version.

## Haskell Type

The state hash is derived from:

- **Type:** `ExtLedgerState (CardanoBlock StandardCrypto)`
- **Module:** `Ouroboros.Consensus.Ledger.Extended`
- **Package:** `ouroboros-consensus`

## Serialization Path

- **Encoder:** `encodeLedgerState` via `Codec.Serialise` / `cardano-binary`
- **Hash:** Blake2b-256 of the serialized `ExtLedgerState`
- **Surface:** The serialization includes both the ledger state and the consensus
  (chain-dependent) state from the extended ledger

## Byte Authority Classification

This artifact is **oracle-derived evidence**:
- NOT project-canonical internal encoding
- NOT protocol wire-byte surface
- Falls outside both branches of the Byte Authority Model
- Used exclusively for differential comparison

## Version Stability

`ExtLedgerState` serialization is **NOT stable** across node versions:
- Hard fork combinator transitions change the state shape
- Era-specific ledger state changes affect serialization
- Consensus library updates may change the encoding
- The `Serialise` instance is an internal implementation detail

## Pinned Oracle Version

- **cardano-node version:** 10.6.2
- **cardano-node git revision:** 0d697f14
- **Extraction source:** LedgerDB snapshots or LocalStateQuery

## Reproduction Instructions

1. Obtain cardano-node 10.6.2 (git rev 0d697f14) with synced ChainDB
2. For LedgerDB method: read `ExtLedgerState` from LedgerDB at target block
3. Serialize with `encodeLedgerState` and compute Blake2b-256
4. For LocalStateQuery method: query state at block point, serialize, hash

## Stability Check

Re-extracting from the same node version and data MUST produce identical bytes.
If bytes differ, the extraction method has a nondeterminism bug.

## Recovery Model Note

Full replay from genesis vs. ChainDB's audited snapshot + forward replay
produce the same `ExtLedgerState` at any given block point. The LedgerDB
stores periodic snapshots; the node replays forward from the nearest snapshot.
Do not overstate the recovery surface when documenting hash provenance.
