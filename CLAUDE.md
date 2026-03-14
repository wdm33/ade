# CLAUDE.md

## Project Overview

Ade is a Cardano block-producing node written in Rust. The workspace is organized as a multi-crate project under `crates/`.

## Build Commands

- `cargo build` — Build all crates
- `cargo test` — Run all tests
- `cargo clippy` — Run linter
- `cargo fmt` — Format code

## Crate Structure

- `ade_codec` — CBOR encoding/decoding
- `ade_types` — Cardano domain types
- `ade_crypto` — Pure cryptographic verification
- `ade_core` — Ledger, consensus, and protocol logic
- `ade_testkit` — Test infrastructure
- `ade_runtime` — I/O, networking, storage, signing (imperative shell)
- `ade_node` — Binary entry point

## Git Commit Guidelines

- Commits MUST include `Co-Authored-By: Claude` trailers
- Use conventional commit format
- Focus on technical changes and business impact
