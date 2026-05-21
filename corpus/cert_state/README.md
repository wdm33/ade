# Conway cert-state accumulation corpus (PHASE4-B4, CE-B4-5)

Backs `crates/ade_ledger/tests/cert_state_corpus.rs`. The certificates are built
in-test as canonical CBOR (no external fixtures), so this directory holds only
this README.

## What is mechanically closed (CE-B4-5)

1. **Positive (synthetic):** a real-shaped Conway cert sequence (stake-key
   registration → pool registration → delegation) accumulates into the correct
   B4-owned `CertState` (delegation + pool) under controlled state.
2. **Replay:** the accumulation is byte-identical across two runs (`T-DET-01`).
3. **Adversarial (no false accept):** malformed / unknown-tag (≥19) /
   removed-tag (5/6) / truncated / trailing-bytes / unregistered-delegation cert
   arrays each reject — never a silent accept.

## Environment-blocked open obligation (NOT closed here)

The **real epoch-576 cert-state-vs-cardano-node oracle** is environment-blocked,
identical to the B3-S5 constraint: the epoch-576 ledger-state / UMap snapshot was
deleted post-extraction and is **not** in this repo (see
`corpus/validity/conway_epoch576/README.md`). Real Conway certs cannot be
accumulated at `track_utxo=true` here because their prior delegation/pool state
would not resolve — `apply_conway_cert` fail-closes on
`StakeNotRegistered`/`PoolNotRegistered` for credentials registered in
blocks/epochs not present locally.

Per the project tier doctrine, the real-corpus `CertState` agreement is
**reclassified environment-blocked**; the synthetic positive + replay +
adversarial gate above is the mechanical closure that B4 ships. This mirrors
`DC-TXV-06`/`DC-TXV-03` (B3), whose real oracle is the same documented open
obligation.
