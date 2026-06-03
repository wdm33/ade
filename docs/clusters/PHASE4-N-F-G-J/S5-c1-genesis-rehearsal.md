# Invariant Slice — PHASE4-N-F-G-J S5: C1 operator-gated genesis-successor rehearsal

> **Status:** Planning artifact (non-normative). Normative authority is the registry + CI.

## §2 Slice Header

- **Slice:** PHASE4-N-F-G-J S5 — C1 operator-gated genesis-successor rehearsal (the mechanical harness closes; the live C1 run stays operator-gated).
- **Cluster:** PHASE4-N-F-G-J — Genesis-successor block correctness (`c167cd41`).
- **Status:** Proposed (doc-before-implement).
- **Cluster Exit Criteria addressed — CE-G-J-5 (verbatim):** "a C1 rerun harness + runbook: a real Haskell follower **is expected to** validate/fetch the Ade-forged genesis-successor block **if the block is protocol-valid**; the only acceptance claim comes from the follower log through `correlate → PrivateRehearsalManifest`. No RO-LIVE flip. `blocked_until_operator_c1_genesis_successor_rehearsal` (the mechanical harness closes; live execution stays gated)." *(CE-G-J-1..4 already met.)*

## §3 Slice Dependencies

- **S4** (`DC-NODE-08`, enforced — `3df8bd4f`) — **hard dependency**: the rehearsal exercises the cold-start path S4 made reachable (both tips `None` → block 0 + `Genesis`).
- **S3** (`CN-WIRE-09`, enforced) — the rehearsed block is the null-prev genesis-successor S3 made position-legal + self-acceptable.
- **G-D rehearsal harness** (`CN-REHEARSAL-FIDELITY-01`, enforced) — reuses `ade_node::ba02_evidence::correlate`, `ade_node::rehearsal_evidence::PrivateRehearsalManifest` (+ `from_correlate_outcome` / `to_canonical_toml`), `ade_node::rehearsal_pass`, `ci/ci_check_rehearsal_manifest_schema.sh`, `ci/ci_check_node_path_fidelity.sh`. No new evidence type.

## §4 Intent (invariant impact)

Extend the **path-faithful, non-promotable** rehearsal discipline (`CN-REHEARSAL-FIDELITY-01`) to the **cold-start genesis-successor** path: the headline G-J question — *will a real Haskell follower validate/fetch an Ade-forged block 0 whose `prev_hash` is CBOR `null`?* — is exercised end-to-end through the **identical** `--mode node` cold-start path (no private-only flag, no from-genesis constructor), and any acceptance claim is produced **only** by a real operator-captured follower log through `correlate → PrivateRehearsalManifest` (`is_rehearsal = true`, `not_bounty_evidence = true`), never an Ade-internal `self_accept`/`forge_succeeded`/served-bytes signal, and **never** a RO-LIVE flip. The mechanical harness + runbook are CI-enforced; the live C1 execution stays `blocked_until_operator_c1_genesis_successor_rehearsal`.

## §5 Scope / What is built

1. **Genesis-successor rehearsal runbook** — `docs/evidence/phase4-n-f-g-j-genesis-rehearsal-README.md`, a strict adaptation of the G-D runbook (`docs/evidence/phase4-n-f-g-d-private-rehearsal-README.md`): **same** `--mode node` path + flags + `correlate` + `NoEvidence` fail-closed; the **only** delta is the **cold-start scenario** — the operator pre-seeds the C1 store with the recovered seed-epoch lineage but **no tip** (a recovered-lineage WarmStart with no persisted block tip), so the node forges the genesis-successor (block 0 + `Genesis`) per S4/S3, and a follower validates/fetches the **null-prev** block.
2. **Hermetic genesis-rehearsal mechanics test** (GREEN/test) — forges a genesis-successor block (block 0 + `Genesis`, via the S3/S4 forge path), takes its hash, drives `correlate` over a **synthetic** peer-accept of that hash → `Ba02Manifest` → `PrivateRehearsalManifest`, and asserts the wrapped block decodes to block 0 + `Genesis`. **Mechanics only** — never written under the rehearsal home, never an acceptance claim (the synthetic event is reducer-fodder, dropped if not allow-listed).
3. **Env-gated live operator harness** — `node_c1_genesis_rehearsal_live` (a RED test, **skipped in CI**, run only by the operator with `ADE_LIVE_*` env + a real captured C1 follower log). Writes a `PrivateRehearsalManifest` **only** on a real `correlate`-produced match; `NoEvidence` panics and writes nothing.
4. **CI gates (reused):** `ci/ci_check_rehearsal_manifest_schema.sh` (closed schema + `is_rehearsal`/`not_bounty_evidence` markers + peer-log sha256 binding + no rehearsal marker under the bounty home) and `ci/ci_check_node_path_fidelity.sh` (the cold-start rehearsal uses the identical `--mode node` path — no private-only flag, no from-genesis constructor).

