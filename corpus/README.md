# Corpus — Golden Mainnet Test Fixtures

This directory stores raw mainnet CBOR block and transaction fixtures
for all seven Cardano eras. These are opaque byte blobs with provenance
metadata — no decoding, parsing, or semantic interpretation is performed.

## Directory Layout

```
corpus/
├── README.md
├── acquire_fixtures.sh      # Fetches golden blocks from Blockfrost API
├── verify_checksums.sh       # Validates SHA-256 checksums against manifests
└── golden/
    ├── byron/
    │   ├── manifest.toml     # Provenance metadata for all fixtures in this era
    │   ├── blocks/           # Raw CBOR block files
    │   └── transactions/     # Raw CBOR transaction files (deferred)
    ├── shelley/              # Same structure
    ├── allegra/
    ├── mary/
    ├── alonzo/
    ├── babbage/
    └── conway/
```

## Provenance Model

Every fixture has full provenance tracked in the era's `manifest.toml`:

| Field           | Description |
|-----------------|-------------|
| `file`          | Relative path to the CBOR file |
| `era`           | Cardano era name |
| `type`          | Fixture type: "block" or "transaction" |
| `height`        | Block height on mainnet |
| `hash`          | Block hash (hex, from Blockfrost) |
| `sha256`        | SHA-256 digest of the stored file |
| `source`        | Exact API version or tool used |
| `fetch_tool`    | Tool and version used for acquisition |
| `fetch_date`    | ISO date when the fixture was fetched |
| `reproducibility` | Executable instructions to reproduce the fetch |

All fields are mandatory. No placeholders or TODOs are permitted.

## Acquisition

Run `acquire_fixtures.sh` with a Blockfrost project ID:

```sh
BLOCKFROST_PROJECT_ID=mainnetXXX ./corpus/acquire_fixtures.sh
```

The script fetches one golden block per era from the Blockfrost mainnet API,
stores raw CBOR bytes, computes SHA-256 checksums, and populates manifest
entries. It is idempotent — existing files are skipped.

The script does NOT decode, parse, normalize, or interpret CBOR content.

## Verification

Run `verify_checksums.sh` to validate all fixtures:

```sh
./corpus/verify_checksums.sh
```

Exits 0 if all checksums match, 1 on any mismatch or missing file.

## Constraints

- Bytes are stored exactly as received from the API (no re-encoding)
- No CBOR decoding or parsing of fixture content
- No domain types or semantic interpretations
- No protocol transcript fixtures (deferred to Phase 0B/T-06)
- Transaction directories are scaffolded but not yet populated
