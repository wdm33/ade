# Corpus — Golden Fixtures & Reference Oracle Data

This directory stores mainnet CBOR block fixtures and reference oracle
data extracted from the Haskell Cardano node for differential comparison.

## Directory Layout

```
corpus/
├── README.md
├── acquire_fixtures.sh       # Status report for golden block inventory
├── verify_checksums.sh       # Validates SHA-256 checksums against manifests
├── golden/                   # Raw mainnet CBOR blocks (Phase 0A)
│   ├── {era}/
│   │   ├── manifest.toml    # Provenance metadata per fixture
│   │   ├── blocks/          # Raw CBOR block files
│   │   └── transactions/    # Scaffolded, not yet populated
├── reference/                # Oracle reference data (Phase 0B)
│   ├── block_fields/        # Decoded block fields (JSON) for differential comparison
│   ├── ledger_state_hashes/ # Ledger state hashes at block boundaries
│   └── protocol_transcripts/ # Demuxed mini-protocol message transcripts
└── tools/                    # Extraction scripts (run on node host)
    ├── .env.example
    ├── extract_block_fields.sh
    ├── extract_state_hashes.sh
    ├── capture_transcripts.sh
    └── demux_transcript.py
```

## Golden Blocks (corpus/golden/)

42 raw mainnet CBOR blocks across all 7 Cardano eras, extracted from
cardano-node 10.6.2 ImmutableDB (Mithril snapshot epoch 618, git rev 0d697f14).

| Era | Blocks | Era Tags |
|-----|--------|----------|
| Byron | 3 | 0 (EBB), 1 (regular) |
| Shelley | 3 | 2 |
| Allegra | 3 | 3 |
| Mary | 3 | 4 |
| Alonzo | 3 | 5 |
| Babbage | 12 | 6 |
| Conway | 15 | 7 |

Provenance tracked per fixture in `manifest.toml`: file, era, type, chunk,
block_index, era_tag, sha256, source, fetch_tool, fetch_date, reproducibility.

## Reference Oracle Data (corpus/reference/)

Each reference subdirectory has a `manifest.toml` with DC-REF-01 provenance
fields. Validated by `ci/ci_check_ref_provenance.sh`.

### Block Fields

42 JSON files (7 eras). Decoded from golden CBOR using Python cbor2 library.
Each file contains header fields extracted from the HFC block envelope.

### Ledger State Hashes

15 JSON files (6 eras: Shelley through Babbage; Conway pending). Blake2b-256
hashes of CBOR-serialized `ExtLedgerState` at block boundaries, extracted via
`db-analyser` from ouroboros-consensus-cardano 0.26.0.3. See
`ledger_state_hashes/hash_surface.md` for oracle evidence discipline.

### Protocol Transcripts

6 JSON files (Handshake, ChainSync, BlockFetch, TxSubmission2, KeepAlive,
PeerSharing). Captured via tcpdump on loopback between two cardano-node 10.6.2
instances, demuxed by `demux_transcript.py`. Byron-era block sync traffic.

## Verification

```sh
./corpus/verify_checksums.sh          # Golden block checksums
ci/ci_check_ref_provenance.sh         # Reference artifact provenance (DC-REF-01)
```
