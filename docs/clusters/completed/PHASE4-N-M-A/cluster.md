# Cluster PHASE4-N-M-A — Oracle-seed + BootstrapAnchor + Ade-native WAL

> **Status:** Planning artifact (non-normative). Introduces
> `CN-SEED-01`, `DC-SEED-01`, `CN-ANCHOR-01`, `DC-ANCHOR-01`,
> `CN-WAL-01`, `DC-WAL-01`, `DC-WAL-02`, `DC-WAL-03` as enforced.
> Declares `RO-GENESIS-REPLAY-01`, `RO-MITHRIL-IMPORT-01` as
> open obligations carried forward. Sub-cluster A of the
> PHASE4-N-M family; B + C close RO-LIVE-05 + RO-LIVE-03.

## Primary invariant

> Ade can bootstrap its runtime ledger state from a cardano-node
> oracle seed (cardano-cli `query utxo --whole-utxo` JSON dump)
> at a named point P, mint a typed `BootstrapAnchor` recording
> the import provenance (network magic, genesis hash, seed
> point, seed artifact hash, imported UTxO fingerprint, initial
> ledger fingerprint), and record every forward-step from the
> anchor in an Ade-native append-only WAL whose fingerprint
> chain is verifiable. The runtime contract `same anchor + same
> inputs + same WAL → byte-identical outputs` is mechanically
> proven by a replay-equivalence test.

## Scope

- **BLUE (new):**
  - `ade_codec::shelley::utxo_seed_json` — pure JSON parser for
    the cardano-cli `query utxo --whole-utxo` output.
  - `ade_ledger::bootstrap_anchor::{anchor, error}` — closed
    `BootstrapAnchor` struct + canonical CBOR codec + closed
    error sum.
  - `ade_ledger::wal::{event, store_trait, replay, error}` —
    closed `WalEntry` sum + `WalStore` trait (`append` +
    `verify_chain` only; no truncate/rewrite/replace) + pure
    replay reducer + closed errors.
- **GREEN (new):**
  - `ade_runtime::seed_import` — adapter wrapping the BLUE JSON
    parser into the runtime `UTxOState` + computes
    `UtxoFingerprint`.
  - `ade_runtime::bootstrap_anchor::mint` — composes the import
    inputs into a `BootstrapAnchor`.
  - `ade_runtime::wal::FileWalStore` — persistent file-based
    `WalStore` impl with CRC32C-suffixed rotated files.
- **RED:** none.

Out-of-scope (declared; honest):
- utxohd-mem on-disk binary decoder (non-goal per
  [[feedback-oracle-seed-then-ade-owns]]).
- Genesis-to-P replay (`RO-GENESIS-REPLAY-01` open obligation).
- Mithril-authenticated import (`RO-MITHRIL-IMPORT-01` open).
- Admission orchestrator + verdict (sub-cluster B).
- Operator live pass (sub-cluster C).

## Grounding (verified at HEAD `48cb2bb`)

- **`ade_codec::shelley::tx_components::encode_tx_out`** —
  existing canonical TxOut encoder. Reused to encode JSON-sourced
  TxOut fields (address, value, datum, ref_script) into
  canonical bytes.
- **`ade_ledger::utxo::{UTxOState, UTxO}`** — existing
  canonical UTxO container (BTreeMap iteration).
- **`ade_ledger::fingerprint::fingerprint`** — Blake2b-256
  authority; reused for `UtxoFingerprint` + `BootstrapAnchor`
  field hashing.
- **`ade_codec::cbor`** — canonical CBOR primitives.
- **`ade_runtime::chaindb`** — patterns reused for
  `FileWalStore` file layout + crash safety.
- **Local docker `cardano-node-preprod`** —
  `.cardano-node-preprod/ipc/node.socket` is the
  cardano-cli `--socket-path` target.

## Slice index

