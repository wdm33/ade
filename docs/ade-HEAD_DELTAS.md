# HEAD Deltas — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Regenerate via `/head-deltas <baseline>`. Baseline is declared in
> `.idd-config.json` (`head_deltas_baseline`).

> Baseline: `e509886` (PHASE4-N-I grounding-doc refresh, 2026-05-26 01:53 +0700)
> HEAD: `f15102f` (feat(rollback): PersistentSnapshotCache + close DC-CONS-21 (PHASE4-N-J S8), 2026-05-26 09:31 +0700)
> 8 commits, 25 files changed, +4,823 / −7 lines

> **Baseline shift note.** This regen narrows the baseline from the
> prior `d509f02` (Phase 3 handoff snapshot) to `e509886` (PHASE4-N-I
> close — the most recent grounding-doc refresh). HEAD_DELTAS now
> narrates **only** the PHASE4-N-J cluster: persistent ledger snapshot
> encoder + restart-safe rollback closure. The prior cluster-by-cluster
> narrative (Phase 4 N-A through N-I) is preserved in the archived
> cluster docs under `docs/clusters/completed/` and in the SEAMS /
> CODEMAP / TRACEABILITY companions.

> **Cluster summary.** PHASE4-N-J ships the canonical on-disk encoder
> for `(LedgerState, PraosChainDepState)` snapshots over 8 slices
> (S1 → S8), closing the `DC-CONS-21` `open_obligation` left by
> PHASE4-N-I and introducing 3 new constitutional rules
> (`DC-STORE-08`, `DC-STORE-09`, `CN-STORE-08`) all flipped to
> `enforced` at S7. The new `ade_ledger::snapshot` BLUE module (8 new
> files, ~3,265 LOC of production + test code) plus the new
> `ade_runtime::rollback::persistent_cache` GREEN bridge (1 new file,
> ~251 LOC) compose the full persistent SnapshotReader stack;
> cross-impl equivalence with the N-I in-memory cache is proven
> mechanically. One new CI script
> (`ci/ci_check_snapshot_encoder_closure.sh`) was added: total
> `ci_check_*.sh` count moves from 54 (at N-I close) to 55 (at N-J
> close).

---

## 1. Commit Log

Verbatim from `git log --oneline --no-merges e509886..f15102f`, ordered newest-first.

| Hash | Type | Summary |
|------|------|---------|
| `f15102f` | feat | feat(rollback): PersistentSnapshotCache + close DC-CONS-21 (PHASE4-N-J S8) |
| `ff1f2f9` | feat | feat(snapshot): combined framing + CN-STORE-08 CI gate, flip DC-STORE-08/09/CN-STORE-08 to enforced (PHASE4-N-J S7) |
| `7535730` | feat | feat(snapshot): LedgerState assemble encoder/decoder (PHASE4-N-J S6) |
| `fb199fe` | feat | feat(snapshot): ConwayGovState + ProtocolParameters + ConwayOnlyDepositParams encoder/decoder (PHASE4-N-J S5) |
| `f2cbdb0` | feat | feat(ledger): snapshot::epoch_state encoder/decoder (PHASE4-N-J S4) |
| `406bcc4` | feat | feat(ledger): snapshot::cert_state encoder/decoder (PHASE4-N-J S3) |
| `fd5eb30` | feat | feat(ledger): snapshot::utxo_state encoder/decoder (PHASE4-N-J S2) |
| `ab4165a` | feat | feat(ledger): snapshot::chain_dep encoder/decoder + error sums (PHASE4-N-J S1) |

All 8 commits are conventional-commits `feat:` prefixed and belong to
cluster PHASE4-N-J. No merges, no fix / docs / chore / refactor /
test commits in this window — the cluster is a single linear feature
stream that builds the snapshot encoder bottom-up (`chain_dep` →
sub-state encoders → assembled `LedgerState` → framing → persistent
cache).

---

