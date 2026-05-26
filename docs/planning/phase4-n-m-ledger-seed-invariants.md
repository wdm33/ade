# PHASE4-N-M family — Oracle-seed bootstrap + Ade-native WAL + live admission — invariants sketch

> This sketch supersedes the v0 sketch
> (`phase4-n-m-ledger-seed-invariants.v0-utxohd-decode.md.bak`,
> archived 2026-05-26) which incorrectly proposed
> reverse-engineering cardano-node's utxohd-mem on-disk format.
> Per
> [[feedback-oracle-seed-then-ade-owns]] the live-admission
> bootstrap uses a cardano-cli JSON UTxO oracle seed; Ade owns
> all runtime storage post-import.

## Framing

PHASE4-N-L-LIVE closed the wire-only half of RO-LIVE-03
(RO-LIVE-04). The remaining half — RO-LIVE-05 (live admission +
per-block agreement_verdict against a real peer) — is what this
family closes.

The mechanical work decomposes into three sub-clusters, ordered
by dependency:

- **PHASE4-N-M-A — Oracle seed + BootstrapAnchor + WAL.** Ship
  the Ade-native storage architecture: import the cardano-cli
  JSON UTxO dump as an oracle bootstrap input, canonicalize it
  into Ade `UTxOState`, mint a typed `BootstrapAnchor` recording
  the import provenance, ship the Ade-native WAL (`WalEntry`
  closed sum) that records every forward step from the anchor,
  and prove the replay-equivalence property `anchor + WAL →
  byte-identical final ledger fingerprint`. CI-only mechanical
  evidence; no live pass.
- **PHASE4-N-M-B — `AgreementVerdict` + admission orchestrator
  wiring.** Closed `AgreementVerdict` sum + ade_node admission
  mode + per-block verdict derivation + admission JSONL
  vocabulary. Hermetic mechanical evidence + smoke against the
  local docker preprod.
- **PHASE4-N-M-C — Operator live pass.** 30-minute admission run
  against the local docker; capture JSONL with
  `agreement_verdict { Agreed }` per admitted block; flip
  RO-LIVE-05 + RO-LIVE-03 to enforced.

This document scopes **all three**. Implementation proceeds
sub-cluster by sub-cluster.

## 0. Doctrine — oracle seed vs runtime authority

Per [[feedback-oracle-seed-then-ade-owns]]:

The cardano-cli JSON UTxO dump is a **bootstrap input artifact
at a named point P** — not the runtime authority. After import,
Ade owns:

  `LedgerState`, `PraosChainDepState`, `ChainDb tip`,
  `WalEntry`s, snapshots, checkpoints, rollback materialization,
  future block/tx validity decisions.

Cardano-node is no longer consulted for "what the ledger is" —
only for "what blocks have arrived" (chain-sync / block-fetch).
The compatibility requirement is *same chain point P + same
inputs → same future outcomes*, NOT *same on-disk layout*.

This intentionally diverges from cardano-node's utxohd-mem
internal storage (custom `tablesCodecVersion: 1` binary, which
we deliberately do NOT decode). Storage layout is Tier-5
deliberate divergence; chain facts at P are Tier-1 must-match.

**Hard-encoded honest scope claims** (one row per claim;
CI-checked at A5):

| Claim                                                  | Status      |
|--------------------------------------------------------|-------------|
| Ade bootstraps from an oracle seed at point P          | yes (N-M-A) |
| Ade validates independently after P                    | yes (N-M-B) |
| Ade storage/recovery is replay-equivalent from anchor  | yes (N-M-A) |
| Ade has independently replayed genesis → P             | NO — open   |
| Ade decodes Haskell utxohd-mem on-disk format          | NO — non-goal |

The "NO — open" row becomes a registry open-obligation
(`RO-GENESIS-REPLAY-01`). The "NO — non-goal" row is recorded as
a Tier-5 deliberate divergence per CE-79.

## 1. What must always be true

### Sub-cluster A — Seed + Anchor + WAL

- **I-A1 Single seed-import authority.** Exactly one `pub fn` in
  `ade_runtime::seed_import::import_cardano_cli_json_utxo`
  converts a cardano-cli JSON UTxO dump into Ade
  `(UTxOState, UtxoFingerprint)`. No second importer; no
  per-call fallback path.
- **I-A2 Canonical UTxO encoding.** The imported `UTxOState`
  uses `BTreeMap<TxIn, TxOut>` iteration order; the
  `UtxoFingerprint` is `Blake2b-256` over canonical-CBOR-encoded
  `(TxIn, TxOut)` pairs in BTreeMap iteration order. Re-importing
  the same JSON yields the same fingerprint byte-identically.
