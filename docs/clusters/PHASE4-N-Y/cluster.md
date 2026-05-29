# PHASE4-N-Y — Mithril-Anchored Bootstrap, Network Forward-Sync & WAL Recovery (cluster doc)

> **Status:** Planning. 1 cluster / 5 slices. **Primary bounty cluster.** Makes Ade
> reach the live tip from a verified Mithril snapshot (or controlled Conway genesis),
> forward-sync through real validation, persist preserved bytes + Ade-canonical
> WAL/checkpoints, and recover deterministically after power loss.
>
> **Predecessor:** PHASE4-N-X (HEAD `c83f2ba`).
> **Inputs:** [`../../planning/mithril-anchored-bootstrap-forward-sync-recovery-cluster-spec.md`](../../planning/mithril-anchored-bootstrap-forward-sync-recovery-cluster-spec.md)
> + [`../../planning/mithril-anchored-bootstrap-forward-sync-recovery-cluster-slice-plan.md`](../../planning/mithril-anchored-bootstrap-forward-sync-recovery-cluster-slice-plan.md).
> **Governance:** registry promotion is **per-slice and minimal**. The
> `bootstrap_initial_state` seam, the two-driver split, and the `WalEntry` shape are
> **mechanical acceptance criteria, never registry law.**

## §1 Primary invariant

> Ade reaches the live tip and survives power loss through the **single closed
> bootstrap authority** (`bootstrap_initial_state`) — fed by a **verified** Mithril
> snapshot or a controlled Conway genesis — preserving Cardano wire bytes for
> hash-critical paths and Ade-canonical bytes for WAL/checkpoints; recovery is
> **byte-identical** to clean execution ([[T-DET-01]], [[DC-STORE-01]]), and Cardano
> compatibility is proven **only on observable surfaces**, never by comparing Ade's
> ledger fingerprint to Haskell's private serialization.

Load-bearing guarantees:

1. **No storage before a verified anchor ([[CN-ANCHOR-01]], strengthened).** Both
   Mithril and genesis sources produce a `BootstrapAnchor` whose binding is verified
   before storage initializes; a mismatch fails closed.
2. **One closed bootstrap authority — no plugin seam.** Both sources populate
   `BootstrapInputs.genesis_initial` / a `SnapshotStore`; **no `GenesisAnchor`/
   `MithrilAnchor` trait** is introduced (`BootstrapAnchor` stays a struct).
3. **Two-layer byte authority (carried, [[DC-STORE-02]]).** Hash-critical paths use
   preserved wire bytes (`StoredBlock.bytes`, `PreservedCbor`); WAL/checkpoints/evidence
   use Ade-canonical bytes. Re-encoding a hash-critical path stays CI-forbidden
   (`ci_check_hash_uses_wire_bytes.sh`).
4. **Recovery = snapshot + forward replay ([[DC-STORE-05]], strengthened), not full
   genesis replay.** Restart loads the latest valid checkpoint + replays forward from
   preserved bytes + WAL.
5. **Compatibility is observable-only.** Internal `fingerprint` equality is valid only
   *genesis-path == snapshot-path* for the same chain state; Haskell agreement is
   verdicts + selected tip hash + block hashes + `query utxo` + transcript.

**Non-claims (honest scope).** This cluster does **not** discharge full N2N/N2C
mini-protocol coverage (sibling cluster — the bounty's N2C blockfetch/txsubmission +
two-Haskell-node leg stays blocked until then), does **not** implement mainnet
Byron→Conway historical genesis replay ([[RO-GENESIS-REPLAY-01]], deferred), and does
**not** itself perform the operator-pass forge leg (layers on after S1–S3).

## §2 Normative anchors

- Determinism / byte authority: [[T-DET-01]], `docs/active/comparison_surface_contract.md`,
  CE-73 reclassification (`docs/active/CE-73_reclassification.md`) — bytes Tier-4
  non-goal, semantics Tier-2; layout may diverge, chain facts must not.
- Storage / recovery: [[DC-STORE-01]], [[DC-STORE-02]], [[DC-STORE-03]], [[DC-STORE-05]],
  [[DC-WAL-01]], [[DC-WAL-02]], [[DC-WAL-03]], [[CN-WAL-01]].
- Bootstrap / seed: [[CN-ANCHOR-01]], [[DC-ANCHOR-01]], [[CN-SEED-01]], [[DC-SEED-01]],
  [[CN-NODE-01]], `docs/active/shelley_bootstrap_contract.md`.
- Admission: [[DC-CONS-20]]. Genesis parser: [[CN-GENESIS-01]].
- Release obligations: [[RO-MITHRIL-IMPORT-01]], [[RO-GENESIS-REPLAY-01]],
  [[CN-OPERATOR-EVIDENCE-01]] (operator-witnessed evidence pattern).

