# SLICE — MITHRIL-VERIFIED-ANCHOR-IMPORT (bounty-facing bootstrap)

## Intent
Bootstrap Ade from a verified **Mithril Cardano DB snapshot** by decoding its V2 LedgerDB **natively**
— no cardano-node, no cardano-cli, no live-tip race — producing the exact state the existing
`bootstrap_from_mithril_snapshot` already consumes.

## Why this is the right path (the correction)
The bounty bootstrap requirement is: the judge starts Ade from a recent Mithril snapshot (or genesis)
and syncs to tip. The cardano-cli exporter (`f09cc0ec`) is NOT that path — its 2.9GB UTxO query takes
138s while the immutable tip drifts 6 blocks, so it cannot produce a point-consistent seed. It is
reclassified as **auxiliary CLI export / compatibility-oracle tooling** (diagnostics + differential
checks), never the judge bootstrap.

## What already exists (reuse — do NOT rebuild)
- `bootstrap_from_mithril_snapshot<D,S>` (mithril_bootstrap.rs): the full routing — `import_mithril_manifest`
  → mint anchor → `verify_mithril_binding` (BLUE cross-check) → `bootstrap_initial_state` → persist the
  seed-epoch sidecar. **Wired at the node FirstRun** ("its first non-test caller"). It takes
  `seed_ledger: LedgerState` + `seed_consensus_inputs` as parameters.
- The Mithril manifest (`RawMithrilManifest`): certificate hash, network/genesis, `certified_point{slot,
  block_hash}`, immutable range. `verify_mithril_binding` binds the seed to it.