**Out of scope:** any change to `correlate` / `PrivateRehearsalManifest` / `forge_one_from_recovered` / the BLUE forge (all reused verbatim).

## §6 Execution Boundary (TCB color)

- **GREEN** — `ade_node::rehearsal_evidence` (`PrivateRehearsalManifest`, deterministic `to_canonical_toml`) + `ade_node::ba02_evidence::correlate` (the closed allow-list reducer): pure, deterministic, reused unchanged; the hermetic mechanics test exercises them.
- **RED** — the env-gated `node_c1_genesis_rehearsal_live` operator harness (peer-log file I/O, env-gated; never a runtime node mode); the runbook is operator documentation (release/evidence scope, not runtime authority).
- **BLUE (reused, unchanged)** — the S3/S4 cold-start forge + `self_accept`; S5 adds **no** BLUE code.
- **No ambiguous colors.** The rehearsal evidence path is GREEN+RED only; no authoritative-core change.

## §7 Invariants Preserved

- **`RO-LIVE-01` / `RO-LIVE-06`** — **no flip**; both stay partial/operator-gated. C1 acceptance ≠ bounty/preprod completion.
- **`CN-REHEARSAL-FIDELITY-01`** — non-promotability (correlate-only, `is_rehearsal`/`not_bounty_evidence` literals, rehearsal-home only) + path fidelity hold for the genesis variant.
- **`CN-OPERATOR-EVIDENCE-01` / `DC-EVIDENCE-01`** — acceptance comes only from the allow-listed follower-log events (`peer_served_block` / `peer_chain_tip`) through `correlate`; every Ade-internal signal is dropped ([[feedback-shell-must-not-overstate-semantic-truth]]).
- **`CN-WIRE-09` (S3) / `DC-NODE-08` (S4)** — the rehearsed block is the position-legal null-prev genesis-successor reached via the cold-start path; unchanged.
- **`DC-FORGE-01` / `DC-NODE-06` / `DC-NODE-07`** — forge determinism + self-accept handoff + single serve; reused.
- **`CN-NODE-01`** — no second bootstrap; the cold-start C1 store is operator-pre-seeded for WarmStart (FirstRun stays Mithril-only).

## §8 Invariants Strengthened

**`CN-REHEARSAL-FIDELITY-01`** — `strengthened_in += "PHASE4-N-F-G-J"`. The path-faithful, non-promotable rehearsal discipline now **covers the cold-start genesis-successor path**: the genesis rehearsal uses the identical `--mode node` cold-start path (fenced by `ci_check_node_path_fidelity.sh`) and produces only a correlate-derived, non-promotable `PrivateRehearsalManifest` (fenced by `ci_check_rehearsal_manifest_schema.sh`). Append the S5 hermetic mechanics tests to the rule; record the genesis-rehearsal `open_obligation = blocked_until_operator_c1_genesis_successor_rehearsal`. **No new rule** (no new semantics); **no RO-LIVE flip**.

## §9 Open questions resolved in this slice

- **The live acceptance stays operator-gated** — the mechanical harness (runbook + correlate wiring + manifest schema + path-fidelity fence) closes in CI; the live C1 genesis rehearsal + any follower-accept claim are `blocked_until_operator_c1_genesis_successor_rehearsal`. No RO-LIVE flip; C1 ≠ preprod/bounty.
- **Genesis binding** — the rehearsal is bound to the genesis-successor by (a) the cold-start runbook setup (lineage-seeded, no tip) and (b) the hermetic test asserting the correlated block decodes to block 0 + `Genesis`; the manifest itself remains the generic `Ba02Manifest` payload (no new evidence type).

