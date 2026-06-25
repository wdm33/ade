# Ade

A Cardano block-producing node written in Rust.

> **Status**: Follows the Cardano **preview** testnet — bootstraps from a verified Mithril snapshot, follows the chain, and recovers across restarts. See **[Getting Started on Preview](docs/getting-started-preview.md)**.

## Crate Structure

| Crate | Purpose |
|-------|---------|
| `ade_codec` | CBOR encoding and decoding |
| `ade_types` | Cardano domain types |
| `ade_crypto` | Cryptographic verification |
| `ade_core` | Ledger, consensus, and protocol logic |
| `ade_testkit` | Test infrastructure |
| `ade_runtime` | I/O, networking, storage, and signing |
| `ade_node` | Binary entry point |

## Building

```sh
cargo build
cargo test
cargo clippy
cargo fmt
```

## Getting started

To run a node on the Cardano preview testnet — fetch a verified Mithril snapshot, bootstrap, and
follow the chain in three commands — see **[Getting Started on Preview](docs/getting-started-preview.md)**.

## License

MIT OR Apache-2.0