## 2. New Modules

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_ledger::snapshot` | **BLUE** | Canonical, deterministic on-disk encoder/decoder for `(LedgerState, PraosChainDepState)` snapshots. Single-authority encode/decode pair (CN-STORE-08); embedded fingerprint cross-check (DC-STORE-08) + closed `u32` version tag (DC-STORE-09); definite-length CBOR; BTreeMap iteration only; no HashMap / wall-clock / float / rand. | `mod.rs` (re-exports), `error.rs` (`SnapshotEncodeError`, `SnapshotDecodeError`, `StructuralReason` sum types), `chain_dep.rs` (PraosChainDepState encode/decode), `utxo_state.rs` (UTxO + fees encode/decode), `cert_state.rs` (DState / PState / VState encode/decode), `epoch_state.rs` (EpochState encode/decode), `gov_state.rs` (ConwayGovState + ProtocolParameters + ConwayOnlyDepositParams encode/decode), `ledger.rs` (assembled LedgerState encode/decode), `framing.rs` (combined snapshot bytes with `SCHEMA_VERSION` + embedded fingerprint cross-check, `encode_snapshot` / `decode_snapshot` SOLE pair) | PHASE4-N-J / S1 – S7 |
| `ade_runtime::rollback::persistent_cache` | **GREEN** | Restart-safe `SnapshotReader` impl bridging the BLUE `ade_ledger::snapshot::framing` encoder/decoder to an on-disk `SnapshotStore`. Implements the same trait surface as the N-I `InMemorySnapshotCache`; cross-impl equivalence proven via `persistent_cache_matches_in_memory_cache_semantics`. Pure projection — no async, no wall-clock, no rand; deterministic over the on-disk store. | `persistent_cache.rs` (`PersistentSnapshotCache`, `PersistentCacheError`, `PERSISTENT_CACHE_SCHEMA_VERSION`) | PHASE4-N-J / S8 |

No new workspace crates. Workspace members at baseline and HEAD are
identical (11 members). The two new modules are added as `pub mod`
sub-trees of existing crates (`ade_ledger` and `ade_runtime`).

Cross-reference: both new modules should be reflected in CODEMAP §BLUE
(`ade_ledger::snapshot`) and §GREEN
(`ade_runtime::rollback::persistent_cache`). If absent, CODEMAP is
stale — regenerate via `/codemap`.

---

## 3. Modules Modified

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_ledger` (lib.rs only — sub-tree additions counted under §2) | +1 line | `pub mod snapshot;` declaration wiring the new §2 BLUE module into the crate. No other edits to existing `ade_ledger` source. |
| `ade_runtime::rollback` (mod.rs only — sub-tree additions counted under §2) | +4 lines | `pub mod persistent_cache;` declaration plus `pub use` re-exports for `PersistentSnapshotCache`, `PersistentCacheError`, `PERSISTENT_CACHE_SCHEMA_VERSION`. No other edits to existing `ade_runtime::rollback` source. |
| `docs/ade-invariant-registry.toml` | −7 / +83 lines | `DC-CONS-21` flipped `declared` → `enforced`; `open_obligation` removed; `strengthened_in = ["PHASE4-N-J"]`; `code_locus` / `tests` / `ci_script` / `cross_ref` populated. 3 new `[[rules]]` entries appended at HEAD: `DC-STORE-08` (encoder canonicality), `DC-STORE-09` (version-tag + fingerprint cross-check), `CN-STORE-08` (single encoder authority) — all `enforced` at S7 (`ff1f2f9`). |

No other source modules were touched. The cluster is **purely
additive** to the existing module graph — N-J introduces a new BLUE
subtree and a new GREEN bridge, and wires both via single-line `pub
mod` declarations. No refactors, no API breakage, no removals.

**Latent bug caught + fixed during the cluster.** S5 (`fb199fe`)
discovered that the runtime-sized container headers emitted by the
S1 – S4 sub-state encoders used `IntWidth::Inline`, which would
overflow / mis-encode for containers above the inline encoding
threshold. S5 ships the fix: `canonical_width` is now used for all
runtime-sized container headers across `chain_dep`, `utxo_state`,
`cert_state`, `epoch_state`, and `gov_state`. The fix is mechanical
and confined to the new module — no existing module behavior changes.

---

## 4. Feature Flags

No Cargo `[features]` table is declared in `ade_ledger`,
`ade_runtime`, or any other workspace crate at baseline or at HEAD.
No new feature flags introduced; no existing feature flags modified
or removed.

The cluster adds two new closed constants that are **not** Cargo
features but are referenced here for completeness:

| Constant | Module | Purpose | Status |
|----------|--------|---------|--------|
| `SCHEMA_VERSION: u32 = 1` | `ade_ledger::snapshot::framing` | Closed snapshot wire-version tag. Unknown versions are rejected before payload decoding (DC-STORE-09). Single authority — CI gate `ci_check_snapshot_encoder_closure.sh` enforces no parallel `pub const SCHEMA_VERSION` outside `framing.rs`. | **New** since baseline |
| `PERSISTENT_CACHE_SCHEMA_VERSION` | `ade_runtime::rollback::persistent_cache` | Closed cache-layer version tag for the on-disk persistent SnapshotStore layout. Independent of the BLUE `SCHEMA_VERSION`; gates the GREEN cache bridge format. | **New** since baseline |