- **I-A3 BootstrapAnchor is a closed record + sole anchor
  source.** Exactly one `pub fn` in
  `ade_runtime::bootstrap_anchor::mint` produces a
  `BootstrapAnchor` with all 6 required fields populated:
  `network_magic`, `genesis_hash`, `seed_point: {slot,
  block_hash}`, `seed_artifact_hash`, `imported_utxo_fingerprint`,
  `initial_ledger_fingerprint`. The struct is `non_exhaustive`-free
  (closed); any missing field is a compile error.
- **I-A4 WAL is append-only + closed-sum.** `WalEntry` is a
  closed sum (one variant in this sub-cluster:
  `AdmitBlock { prior_fp, block_hash, slot, verdict, post_fp }`;
  future entries — `RollBackward`, `CaptureSnapshot` — are
  additive). `WalStore::append` is the SOLE mutation method; no
  `truncate` / `rewrite` / `replace` exists.
- **I-A5 WAL fingerprint chain is verifiable.** Every
  `WalEntry::AdmitBlock` has a `prior_fp` that equals the
  previous entry's `post_fp` (or the anchor's
  `initial_ledger_fingerprint` for the first entry). A
  `WalStore::verify_chain()` method walks the WAL + asserts the
  chain holds. Verify failure is authority-fatal.
- **I-A6 Replay-equivalence is mechanically proven.** Replaying
  `(BootstrapAnchor + WAL entries 1..N)` against the
  `(initial_ledger from import + per-entry block bytes)`
  produces a final ledger whose fingerprint equals
  `WAL[N].post_fp` byte-identically across two runs.

### Sub-cluster B — AgreementVerdict + admission

- **I-B1 Closed `AgreementVerdict` sum.** Four variants:
  - `Agreed { our_hash, peer_hash }` — admit produced a block
    whose hash equals the peer's announced tip hash at the same
    slot.
  - `Lagging { our_slot, peer_slot }` — we're behind but not
    diverged.
  - `Diverged { our_hash, peer_hash, slot }` — admit succeeded
    but hash differs from peer's. Authority-fatal.
  - `InputNotFound { tx_in }` — UTxO miss (typically stale seed).
- **I-B2 Diverged is authority-fatal.** Mirrors PHASE4-N-K
  DC-NODE-04. Distinct exit code `EXIT_LIVE_AGREEMENT_DIVERGED
  = 30`.
- **I-B3 Per-admit emission.** Exactly one `agreement_verdict`
  JSONL event per admit-attempt. Never twice; never zero.
- **I-B4 Closed admission JSONL vocabulary.** Separate from
  wire-only's `LiveLogEvent`; new `AdmissionLogEvent` closed
  enum. CI grep keeps wire-only-mode files free of
  admission-mode literals and vice versa.

### Sub-cluster C — Live pass
- **I-C1 Operator pass capture is the deliverable.** Cluster does
  NOT close until a captured JSONL under
  `docs/clusters/completed/PHASE4-N-M-LEDGER-SEED/` shows ≥ 90%
  `Agreed` over ≥ 30 min against local docker preprod. Any
  `Diverged` halts — we debug instead of closing.

## 2. What must never be possible

### Sub-cluster A
- **¬P-A1** A second JSON-seed importer (CI grep).
- **¬P-A2** A `UtxoFingerprint` computed without canonical
  `BTreeMap` iteration.
- **¬P-A3** A `BootstrapAnchor` with any default-filled field.
- **¬P-A4** A WAL `rewrite` / `truncate` / `replace` method.
- **¬P-A5** A WAL entry whose `prior_fp` ≠ previous `post_fp`.
- **¬P-A6** Decoding `tablesCodecVersion: 1` binary — explicit
  non-goal.

### Sub-cluster B
- **¬P-B1** `AgreementVerdict { Agreed }` when hashes differ.
- **¬P-B2** `agreement_verdict` without a preceding
  `admit_via_block_validity`.
- **¬P-B3** Silently retrying after `Diverged`.

### Sub-cluster C
- **¬P-C1** Committing a JSONL with any `Diverged` verdict
  — the cluster doesn't close on a chain-disagreement log.

## 3. What must remain identical across executions

- Sub-A: `(json_seed_bytes) → (UTxOState_canonical_bytes,
  UtxoFingerprint)` is deterministic.
- Sub-A: `(anchor, wal[0..N], block_bytes[0..N]) →
  final_ledger_fingerprint` is deterministic across two
  replays.