## §3 Entry conditions (already enforced — this cluster builds on, doesn't re-prove)

- Closed bootstrap authority `bootstrap_initial_state` with cold/warm matrix
  (test `bootstrap_cold_start_returns_genesis_when_empty`,
  `bootstrap_warm_start_equals_direct_materialize`).
- Preserved-byte `ChainDb` (in-memory + persistent redb), Ade-canonical `snapshot::*`
  encode/decode, canonical WAL + `replay_from_anchor`
  (test `replay_from_anchor_two_runs_byte_identical`), `BootstrapAnchor` types,
  `fingerprint`. Gates `ci_check_wal_append_only.sh`, `ci_check_seed_import_closure.sh`,
  `ci_check_bootstrap_anchor_closure.sh`, `ci_check_snapshot_encoder_closure.sh`,
  `ci_check_live_consensus_inputs_closure.sh` pass at entry.
- Forward-sync protocol clients (Handshake/ChainSync/BlockFetch/KeepAlive) + the
  receive/admission reducers + `ade_core_interop::follow` exist.

## §4 Slice index

| Slice | Purpose | Strengthens | Introduces (candidate IDs) |
|---|---|---|---|
| **S1 — Mithril import authority** | Verify a Mithril snapshot + bind the closed anchor field-set {magic, genesis hash, chain point, era, ledger fingerprint, immutable range, sync boundary}; fail-closed on mismatch; produce `BootstrapInputs` for the existing authority. **Slice-entry decision (OI-Y.1):** verify STM multisig in BLUE vs. trust the documented mithril-client + verify content-binding/fingerprint-recompute (per OP-x). | [[CN-ANCHOR-01]], [[DC-ANCHOR-01]], [[CN-SEED-01]] | **CN-MITHRIL-01**, **DC-MITHRIL-01**; flip [[RO-MITHRIL-IMPORT-01]] declared→partial (→enforced only at CE-Y-15) |
| **S2 — Network forward-sync durable lifecycle** | Anchor → ChainSync/BlockFetch → decode → header+ledger validate → chain-select → preserved-byte store → WAL append → checkpoint → tip. Admission only via chokepoints; preserved-bytes + WAL **before** tip advance. | [[DC-CONS-20]], [[DC-STORE-02]], [[DC-STORE-05]], [[T-DET-01]] | **DC-SYNC-01** *(only if no DC-CONS/DC-STORE rule covers the durable forward-sync admission ordering — confirmed at slice-doc; OI-Y.3)* |
| **S3 — End-to-end crash recovery wiring** | Node-binary restart reconstructs byte-identical state from {anchor + preserved bytes + WAL + latest checkpoint + replay}; crash at any phase recovers, no operator repair. | [[DC-STORE-01]], [[DC-STORE-03]], [[DC-WAL-01]], [[DC-WAL-02]], [[DC-WAL-03]], [[T-DET-01]] | — (strengthens existing recovery/replay laws; no new rule) |
| **S4 — Conway-genesis bootstrap source** | Conway genesis enters the **same** closed authority; non-Conway fails closed; genesis→initial-state is pure/deterministic; no historical replay. | [[CN-GENESIS-01]], [[CN-ANCHOR-01]], [[CN-NODE-01]] | **DC-GENESIS-SRC-01** |
| **S5 — Compatibility evidence bundle** | Snapshot-point→tip differential vs Haskell; selected tip hash + block/tx verdict + `query utxo` agreement; named fixtures + oracle versions + reproducible inputs; regression fixture per mismatch; two-Haskell-node private Conway testnet (operator-witnessed). | [[CN-OPERATOR-EVIDENCE-01]] | **DC-COMPAT-01**, **RO-SYNC-EVIDENCE-01** |

## §5 Exit criteria (CI-verifiable)