- `encode/decode_cert_state` + `PoolParams` (the output types); `decode_babbage_tx_out` (ade_codec, the
  tables TxOut values); `extract_praos_nonces` (the state's five nonces); `TxIn` / `UTxOState`.
- The ECA epoch-continuity machinery (consume the bootstrapped state afterwards).

## The gap (this slice): the native V2 LedgerDB decode
`node_lifecycle` says it explicitly: **"NO native Mithril UTXO-HD/LedgerDB decode."** That is the gap.
Read the snapshot's `db/ledger/<slot>/{meta,state,tables}` → Ade's `LedgerState` +
`LiveConsensusInputsCanonical` + the certified point, and feed the existing
`bootstrap_from_mithril_snapshot` (replacing the cardano-cli `--json-seed`).

## Probe result (2026-06-22, against the real preview snapshot) — native decode is VIABLE
V2 format (cardano-node 11.x):
- `meta` (JSON): `{"backend":"utxohd-mem","checksum":<u32>,"tablesCodecVersion":1}`
- `state` (~32MB): the **NewEpochState CBOR** (`array(2)[1, …]`) — cert-state, pools+VRF, delegations,
  rewards, future pools, retirements, stake snapshots, the 5 nonces, protocol params, the epoch. NO UTxO.
- `tables` (~601MB): a **CBOR map** `array(1)[ { TxIn(34B: 32-byte TxId + 2-byte index) → TxOut(CBOR) } ]`.

1. `state` → CertState/nonce/profile: **YES** (structural NewEpochState parse). Reuse `extract_praos_nonces`
   for nonces; `snapshot_loader` is a *reference* for the cert-state delegation/reward shape (old tarball
   → adapt to V2). **GAP: the pstate pool-params + REAL VRF** — `snapshot_loader` zeroes the VRF and no code
   decodes the NewEpochState `pstate`. Leadership-critical → the core new work.
2. `tables` → UTxO: **YES** — a plain CBOR map; reuse `decode_babbage_tx_out` for the values; iterate →
   `UTxOState`. The `meta` checksum bounds integrity.
3. Safe reuse: `decode_babbage_tx_out`, `extract_praos_nonces`, `encode/decode_cert_state`, `PoolParams`,
   the whole Mithril routing+binding+composition.
4. Absent / version-dependent: pstate pool-params + REAL VRF (the bulk); the V2 layout (`utxohd-mem`,
   `tablesCodecVersion 1`) is a bounded adapter vs the old tarball; the full Conway NewEpochState decode;
   confirm `decode_babbage_tx_out` matches the tables `TxOut` exactly (first-impl check).
5. Binding: **YES** — the dir slot == the manifest `certified_point.slot`; `verify_mithril_binding` already
   binds it; snapshot commitment = `blake2b(state ++ tables ++ meta)`.

**Boundary:** no compatibility adapter is needed for the bulk (CBOR, versioned, Ade has the codecs). The
new work is a production Conway NewEpochState `state` parser (with REAL VRF) + a `tables` CBOR-map reader.
A narrow adapter would be a fallback only for individual version-brittle fields if they surface.

## Proof obligation
Every emitted artifact (UTxO seed, CertState, pool distribution, nonce/profile, source point) is
derived from AND bound to one snapshot point + one snapshot-bytes commitment, and to the Mithril
`certified_point` via `verify_mithril_binding`.

## Classification (tiers)
- **True:** bootstrap inputs derive from ONE verified, point-consistent authority (the snapshot).
- **Derived:** Mithril verification + LedgerDB interpretation match Cardano semantics.
- **Release:** native decode + bootstrap-to-sync evidence are required before claiming Mithril bootstrap.
- **Operational:** the CLI export (`f09cc0ec`) remains useful for diagnostics / differential checks, never
  the required judge path.

## Acceptance (MITHRIL-VERIFIED-ANCHOR-IMPORT)
1. Select/download a Mithril Cardano DB snapshot. 2. Verify its certificate chain + digest. 3. Read the
certified immutable point. 4. Native-decode the LedgerState + consensus inputs from that ONE snapshot.
5. Populate Ade's UTxO seed + CertState + profile bindings from that single source. 6. `bootstrap_from_mithril_snapshot`
→ persist Ade's snapshot/WAL. 7. Warm restart with NO cardano-node / cli dependency. 8. ChainSync forward
and prove the selected tip converges.

## Stage plan
1. V2 layout + `meta` + the snapshot commitment + checksum verify (bounded; fail-closed on schema).
2. NewEpochState `state` decode → CertState (pools + **REAL VRF**, future, retiring, delegations, rewards);
   reject zeroed/missing VRF. Reuse `extract_praos_nonces`; adapt the `snapshot_loader` cert-state shape.
3. Pool distribution + nonce/profile from `state`.
4. `tables` CBOR-map reader → `UTxOState` (reuse `decode_babbage_tx_out`).
5. Point + snapshot commitment binding; feed `bootstrap_from_mithril_snapshot`.
6. Live: real Mithril snapshot → bootstrap → warm restart (no node/cli) → ChainSync convergence.

## Status
**STAGE 1 DONE (2026-06-23) — all 5 gates green; plan steps 1–3 (the `state` decode).** The native
NewEpochState decoder `crates/ade_ledger/src/ledgerdb_state.rs` `probe_ledgerdb_state` (DC-MITHRIL-01)
decodes the Conway NewEpochState → canonical `CertState` (pools with REAL VRF, future, retiring,
delegations, rewards) + pool distribution + Praos nonces — deterministic, fail-closed, NON-EMITTING (a
structured probe report only; no LedgerState/UTxO/admission). Gates:
- local Preview corpus — 704 pools all real-VRF, counts match the cardano-cli producer run;
- determinism (same bytes + epoch → byte-identical canonical CertState + commitment);
- canonical encode/decode round-trip self-check (in-probe);
- 7 hermetic fail-closed — zero-VRF, wrong-era (no fallback to latest), PoolDistr-mismatch,
  epoch-mismatch, malformed (`crates/ade_ledger/tests/ledgerdb_state_hermetic.rs`);
- **verified Mithril snapshot** (preprod ancillary via `--include-ancillary`, epoch 296, 528 pools all
  real-VRF, same verdict, deterministic — the NES epoch 296 == the certificate beacon, NOT the filename
  slot 126400064) (`crates/ade_runtime/tests/ledgerdb_state_mithril.rs`).
Gate: `ci/ci_check_ledgerdb_state_decode.sh`.

**STAGE 2 DONE (2026-06-23) — the `tables` MemPack TxOut decoder (plan step 4).**
`crates/ade_ledger/src/ledgerdb_tables.rs`: the compact (non-CBOR) TxOut values decode natively —
`MemPackReader` (explicit-endian primitives) + CompactAddr / Addr28Extra (PO#1) + faithful-u64
`TxOutValue` + datum/script (original bytes preserved) + the 6-tag `read_txout` +
`decode_tables_commitment` (era-bound, canonically-sorted, deterministic). The boundary was MemPack,
NOT CBOR — `decode_babbage_tx_out` did not apply (a grounded probe overturned that assumption). Gates:
- 15 hermetic tests (primitives; PO#1 Addr28Extra round-trip; faithful u64 — i64::MAX / i64::MAX+1 /
  u64::MAX preserved, VarLen overflow terminal; 6-tag dispatch; deterministic + era-bound commitment);
- **300000 real preprod TxOuts decode faithfully** (all 6 tags, consume-exactly —
  `crates/ade_runtime/tests/ledgerdb_tables_decode.rs`);
- **cardano-cli oracle cross-check 10/10 MATCH** (6 tag-2 Addr28Extra base addresses + coins ==
  `cardano-cli query utxo` via the live preview node — **PO#1 CLOSED** —
  `crates/ade_runtime/tests/ledgerdb_tables_oracle.rs`);
- deterministic whole-tables commitment + Conway-era binding (PO#2, from the Stage-1 `state` era,
  never the tables file or a flag).
DC-MITHRIL-02 (faithful Word64 quantity, tier=true); the **i64 ceiling is a downstream release blocker
for full ledger validation** (Ade's i64 `MultiAsset` cannot validate UTxOs with quantities > i64::MAX
— a separate ledger-value-model slice). Gate: `ci/ci_check_ledgerdb_tables_decode.sh`.

**Stage 3 = native Mithril bootstrap WIRING (DecodedTxOut → Ade `UTxOState`, feed
`bootstrap_from_mithril_snapshot`, warm-restart, ChainSync) is next.**