- Sub-B: `(admit_outcome, peer_announced_tip) → AgreementVerdict`
  is a pure reducer.

## 4. What must be replay-equivalent

- Sub-A headline test:
  `wal_replay_from_anchor_two_runs_byte_identical`.

## 5. State transitions in scope

```text
# Sub-cluster A

seed_import::import_cardano_cli_json_utxo(json_path)
  -> Result<(UTxOState, UtxoFingerprint), JsonSeedError>

bootstrap_anchor::mint(import_inputs)
  -> BootstrapAnchor

wal::WalStore::append(entry: WalEntry)
  -> Result<(), WalError>

wal::WalStore::verify_chain(anchor: &BootstrapAnchor)
  -> Result<(), WalError>

wal::replay_from_anchor(anchor, wal, block_bytes_for_each_entry)
  -> Result<LedgerState, WalReplayError>


# Sub-cluster B

admission::verdict::derive(admit_outcome, peer_tip)
  -> AgreementVerdict
  // pure reducer, closed -> closed

admission::run_admission(cli) -> ExitCode
  // 1. Import JSON seed if --json-seed PATH supplied.
  // 2. Mint BootstrapAnchor.
  // 3. Persist seed snapshot via PersistentSnapshotCache.
  // 4. Call bootstrap_initial_state (warm-start).
  // 5. Spawn N2nDialer for each --peer.
  // 6. Per AdmittedBlock effect:
  //    - WalStore::append(WalEntry::AdmitBlock { ... })
  //    - derive AgreementVerdict
  //    - emit AdmissionLogEvent::AgreementVerdict
  //    - Diverged → halt fatal.


# Sub-cluster C — operator-action only; no Rust transitions.
```

## 6. TCB color hypothesis

### Sub-cluster A
- **BLUE (new):**
  - `ade_codec::shelley::utxo_seed_json` — pure JSON parser
    for the cardano-cli output shape.
  - `ade_ledger::bootstrap_anchor::{anchor, error}` —
    `BootstrapAnchor` struct + canonical CBOR serializer + closed
    `BootstrapAnchorError`.
  - `ade_ledger::wal::{event, store_trait, replay, error}` —
    closed `WalEntry` sum + `WalStore` trait + pure replay
    reducer + closed errors.
- **GREEN (new):**
  - `ade_runtime::seed_import` — adapter wrapping the BLUE JSON
    parser + canonical `UTxOState` build.
  - `ade_runtime::bootstrap_anchor` — `mint` composition.
  - `ade_runtime::wal::FileWalStore` — persistent file-based
    `WalStore` impl with checksums.
- **RED:** none.

### Sub-cluster B
- **GREEN (new):**
  - `ade_node::admission::verdict` — pure verdict reducer.
  - `ade_node::admission_log::{event, writer}` — closed
    `AdmissionLogEvent`.
- **RED (new):**
  - `ade_node::admission` — tokio admission-mode runner.
  - `ade_node::main` — extended `--mode` dispatch.

### Sub-cluster C
- No new code. Operator action + captured log.

## 7. Decisions on framing questions

| # | Question | Decision |
|---|----------|----------|
| 1 | Seed source | `cardano-cli query utxo --whole-utxo --out-file utxo.json` via local docker's N2C socket. Documented in cluster procedure. |
| 2 | JSON parser | `serde_json` (already in `ade_runtime`). Pure parsing only. |
| 3 | Per-TxOut bytes | cardano-cli JSON includes TxOut as a structured object (address bech32, value with multi-asset, datum, ref_script). Re-encode via existing `ade_codec::shelley::tx_components::encode_tx_out`. Compose existing canonical encoder over JSON-sourced fields. |
| 4 | UtxoFingerprint algorithm | Blake2b-256 over canonical CBOR `map(N) [TxIn → TxOut]` in BTreeMap order. Matches `ade_ledger::fingerprint` discipline. |
| 5 | BootstrapAnchor serialization | Canonical CBOR via existing `ade_codec::cbor`. Versioned `SCHEMA_VERSION = 1`. |
| 6 | WAL storage backend | One file per cluster of entries; rotation at 10K entries or 100MB; each file has CRC32C suffix. |
| 7 | WAL append durability | `fdatasync` after every `append`; no batching (Tier-5 future). |
| 8 | Operator pass scope | 30 min against local docker preprod. Operator uses whole-utxo dump from same docker (oracle = peer). |
| 9 | Mithril import | Out of scope; carried as `RO-MITHRIL-IMPORT-01` open obligation. |
| 10 | Genesis replay | Out of scope; carried as `RO-GENESIS-REPLAY-01` open obligation. |

