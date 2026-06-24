# SLICE: MITHRIL-VERIFIED-ANCHOR-INTEGRATION / S1b — authority transition (assemble + persist)

## Claim
Assemble the COMPLETE authoritative seed — `LedgerState` + `PraosChainDepState` + a NATIVE
`LiveConsensusInputsCanonical` — from ONLY the verified manifest + the S1a `NativeSnapshotNonUtxoState`
(Stage 1) + the Stage 2 `tables` UTxO + genesis constants, with **no cardano-cli / JSON consensus-input
bundle / operator seed / convenience fallback**, enforce POINT COHERENCE, feed it into
`bootstrap_from_mithril_snapshot` (the CLI/JSON seed REMOVED from this path), and persist the durable
bootstrap artifacts ATOMICALLY **before** the authority becomes visible.

## Input → Output
- Input: the verified manifest (certified point + network magic + epoch), the S1a
  `NativeSnapshotNonUtxoState`, the Stage-2 `tables` UTxO (`UTxOState`, via `decode_tables`/`from_map`),
  genesis constants only (`max_lovelace_supply`, active-slots coeff, …).
- Output: the persisted durable bootstrap state (slot snapshot + seed-epoch sidecar + recovered-anchor
  point + the WAL commit-point), point-coherent + atomic; `MithrilBootstrapOutput { ledger, chain_dep,
  tip, anchor }`.

## Assembly (sources are EXCLUSIVELY manifest / S1a / Stage-2 / genesis — no operator bundle)
- `LedgerState`: `utxo_state` ← Stage-2; `cert_state` ← S1a; `epoch_state{ epoch ← S1a, slot ← manifest
  point, reserves+treasury ← S1a, snapshots = cold-start empty, block_production ← S1a, epoch_fees = 0 }`;
  `protocol_params` ← S1a (incl. the `MinUtxoRule::PerByte`); `era = Conway`; `max_lovelace_supply` ←
  genesis constant; `gov_state = None`; `conway_deposit_params = None`; `track_utxo = false`.
- `PraosChainDepState`: the five nonces ← S1a; `op_cert_counters = empty`; `last_* = None` (cold start).
- NATIVE `LiveConsensusInputsCanonical`: `network_magic` ← manifest; `genesis_hash` ← manifest;
  `epoch_no` ← S1a; `epoch_start/end_slot` ← derived (era schedule + S1a epoch); `active_slots_coeff` ←
  genesis; `epoch_nonce` ← S1a eta0; `pool_distribution` ← S1a `pool_distr` (stake); `pool_vrf_keyhashes`
  ← S1a `pool_distr` (VRF); `protocol_params_hash` ← S1a params; `source_tip` ← manifest point. NO
  cardano-cli provenance / operator bundle — the snapshot IS the source.

## Point coherence (TERMINAL before any persistence/visibility)
S1a point == manifest certified point == the persisted recovered-anchor point; S1a epoch == manifest
epoch (already in S1a); S1a `network_id` == manifest-magic-derived id (already in S1a); the assembled
anchor's seed_point == the manifest point. ANY mismatch / missing input → a structured terminal error,
NO authority assembled, NO partial persisted.

## Persist BEFORE visibility (atomic; no bootable partial state)
Write the durable artifacts via `SnapshotStore` + the WAL commit-point (redb MVCC + the WAL append is
the commit): the slot snapshot, the seed-epoch consensus sidecar, and the recovered-anchor point (call
`put_recovered_anchor_point` — currently uncalled). The WAL append is the SOLE point at which the anchor
lineage becomes discoverable; an interrupted import (any write failing before the WAL commit) leaves NO
bootable partial authority state. The authority is visible ONLY after the atomic commit.

## Hard fence
- NO `--json-seed-path`, NO `import_live_consensus_inputs` operator bundle, NO `import_cardano_cli_json_utxo`,
  NO convenience fallback ON THIS PATH. The CLI/JSON seed is REMOVED from the native bootstrap path (the
  current `bootstrap_from_mithril_snapshot` seed inputs are replaced by the native assembly, or a native
  entry supersedes them). cardano-cli / JSON-seed stay RED diagnostic/oracle tooling, never bootstrap
  authority.
- Do NOT implement the Conway per-byte min-UTxO calculator (DC-LEDGER-PARAMS-01 release blocker; native
  Conway FOLLOW is gated on it — S2's concern, not S1b's). S1b assembles + persists; it does not follow.

## Acceptance
- same verified snapshot bytes + manifest → byte-identical assembled seed + canonical commitments;
- interrupted import NEVER leaves a bootable partial authority state (a kill before the WAL commit → no
  discoverable anchor lineage);
- a malformed / mixed / point-incoherent / wrong-era snapshot is rejected with a structured canonical error;
- the imported anchor point is explicitly evidenced (and recoverable via `load_recovered_anchor_point`);
- NO CLI / JSON consensus-input bundle / operator seed participates.

## Scope fence
Assembly + atomic persistence ONLY. NOT the cold restart / warm-start recovery / ChainSync (that is S2,
the release gate). The value model (DC-LEDGER-VALUE-01) and the pparams representation (DC-LEDGER-PARAMS-01
representation half) are frozen for this slice.