- [ ] **CE-Y-1.** Planning docs exist (spec + slice plan) and `docs/clusters/PHASE4-N-Y/{cluster,S1..S5}.md` exist.
- [ ] **CE-Y-2.** Mithril binding is a pure deterministic predicate over the closed field-set; tests `mithril_anchor_binding_is_deterministic` + `mithril_anchor_rejects_field_mismatch` (each field flipped → fail-closed) pass.
- [ ] **CE-Y-3.** Mithril-sourced state enters **only** via `bootstrap_initial_state`; grep gate `ci_check_mithril_uses_bootstrap_initial_state.sh` (positive `bootstrap_initial_state(`; negative: no second storage-init path; no `trait *Anchor`).
- [ ] **CE-Y-4.** Storage does not initialize on an unverified/mismatched anchor; test `mithril_import_fail_closed_blocks_storage_init` passes.
- [ ] **CE-Y-5.** Forward-sync admits blocks only through `decode_block`→`validate_and_apply_header`→`block_validity`→fork-choice; gate `ci_check_forward_sync_chokepoint_only.sh` (negative: no block reaches the store without passing the validators).
- [ ] **CE-Y-6.** Each admitted block is preserved-byte-stored **and** WAL-committed before tip advance; test `forward_sync_wal_and_bytes_precede_tip_advance` passes.
- [ ] **CE-Y-7.** Forward-sync replay-equivalence: test `forward_sync_replay_two_runs_byte_identical` over corpus `corpus/sync/preprod_snapshot_to_tip_*` — same anchor + same block sequence → byte-identical post-state fingerprint + WAL.
- [ ] **CE-Y-8.** Crash recovery: tests `recovery_crash_at_phase_{import,sync,admit,checkpoint}_byte_identical` — kill at each phase, restart through the node binary, recovered fingerprint == clean-run fingerprint, no operator step.
- [ ] **CE-Y-9.** `replay_from_anchor_two_runs_byte_identical` + `bootstrap_warm_start_equals_direct_materialize` still pass (carry-forward; recovery builds on them).
- [ ] **CE-Y-10.** Conway genesis enters the same authority; test `conway_genesis_bootstrap_through_single_authority`; non-Conway genesis → fail-closed (`genesis_non_conway_fail_closed`); `genesis_to_initial_state_deterministic` (two runs byte-identical).
- [ ] **CE-Y-11.** Internal cross-path determinism (private Conway net): test `genesis_path_fp_equals_snapshot_path_fp` — bootstrap a state from genesis+blocks, snapshot it, re-bootstrap from the snapshot, fingerprints equal.
- [ ] **CE-Y-12.** Differential harness `sync_differential_snapshot_to_tip` passes vs the Haskell oracle on selected tip hash + per-block verdict + `query utxo`, with the fixture pinning oracle versions (cardano-node/cardano-cli) + reproducible inputs.
- [ ] **CE-Y-13.** Gate `ci_check_no_haskell_fingerprint_equality.sh` — no test asserts Ade-ledger-fingerprint == a Haskell/cardano-node serialized state hash (observable-surface proof only).
- [ ] **CE-Y-14.** Each discovered mismatch is committed as a named regression fixture under `corpus/sync/regressions/` (schema-checked when present).
- [ ] **CE-Y-15.** Registry: `CN-MITHRIL-01`, `DC-MITHRIL-01`, `DC-GENESIS-SRC-01`, `DC-COMPAT-01`, `RO-SYNC-EVIDENCE-01` flip to `enforced` (or `partial` where operator-witnessed) with populated `tests`/`code_locus`/`ci_script`; `DC-SYNC-01` resolved (enforced or folded into a strengthened existing rule). [[RO-MITHRIL-IMPORT-01]] → `partial` (→ `enforced` only with the committed reproducible Mithril fixture + CI/release evidence). Strengthenings recorded (`strengthened_in += "PHASE4-N-Y"`): `CN-ANCHOR-01`, `DC-ANCHOR-01`, `DC-CONS-20`, `DC-STORE-01`, `DC-STORE-02`, `DC-STORE-05`, `DC-WAL-01..03`, `CN-GENESIS-01`, `CN-NODE-01`, `T-DET-01`.
- [ ] **CE-Y-16 (operator-witnessed).** Two-Haskell-node private Conway testnet interop + snapshot→tip live evidence captured per the `CN-OPERATOR-EVIDENCE-01` manifest pattern; `blocked_until_operator_pass_executed` until committed (the schema gate is vacuously satisfied until a manifest exists).
- [ ] **CE-Y-17.** `cargo test --workspace` clean; carry-forward gates pass (`ci_check_wal_append_only.sh`, `ci_check_bootstrap_anchor_closure.sh`, `ci_check_seed_import_closure.sh`, `ci_check_snapshot_encoder_closure.sh`, `ci_check_live_consensus_inputs_closure.sh`, `ci_check_hash_uses_wire_bytes.sh`, `ci_check_dependency_boundary.sh`).

> No human review may substitute for these checks. CE-Y-16 is the one operator-witnessed
> gate (live evidence cannot close in CI), mirroring `CN-OPERATOR-EVIDENCE-01`.

## §6 TCB color map (FC/IS partition)