## 8. Registry deltas (planned at /cluster-plan)

### New families (declared at append; flipped per slice)

- `CN-SEED-01` — single JSON seed-importer authority.
- `DC-SEED-01` — canonical UtxoFingerprint determinism.
- `CN-ANCHOR-01` — single BootstrapAnchor mint authority.
- `DC-ANCHOR-01` — BootstrapAnchor closed record + round-trip.
- `CN-WAL-01` — single WalStore::append authority.
- `DC-WAL-01` — WAL is append-only by type (no rewrite method).
- `DC-WAL-02` — WAL fingerprint-chain integrity verifiable.
- `DC-WAL-03` — anchor + WAL replay-equivalence
  (`anchor + WAL → byte-identical final fingerprint`).
- `CN-ADMIT-01` — single admission-mode entry.
- `DC-ADMIT-01` — closed `AgreementVerdict` sum.
- `DC-ADMIT-02` — verdict emitted exactly once per admit.
- `DC-ADMIT-03` — Diverged is authority-fatal.
- `DC-ADMIT-04` — closed AdmissionLogEvent vocabulary.

### New honest-scope obligations
- `RO-GENESIS-REPLAY-01` — Ade independently replays
  genesis → P. `open_obligation = "blocked_until_genesis_replay_cluster"`.
- `RO-MITHRIL-IMPORT-01` — Authenticated Mithril snapshot import
  alternative. Lower priority.

### Strengthenings (at cluster close)
- `T-DET-01`, `CN-CONS-08`, `CN-STORE-07`, `CN-STORE-08`,
  `DC-CONS-21`, `DC-STORE-08` all `strengthened_in +=
  "PHASE4-N-M-LEDGER-SEED"`.

## 9. Slice shape (sub-cluster A)

| Slice | Scope | TCB | Effort |
|-------|-------|-----|--------|
| A1 | BLUE `ade_codec::shelley::utxo_seed_json` + GREEN `ade_runtime::seed_import`. CN-SEED-01, DC-SEED-01. | BLUE + GREEN | medium |
| A2 | BLUE `ade_ledger::bootstrap_anchor::{anchor, error}` + GREEN `ade_runtime::bootstrap_anchor::mint`. CN-ANCHOR-01, DC-ANCHOR-01. | BLUE + GREEN | small |
| A3 | BLUE `ade_ledger::wal::{event, store_trait, replay, error}` + GREEN `ade_runtime::wal::FileWalStore`. CN-WAL-01, DC-WAL-01, DC-WAL-02. | BLUE + GREEN | medium |
| A4 | Integration test: `wal_replay_from_anchor_two_runs_byte_identical`. CI gates for single-authority. DC-WAL-03. | test + CI | small |
| A5 | Honest-scope CI gate (RO-GENESIS-REPLAY-01 open-obligation asserted-as-still-open). Cluster close. | CI | small |

Dependencies: A2 ↔ A1 (UtxoFingerprint); A3 ↔ A2 (anchor for
verify_chain); A4 ↔ A1-A3; A5 ↔ A4.

## 10. Honest-scope carry-forward

- **Sub-cluster B** is the next cluster after A.
- **Sub-cluster C** is the cluster after B.
- **`RO-GENESIS-REPLAY-01`** — open obligation. Resolves via a
  separate byron→shelley→…→conway era replay cluster
  (multi-month, not bounty-critical).
- **`RO-MITHRIL-IMPORT-01`** — lower-priority alternative.
- **utxohd-mem on-disk format** — explicit non-goal per
  [[feedback-oracle-seed-then-ade-owns]].

## 11. Why this is the right shape

The user's framing (memory:
[[feedback-oracle-seed-then-ade-owns]]) makes the architecture
explicit: cardano-node is the oracle at P; Ade owns the runtime
representation after import. The cluster ships the Ade-native
storage layer (UTxO + Anchor + WAL) the bounty actually needs,
without reverse-engineering cardano-node's internal storage.
This:

- Closes RO-LIVE-05 honestly (live admission from a real seed
  via Ade-native storage).
- Doesn't pretend to claim "Ade replays from genesis" (recorded
  as open obligation, not faked).
- Doesn't pull on the utxohd binary thread (explicit non-goal).
- Sets up Tier-5 divergence on storage layout that "improves in
  our own image" — canonical BTreeMap UTxO, WAL with fingerprint
  chain, snapshot-as-compression-of-replay.

Cost: sub-cluster A is ~1-2 weeks honest scope; B and C roughly
mirror PHASE4-N-L + PHASE4-N-L-LIVE (~3-5 days each).