## §11 Replay / Crash / Epoch Validation

- **Genesis-rehearsal mechanics (new, hermetic):** `genesis_rehearsal_manifest_binds_block_zero_genesis` — forge a genesis-successor block; `correlate` a synthetic peer-accept of its hash → `PrivateRehearsalManifest`; assert the wrapped block decodes to block 0 + `PrevHash::Genesis`, `matched_block_hash_hex` == the forged hash, and `to_canonical_toml` emits `is_rehearsal = true` + `not_bounty_evidence = true`. Mechanics only — no rehearsal-home write.
- **Fail-closed non-promotability (new, hermetic):** `genesis_rehearsal_no_evidence_writes_nothing` — `PrivateRehearsalManifest::from_correlate_outcome(NoEvidence, _) == None` (no synthetic manifest path for the genesis rehearsal).
- **Live arm (env-gated, skipped in CI):** `node_c1_genesis_rehearsal_live` — operator-run; asserts a real follower-log `correlate` match writes the manifest, `NoEvidence` writes nothing.
- **Crash/epoch:** none new — evidence-tier; single-epoch containment (`DC-EPOCH-03`) inherited from the forge path.

## §12 Mechanical Acceptance Criteria

Complete only when all pass in CI:

- [ ] `genesis_rehearsal_manifest_binds_block_zero_genesis` (hermetic: forge block 0 + `Genesis` → `correlate` → `PrivateRehearsalManifest` bound to that block; `is_rehearsal`/`not_bounty_evidence` literals).
- [ ] `genesis_rehearsal_no_evidence_writes_nothing` (hermetic: `NoEvidence` ⇒ `None`).
- [ ] `node_c1_genesis_rehearsal_live` **compiles** and is **env-gated/skipped** without `ADE_LIVE_*` (operator-only; never runs in CI).
- [ ] `bash ci/ci_check_rehearsal_manifest_schema.sh` green (vacuous until a manifest is committed; verifies the closed schema + rehearsal markers + peer-log sha256 + no rehearsal marker under the bounty home — covers the new `phase4-n-f-g-j-genesis-rehearsal-*` home).
- [ ] `bash ci/ci_check_node_path_fidelity.sh` green (the genesis rehearsal uses the identical `--mode node` cold-start path; no private-only flag, no from-genesis constructor).
- [ ] `cargo test -p ade_node` green (unmasked exit code). *(Full `cargo test --workspace` unmasked is the cluster-close gate, `RO-CLOSE-01`.)*

## §13 Failure Modes

- **`NoEvidence` / non-allow-listed follower log** — the live arm panics and writes nothing; an Ade-internal signal can never be coerced into acceptance (allow-list, `CN-OPERATOR-EVIDENCE-01`). Deterministic.
- **A committed manifest under the wrong home / with mismatched peer-log sha256** — `ci_check_rehearsal_manifest_schema.sh` fails closed.
- **A rehearsal that tried a private-only flag / from-genesis constructor** — rejected by the binary (unknown flag) or unrepresentable (no such constructor); `ci_check_node_path_fidelity.sh` fails closed.

## §14 Hard Prohibitions

Inherits cluster §11 in full. Slice-specific:
- **No `RO-LIVE-01`/`RO-LIVE-06` flip**; C1 genesis acceptance ≠ preprod/bounty completion.
- **No synthetic manifest** under the rehearsal home — a manifest is always `correlate`-produced (`NoEvidence` writes nothing); hermetic correlate fixtures are mechanics-only and never committed as evidence.
- **No acceptance claim from any Ade-internal signal** (`self_accept` / `forge_succeeded` / served bytes / wire success) — only the allow-listed follower-log events.
- **No new consensus rule, no new evidence type, no BLUE change**; reuse `correlate` / `PrivateRehearsalManifest` verbatim.
- **No mainnet/preprod claim, no Mithril expansion, no durable block-1+ progression, no broad live sync, no private-only flag/constructor** (path fidelity).

## §15 Explicit Non-Goals

The bounty deliverable / preprod (C2) pass; flipping any RO-LIVE rule; Mithril behavior; durable progression to block 1+; broad/unbounded live sync; any new consensus or evidence semantics; running the live C1 rehearsal in CI (it is operator-gated, `blocked_until_operator_c1_genesis_successor_rehearsal`).