- **BLUE (authoritative):** `ade_codec` (decode chokepoints, `PreservedCbor`);
  `ade_core::consensus::{header_validate, fork_choice, nonce}`;
  `ade_ledger::{block_validity, wal (encode + replay_from_anchor), snapshot,
  bootstrap_anchor, fingerprint}`; `ade_crypto` (digest / Mithril-binding verification
  primitives). The Mithril **binding/verification predicate** and the
  **genesis→canonical-state transform** are BLUE chokepoints.
- **GREEN:** `ade_runtime::bootstrap::bootstrap_initial_state` (reused, closed authority
  — CE locus, not law); **NEW** forward-sync lifecycle reducer + recovery reducer
  (GREEN-by-content, BLUE-style banner + deny attrs, purity-gated); `ade_testkit`
  differential harness. **Open question OI-Y.2:** the forward-sync reducer's color is
  GREEN *only if* it holds no socket/clock state — confirmed at slice-doc; if it must,
  it splits into a GREEN reducer + a RED pump (like `session` + `mux_pump`).
- **RED:** `ade_runtime` (Mithril-client fetch shell, network drivers
  `mux_pump`/`n2n_dialer`, `chaindb` redb writes, node-binary recovery/restart driver);
  `ade_core_interop` (live evidence drivers, cardano-cli invocation); `ade_node`
  (binary lifecycle, CLI).

Rules: no RED behavior in BLUE; GREEN must not affect authoritative outputs; color
resolved before any slice begins (OI-Y.2 is the one open color question).

## §7 Forbidden during this cluster (slices inherit)

- **No `GenesisAnchor`/`MithrilAnchor` trait or any plugin/extensibility seam** in the
  bootstrap authority — single closed authority only.
- **No cardano-node storage-layout mimicry** — no parsing of cardano-node's
  ImmutableDB/VolatileDB/LedgerDB / utxohd binary; recovery matches the *behavior*
  ([[DC-STORE-05]]), not the layout.
- **No re-encoding hash-critical bytes** — preserved wire bytes only on hash paths.
- **No storage init before a verified anchor.**
- **No tip advance before the block's preserved bytes + WAL entry are durable.**
- **No Ade-vs-Haskell private-serialization equality** as a compatibility proof.
- **No Byron→Conway historical replay** in this cluster (deferred to
  [[RO-GENESIS-REPLAY-01]]).
- **No promotion of implementation facts to registry law** — `bootstrap_initial_state`,
  the two-driver split, and the `WalEntry` shape stay CE.
- **No claim of full N2N/N2C coverage or the bounty's two-node leg** from this cluster
  alone.

## §8 Replay obligations

- **S1:** new corpus `corpus/sync/mithril_snapshot_fixture_*` + expected anchor binding;
  optional **additive** `WalEntry::SnapshotImport` (append-only; never mutate
  `AdmitBlock`).
- **S2 (primary):** `corpus/sync/preprod_snapshot_to_tip_*` — captured block sequence +
  expected post-state fingerprint; same-anchor + same-sequence → byte-identical
  (CE-Y-7). Optional additive WAL entries (RollForward/Rollback) — append-only.
- **S3:** crash-recovery corpus — kill/restart per phase → byte-identical (CE-Y-8).
- **S4:** `corpus/sync/conway_genesis_fixture_*` + expected initial-state fingerprint
  (CE-Y-10, CE-Y-11).
- **S5:** regression fixtures `corpus/sync/regressions/*` — one per discovered mismatch,
  with oracle version + reproducible input (CE-Y-14).

## §9 Open issues

- **OI-Y.1 — Mithril verification depth.** STM multisig verification in BLUE vs. trust
  the documented mithril-client (RED acquisition infra) + verify content-binding +
  recompute the ledger fingerprint over imported state. Resolve at S1 slice-doc; default
  per OP-x is trust-client + verify-binding (Mithril is infra, not a BLUE trust root).
- **OI-Y.2 — forward-sync reducer color.** GREEN reducer + RED pump split (like
  `session`/`mux_pump`) vs a single RED driver. Resolve before S2 begins.
- **OI-Y.3 — `DC-SYNC-01` necessity.** Confirm at S2 slice-doc whether the durable
  forward-sync admission ordering is already covered by [[DC-CONS-20]] + [[DC-STORE-02]]
  (then strengthen, no new rule) or needs the new `DC-SYNC-01`.
- **OI-Y.4 — networks.** Mainnet sync = Mithril; private-testnet = Conway genesis;
  preprod block-production venue uses Mithril-anchor. Confirm the differential-harness
  oracle network(s) for CE-Y-12 at S5 slice-doc.

---

> **Authority reminder.** This is a planning aid. All correctness rules live in the
> invariant registry + CI enforcement. On disagreement: normative documents + CI win.
