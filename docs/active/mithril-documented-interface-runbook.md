# Mithril documented-interface bootstrap — decision record + operator runbook

> **Status:** Decision record + operator guidance (not a cluster, not an implementation
> slice). Records how Ade satisfies `RO-MITHRIL-IMPORT-01` item (a) and provides the
> operator procedure for the documented-interface Mithril path.

## Decision

**Native decoding of Mithril ancillary / UTXO-HD / LedgerDB bytes is a Tier-4 non-goal
for this bounty path.** `RO-MITHRIL-IMPORT-01` item (a) is satisfied by the
**documented-interface path**:

```
Mithril-bootstrapped Cardano peer
  → documented cardano-cli / query extraction (cardano-cli query utxo, consensus-input extraction)
  → Ade seed_import
  → CN-MITHRIL-01 / DC-MITHRIL-02 binding (verify_mithril_binding, fail-closed)
```

A native byte path, if ever needed, belongs under `RO-GENESIS-REPLAY-01` via **certified
block replay** of ImmutableDB chunks — **not** UTXO-HD reverse engineering.

**Why** (the artifact-type spike): a Mithril *cardano-db* snapshot is the ImmutableDB
block chunks (→ forward-replay = `RO-GENESIS-REPLAY-01`, deferred); the
`--include-ancillary` ledger-state snapshot is cardano-node's **UTXO-HD InMemory**
binary, converted via `mithril client utxo-hd snapshot-converter` to LMDB/Legacy — i.e.
cardano-node's **private storage serialization**, not an Ade-stable interchange format.
Decoding it in Ade would pull cardano-node's private serialization into Ade's authority
surface, contra [[feedback-oracle-seed-then-ade-owns]] and
[[feedback-mithril-is-peer-infra-not-ade-authority]] ("documented cli interfaces over
reverse-engineering utxohd").

## Operator runbook — the documented-interface path

1. **Acquire the certified snapshot:** `mithril-client cardano-db download <DIGEST>`
   (+ `--include-ancillary`) into the peer's `db/`, verifying against the genesis
   verification key. Mithril is acquisition/peer infra, **never an Ade trust root**.
2. **Bring the peer to the certified state:** `cardano-node run` on the restored DB;
   if it uses LMDB/Legacy UTXO-HD, run `mithril client utxo-hd snapshot-converter`
   first (cardano-node-side tooling — **not** consumed by Ade).
3. **Extract the seed via documented interfaces:** `cardano-cli query utxo --whole-utxo
   --out-file utxo.json`; extract the consensus inputs (epoch nonce / stake / ASC /
   epoch window); note the **operator seed point** (slot + block hash) of the extraction.
4. **Import into Ade:** `seed_import` ingests `utxo.json` → Ade-canonical
   `(UTxOState, UtxoFingerprint)`.
5. **Bind + bootstrap:** `bootstrap_from_mithril_snapshot` (PHASE4-N-Z) mints the
   `BootstrapAnchor` with `seed_point` = the **operator-extracted** point (independent
   origin) and `seed_provenance` = the Mithril manifest's
   `{certificate_hash, certified_point, immutable_range}`, then runs
   `verify_mithril_binding` (fail-closed on mismatch — CN-MITHRIL-01 / DC-MITHRIL-02)
   before `bootstrap_initial_state`.
6. **Ade owns all state after the anchor** (forward-sync).

## What this deliberately does NOT do

- Ade does **not** decode the Mithril ledger-state ancillary (UTXO-HD/LedgerDB bytes) —
  Tier-4 non-goal.
- It does **not** forward-replay ImmutableDB chunks (that is `RO-GENESIS-REPLAY-01`).
- It does **not** add a `--mithril-manifest` CLI flag (a future operator-UX slice).

## Evidence shape (operator guidance for a future fixture/evidence slice)

When a reproducible documented-interface pass is captured (the remaining
`RO-MITHRIL-IMPORT-01` item (c), operator-witnessed), record at least: `ade_commit`,
`network`, `mithril_aggregator_endpoint`, `mithril_certificate_hash`, `mithril_version`,
`cardano_node_version`, `cardano_cli_version`, the operator seed point (slot + block
hash), the extracted `utxo.json` artifact + its sha256, Ade's recomputed
`seed_artifact_hash`, and the `verify_mithril_binding` result. This is operator
guidance; **no CI gate ships with this decision record** — a schema gate is part of the
future item-(c) evidence slice, when a real reproducible fixture exists, not before
(no document-only / theater enforcement).

## References

- [[feedback-oracle-seed-then-ade-owns]], [[feedback-mithril-is-peer-infra-not-ade-authority]].
- PHASE4-N-Z (`bootstrap_from_mithril_snapshot`, `verify_mithril_binding`, `DC-MITHRIL-02`).
- `RO-MITHRIL-IMPORT-01` (item (a) reclassified here; item (c) operator-witnessed),
  `CN-MITHRIL-01`, `CN-SEED-01`, `RO-GENESIS-REPLAY-01`.
- Mithril docs: bootstrap a Cardano node; mithril-client (`cardano-db --include-ancillary`,
  `utxo-hd snapshot-converter`).