| Slice | Scope | TCB |
|-------|-------|-----|
| A1 | BLUE `ade_codec::shelley::utxo_seed_json` + GREEN `ade_runtime::seed_import`. CN-SEED-01 + DC-SEED-01. | BLUE + GREEN |
| A2 | BLUE `ade_ledger::bootstrap_anchor` + GREEN `ade_runtime::bootstrap_anchor::mint`. CN-ANCHOR-01 + DC-ANCHOR-01. | BLUE + GREEN |
| A3 | BLUE `ade_ledger::wal` + GREEN `ade_runtime::wal::FileWalStore`. CN-WAL-01 + DC-WAL-01 + DC-WAL-02. | BLUE + GREEN |
| A4 | Replay-equivalence integration test + CI gates. DC-WAL-03. | test + CI |
| A5 | Honest-scope CI gate (RO-GENESIS-REPLAY-01 carries open obligation, asserted). Cluster close. | CI |

## Exit criteria (CI-verifiable)

- [ ] **CE-N-M-A-1 (CN-SEED-01)** — `ci/ci_check_seed_import_closure.sh`
  asserts a single pub `import_cardano_cli_json_utxo` in the
  workspace.
- [ ] **CE-N-M-A-2 (DC-SEED-01)** — `utxo_seed_two_imports_byte_identical`
  test.
- [ ] **CE-N-M-A-3 (CN-ANCHOR-01 + DC-ANCHOR-01)** —
  `ci/ci_check_bootstrap_anchor_closure.sh` + round-trip test.
- [ ] **CE-N-M-A-4 (CN-WAL-01 + DC-WAL-01)** —
  `ci/ci_check_wal_append_only.sh` asserts no truncate/rewrite/
  replace methods.
- [ ] **CE-N-M-A-5 (DC-WAL-02)** —
  `wal_verify_chain_catches_break` test.
- [ ] **CE-N-M-A-6 (DC-WAL-03)** —
  `wal_replay_from_anchor_two_runs_byte_identical` integration
  test.
- [ ] **CE-N-M-A-7 (RO-GENESIS-REPLAY-01 honesty)** —
  `ci/ci_check_genesis_replay_open_obligation.sh` asserts the
  rule remains `declared` with the open-obligation pointer
  intact (prevents accidental "we replayed from genesis" claim
  drift).

## TCB color map

- **BLUE:**
  - `crates/ade_codec/src/shelley/utxo_seed_json.rs`
  - `crates/ade_ledger/src/bootstrap_anchor/{anchor,error,mod}.rs`
  - `crates/ade_ledger/src/wal/{event,store_trait,replay,error,mod}.rs`
- **GREEN:**
  - `crates/ade_runtime/src/seed_import.rs`
  - `crates/ade_runtime/src/bootstrap_anchor.rs`
  - `crates/ade_runtime/src/wal/file_wal_store.rs`
  - `crates/ade_runtime/src/wal/mod.rs`
- **RED:** none.

## Forbidden during this cluster

- No `HashMap` / `HashSet` in any new BLUE file.
- No wall-clock / rand / float in any new BLUE file.
- No `truncate` / `rewrite` / `replace` / `delete` / `clear`
  method on `WalStore` (trait or impls).
- No second `pub fn import_cardano_cli_json_utxo` /
  `pub fn mint` / `pub trait WalStore` definitions.
- No decoding of `tablesCodecVersion: 1` binary (utxohd-mem
  on-disk format) — explicit non-goal per
  [[feedback-oracle-seed-then-ade-owns]].
- No claim in any cluster artifact that "Ade replays from
  genesis" — that's `RO-GENESIS-REPLAY-01`'s job, still open.

## Replay obligations

- New canonical replay surface: WAL bytes. Same anchor + same
  WAL → byte-identical replay.
- `T-DET-01.strengthened_in += "PHASE4-N-M-A"` (cluster close).
- `CN-CONS-08.strengthened_in += "PHASE4-N-M-A"` (admit path
  composed into WAL entries; cluster close).

## Open obligations carried after closure

- `RO-GENESIS-REPLAY-01` —
  `open_obligation = "blocked_until_genesis_replay_cluster"`.
- `RO-MITHRIL-IMPORT-01` —
  `open_obligation = "blocked_until_mithril_import_cluster"`.
- `RO-LIVE-05`, `RO-LIVE-03` — still open until N-M-B + N-M-C
  close.

## Authority reminder

Correctness rules live in `docs/ade-invariant-registry.toml`.
If guidance here conflicts with the registry:

> **Registry + CI enforcement win.**