No coupling between the two: the BLUE `SCHEMA_VERSION` gates the
encoded snapshot payload; the GREEN `PERSISTENT_CACHE_SCHEMA_VERSION`
gates the surrounding cache-store layout. Both are closed `u32`
constants with explicit `Unknown*Version` decode-error variants.

---

## 5. CI Checks

### PHASE4-N-J snapshot encoder closure (`ff1f2f9`, S7) — 1 new script (the 55th)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_snapshot_encoder_closure.sh` | **New** (`ff1f2f9`, S7) — the **55th** script | Enforces `CN-STORE-08` + `DC-STORE-08` + `DC-STORE-09` via 5 mechanical guards: (1) the canonical sites `crates/ade_ledger/src/snapshot/{framing,ledger,chain_dep}.rs` exist; (2) `pub fn encode_snapshot` / `decode_snapshot` appear **only** in `framing.rs` (single-authority for the combined snapshot pair — CN-STORE-08); (3) `pub fn encode_ledger_state` / `decode_ledger_state` appear **only** in `ledger.rs`; (4) `pub fn encode_chain_dep` / `decode_chain_dep` appear **only** in `chain_dep.rs`; (5) `pub const SCHEMA_VERSION` appears **only** in `framing.rs` (DC-STORE-09); (6) `framing.rs` references both `FingerprintMismatch` and `UnknownVersion` `SnapshotDecodeError` variants — proves the cross-check + version-gate paths exist at the framing layer (DC-STORE-08 + DC-STORE-09). |

Total CI script count: **54 → 55** (`ci/ci_check_*.sh`).
No removals, no modifications to existing scripts in the
`e509886..f15102f` window — the cluster strictly appends one script.

TRACEABILITY cross-reference: the new script appears as a
`ci_script` for `DC-STORE-08`, `DC-STORE-09`, `CN-STORE-08`, and
`DC-CONS-21` in `docs/ade-invariant-registry.toml` (4 new
`ci_script ↔ rule` edges). Re-traced via
`ci/ci_check_constitution_coverage.sh` — passes at HEAD.

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is null.
Canonical-type rules live inline in the invariant registry under
family `T`.

**PHASE4-N-J introduced new closed sum types** in support of the
persistent snapshot encoder (BLUE + GREEN combined):

- `ade_ledger::snapshot::error::SnapshotEncodeError` — closed
  encode-side error sum.
- `ade_ledger::snapshot::error::SnapshotDecodeError` — closed
  decode-side error sum (includes the load-bearing `UnknownVersion`
  and `FingerprintMismatch` variants gating DC-STORE-09 / DC-STORE-08).
- `ade_ledger::snapshot::error::StructuralReason` — closed sum
  enumerating per-sub-state structural-decode failure reasons.
- `ade_runtime::rollback::persistent_cache::PersistentCacheError` —
  closed GREEN cache-bridge error sum.

Plus the canonical encode/decode pairs that are now the SOLE
authority sites (CN-STORE-08):

- `encode_snapshot` / `decode_snapshot` (framing.rs)
- `encode_ledger_state` / `decode_ledger_state` (ledger.rs)
- `encode_chain_dep` / `decode_chain_dep` (chain_dep.rs)

**Removals: 0** (expected under append-only discipline).

Exact whole-project type recount belongs to the TRACEABILITY regen
that follows this HEAD_DELTAS.

---

## 7. Normative Rule Delta

The project's invariant registry tracks structured rules (TOML), not
prose normative-doc rules; this section reports on it.

- Rules at baseline (`e509886:docs/ade-invariant-registry.toml`): **206**
- Rules at HEAD (`f15102f:docs/ade-invariant-registry.toml`): **209**
- Net additions: **+3**
  - `DC-STORE-08` (derived) — snapshot encoder canonicality:
    `encode_snapshot(s)` is byte-identical across runs; BTreeMap-only
    iteration; no HashMap / wall-clock / float / rand; definite-length
    CBOR. Introduced and flipped to `enforced` at S7 (`ff1f2f9`).
  - `DC-STORE-09` (derived) — closed `u32` version tag + embedded
    blake2b-256 fingerprint cross-check; decoder reads version first
    and rejects unknown versions before payload decoding; decoder
    recomputes fingerprint on decoded state and rejects on mismatch.
    Introduced and flipped to `enforced` at S7 (`ff1f2f9`).
  - `CN-STORE-08` (release) — single encoder authority: the
    `encode_*` / `decode_*` pairs for `LedgerState`,
    `PraosChainDepState`, and combined snapshot bytes are the SOLE
    `pub fn` pairs in the project encoding or decoding those types
    to/from bytes; type-level + CI grep enforcement mirroring
    `CN-STORE-07`. Introduced and flipped to `enforced` at S7
    (`ff1f2f9`).
