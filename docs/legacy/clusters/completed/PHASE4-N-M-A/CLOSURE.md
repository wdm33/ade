# PHASE4-N-M-A — Closure record

Sub-cluster A of the PHASE4-N-M family. Ships the Ade-native
storage architecture (oracle seed importer + BootstrapAnchor +
WAL) that sub-clusters B (admission orchestrator) and C (live
operator pass) sit on top of.

## Registry deltas applied

| Rule | Change | Notes |
|------|--------|-------|
| `CN-SEED-01` | `declared → enforced` | `ade_runtime::seed_import::import_cardano_cli_json_utxo` is SOLE pub fn. `ci/ci_check_seed_import_closure.sh`. |
| `DC-SEED-01` | `declared → enforced` | UtxoFingerprint = Blake2b-256 over canonical CBOR `map(N) [TxIn → TxOut_raw]` in BTreeMap order. Two-import byte-identity + source-order independence verified on real preprod data. |
| `CN-ANCHOR-01` | `declared → enforced` | `ade_runtime::bootstrap_anchor::mint` is SOLE pub fn returning a fully-populated BootstrapAnchor. No Default, no #[non_exhaustive]. `ci/ci_check_bootstrap_anchor_closure.sh`. |
| `DC-ANCHOR-01` | `declared → enforced` | Canonical CBOR round-trip; SCHEMA_VERSION=1; fail-fast on unknown version / wrong array length / short hash / trailing bytes. |
| `CN-WAL-01` | `declared → enforced` | `WalStore::append` is the SOLE mutation method on the trait + on `FileWalStore`. `ci/ci_check_wal_append_only.sh`. |
| `DC-WAL-01` | `declared → enforced` | Trait surface enumerated: `append` / `read_all` / `verify_chain`. Forbidden methods (`truncate` / `rewrite` / `replace` / `delete` / `clear`) absent across `wal/*.rs`. |
| `DC-WAL-02` | `declared → enforced` | `verify_chain` walks the WAL asserting `prior_fp == previous post_fp`. ChainBreak is authority-fatal. |
| `DC-WAL-03` | `declared → enforced` | `wal_replay_from_anchor_two_runs_byte_identical` proves the runtime contract `same anchor + same inputs + same WAL → byte-identical outputs`. |
| `RO-GENESIS-REPLAY-01` | unchanged (`declared`, `open_obligation = "blocked_until_genesis_replay_cluster"`) | Honest scope: this cluster does NOT claim Ade replays from genesis. `ci/ci_check_genesis_replay_open_obligation.sh` prevents drift. |
| `RO-MITHRIL-IMPORT-01` | unchanged (`declared`) | Lower-priority alternative. |

## Strengthenings (existing rules)

- `T-DET-01.strengthened_in += "PHASE4-N-M-A"` — determinism
  now extends to imported initial state + WAL fingerprint chain
  + anchor + WAL replay-equivalence.
- `CN-CONS-08.strengthened_in += "PHASE4-N-M-A"` — admit
  authority's output is now recorded in the WAL fingerprint
  chain (every forward step is auditable).

## Mechanical artifacts shipped

### New BLUE modules / files
- `ade_ledger::bootstrap_anchor::{anchor, error, mod}` — closed
  `BootstrapAnchor` + `SeedPoint` + canonical CBOR codec +
  closed `BootstrapAnchorError`.
- `ade_ledger::wal::{event, error, store_trait, replay, mod}`
  — closed `WalEntry` sum (1 variant: `AdmitBlock`) + closed
  `WalError` + `WalStore` trait (3 methods only) + pure
  `replay_from_anchor` reducer.

### New GREEN modules / files
- `ade_runtime::seed_import::{json, importer, mod}` —
  serde_json deserialization + canonical UTxOState build +
  Blake2b-256 fingerprint.
- `ade_runtime::bootstrap_anchor` — `mint` composition.
- `ade_runtime::wal::{file_wal_store, mod}` — file-backed
  `WalStore` impl with rotated `wal-NNNN.bin` files +
  CRC32C-suffixed sealed files + `fdatasync`-after-append
  durability.

### New CI gates (4)
- `ci/ci_check_seed_import_closure.sh`
- `ci/ci_check_bootstrap_anchor_closure.sh`
- `ci/ci_check_wal_append_only.sh`
- `ci/ci_check_genesis_replay_open_obligation.sh`

### New deps
- `bech32 = "0.11"` on `ade_runtime` (GREEN seed importer only;
  BLUE files do not depend on it).

### Test summary
- `ade_runtime::seed_import::*` → 13/13 pass.
- `ade_ledger::bootstrap_anchor::*` + `ade_runtime::bootstrap_anchor::*` → 13/13 pass.
- `ade_ledger::wal::*` + `ade_runtime::wal::file_wal_store::*` → 17/17 pass.
- `crates/ade_runtime/tests/wal_replay_from_anchor.rs` → 6/6 pass.
- All 4 N-M-A CI gates pass.

### Smoke against real data
- 580 MB cardano-cli `query utxo` JSON dump from the local
  docker preprod (12.9M lines). A 129-entry slice parses
  cleanly: 105 lovelace-only entries, 13 inline-datum, 10
  datum-hash, 1 reference-script (fail-fast per honest scope).

## Honest scope claims (CI-checked at A5)

Per [[feedback-shell-must-not-overstate-semantic-truth]] and
[[feedback-oracle-seed-then-ade-owns]]:

| Claim                                                  | Status                |
|--------------------------------------------------------|-----------------------|
| Ade bootstraps from an oracle seed at point P          | yes (this cluster)    |
| Ade validates independently after P                    | open — sub-cluster B  |
| Ade storage/recovery is replay-equivalent from anchor  | yes (this cluster)    |
| Ade has independently replayed genesis → P             | NO — open obligation  |
| Ade decodes Haskell utxohd-mem on-disk format          | NO — non-goal (Tier-5)|

## Carry-forward open obligations

- `RO-GENESIS-REPLAY-01` (open) — Ade independently replays
  genesis → P. Multi-month effort; not bounty-critical.
- `RO-MITHRIL-IMPORT-01` (open) — Mithril-authenticated import
  as alternative to cardano-cli JSON seed.
- `RO-LIVE-05` (still open) — sub-cluster B (admission +
  agreement verdict).
- `RO-LIVE-03` (still open) — flips when B + C also close.
- **Reference script TxOut features** — fail-fast in this
  slice. Sub-cluster B (or A's A1.1 extension) adds support
  before C operator pass.

## What's NOT in this cluster

- Admission-mode binary wiring (sub-cluster B).
- `AgreementVerdict` reducer (sub-cluster B).
- Live operator pass (sub-cluster C).
- Reference-script TxOut decode.
- Genesis → P replay.
- Mithril import.
- utxohd-mem on-disk binary decoder.

## Cluster docs

- Sketch: `docs/planning/phase4-n-m-ledger-seed-invariants.md`
- Cluster doc: `docs/clusters/completed/PHASE4-N-M-A/cluster.md`
- Slice docs: `A1.md` … `A5.md`
- Doctrine references: `[[feedback-oracle-seed-then-ade-owns]]`,
  `[[feedback-shell-must-not-overstate-semantic-truth]]`.
