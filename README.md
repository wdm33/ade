# Ade

A Cardano block-producing node written in Rust.

> **Status**: Early stage — scaffolding and core type definitions in progress.

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

## License

MIT OR Apache-2.0