- Removals: **0** (expected under append-only discipline; clean).

- **Status flips on carried-forward rules (1):**
  - **`DC-CONS-21` `declared` → `enforced`** at S8 (`f15102f`).
    The snapshot encode/decode round-trip rule was carried forward
    from PHASE4-N-I `declared` with
    `open_obligation = "persistent_ledger_snapshot_encoding_follow_on_cluster"`.
    PHASE4-N-J ships the persistent encoder
    (`ade_ledger::snapshot::framing::{encode_snapshot,decode_snapshot}`)
    + the GREEN bridge (`PersistentSnapshotCache`) that closes the
    obligation atomically. `open_obligation` removed;
    `strengthened_in = ["PHASE4-N-J"]`; `cross_ref` extended with
    the new triple `["DC-STORE-08", "DC-STORE-09", "CN-STORE-08"]`;
    `code_locus` + `tests` + `ci_script` populated.

- **Strengthenings recorded by PHASE4-N-J:**
  - **`DC-CONS-21.strengthened_in += "PHASE4-N-J"`** — see above.
  - No other rules strengthened in this window.

- **Open obligations status at HEAD:**
  - **`DC-CONS-21.open_obligation` REMOVED** by PHASE4-N-J S8 — the
    persistent encoder + bridge are now mechanically enforced via
    `ci_check_snapshot_encoder_closure.sh` + the
    `snapshot::framing::tests::snapshot_round_trip` +
    `rollback::persistent_cache::tests::persistent_cache_matches_in_memory_cache_semantics`
    test pair.
  - **`RO-LIVE-02.open_obligation = "blocked_until_operator_peer_available"`**
    — carried forward from PHASE4-N-H. Unchanged.
  - **`RO-LIVE-01.open_obligation = "blocked_until_operator_peer_available"`**
    — carried forward from PHASE4-N-G. Unchanged.
  - **`CN-CONS-06.open_obligation = "blocked_until_operator_stake_available"`**
    — carried forward from PHASE4-N-C. Unchanged.
  - **`OP-OPS-04.open_obligation`** (Sum6KES skey loader) — carried
    forward; unchanged.

---

## Anomalies and Cross-Reference Warnings

- **No canonical-type or invariant-rule removals.** Append-only
  discipline preserved across the cluster.
- **No conventional-commits violations.** All 8 commits carry the
  `feat:` prefix with `(scope)` and `(PHASE4-N-J SN)` suffix.
- **CODEMAP cross-reference**: the two new modules (§2) must appear
  in CODEMAP. If absent at the next read, CODEMAP is stale — regen
  via `/codemap`.
- **TRACEABILITY cross-reference**: the one new CI script (§5) and
  the three new rules + one flipped rule (§7) must appear in
  TRACEABILITY. If absent at the next read, regen via `/traceability`.
- **Latent encoder bug fixed mid-cluster** (S5 found, S5 fixed): the
  S1 – S4 sub-state encoders initially used `IntWidth::Inline` for
  runtime-sized container headers; the S5 cluster path switched them
  to `canonical_width`. This was caught **before** any encoder
  output landed in a persisted artifact — no downstream
  contamination, no replay corruption. Surfaced here because it is a
  mid-cluster correction worth recording for the audit trail.

---

## Generation Notes

This regen was produced by `/head-deltas e509886` against HEAD
`f15102f`. The baseline was shifted from the prior Phase 3 handoff
(`d509f02`) to the most recent grounding-doc refresh (`e509886`,
PHASE4-N-I close) per the cluster-close cadence. Future regens
should continue to baseline at the **previous** grounding-doc
refresh, not the original Phase 3 handoff, so the document remains
narrow and reviewable per-cluster rather than accumulating
multi-cluster narrative.

Mechanical inputs:
- `git log --oneline --no-merges e509886..f15102f` → §1.
- `git diff --name-status e509886..f15102f` → §2 + §3.
- `git diff --stat e509886..f15102f -- crates/<crate>/` → §3 scope
  column.
- Workspace `Cargo.toml` diff (no membership change) → §2
  (no new crates).
- `git ls-tree e509886 ci/` vs `git ls-tree f15102f ci/` → §5.
- `git diff e509886..f15102f -- docs/ade-invariant-registry.toml`
  + entry count (`grep -c '^\[\[rules\]\]'`) → §7.
