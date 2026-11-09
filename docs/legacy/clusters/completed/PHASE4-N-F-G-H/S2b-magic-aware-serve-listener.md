# Invariant Slice — PHASE4-N-F-G-H S2b: Magic-aware node/producer serve listener

> **Status:** Planning Artifact (Non-Normative). Normative authority is the registry + CI.

## 2. Slice Header

### Slice Name
Magic-aware node/producer serve listener (serve listeners advertise N2N versions using the **configured** network magic, not the static mainnet `N2N_SUPPORTED` table).

### Cluster
**PHASE4-N-F-G-H** — Node-spine live serve-to-peer. *Inserted between S2 and S3 as a prerequisite shared-path correctness fix* — the S2 socket test passes only because both sides used mainnet magic; a real preprod (magic 1) or C1 (magic 42) follower would fail the N2N handshake. This directly blocks both the C1 private dry-run (S3) and the C2 preprod bounty pass.

### Status
Merged (impl `a8ca5e52`). Depends on S2 (the node serve sibling).

### Cluster Exit Criteria Addressed
- [ ] **CE-G-H-2b (magic-aware serve handshake — MECHANICAL, closeable)** — the node-spine and producer serve listeners advertise N2N versions using the configured network magic (node: `cli.network_magic`; produce: parsed-genesis `network_magic`); the static mainnet-only `N2N_SUPPORTED` is **not** used as a live serve listener's `our_supported`; the closed/enumerated version *set* is unchanged (only the `network_magic` field is parameterized).

### Slice Dependencies
- **S2** (`8c6a6a7e`): `run_node_serve_task` + the node serve sibling (its `our_supported` is currently the static `N2N_SUPPORTED` — this slice fixes that).

## 3. Implementation Instruction (AI)
Add a BLUE, **additive** magic-aware responder-table builder in `ade_network::handshake::version_table` — `n2n_supported_for_magic(magic: u32) -> Vec<(u16, VersionData)>` — that mirrors `N2N_SUPPORTED`'s **closed version set** exactly (V11..=V16, same `VersionData` shape) but with `network_magic = magic`. Keep `N2N_SUPPORTED` + `n2n()` unchanged **unless a tiny refactor is necessary to avoid duplication** (e.g. `N2N_SUPPORTED` becomes the mainnet specialization sharing the same version set) — the hard rule is **no version-set change**, no version widening, no new canonical type. Change the serve-listener config's `our_supported` from `&'static [(u16, VersionData)]` to an **owned** `Arc<[(u16, VersionData)]>` (cheap per-peer clone; the spawned session needs ownership, not `'static`) in `N2nListenerConfig` + `PerPeerSessionConfig`, threading it to `run_n2n_handshake_responder` by borrow. Re-point the two **live** serve listeners: `produce_mode` (`:261`) builds from parsed-genesis `network_magic`; `run_node_serve_task` takes a `network_magic: u32` param and builds from it (the node On-arm passes `cli.network_magic`, already required when serving). Add `ci/ci_check_serve_listener_magic_aware.sh`. No new flag; no private-only branch; no version widening; no serve-authority / serializer / dispatch change; no RO-LIVE flip. Commit carries the project attribution trailer.

## 4. Intent
Make it **mechanically correct** for `--mode node` and `--mode produce` to serve a peer on a **non-mainnet** network: the serve listener advertises the configured network magic, so a preprod (magic 1) or C1 (magic 42) follower's N2N handshake succeeds. This is a **shared-path correctness bug fix** (both serve listeners were mainnet-only), prerequisite to S3's real C1 follower and the C2 preprod bounty pass.

## 5. Scope
- **Modules / crates:**
  - `crates/ade_network/src/handshake/version_table.rs` (**BLUE — additive, the only approved BLUE touch in G-H**): add `n2n_supported_for_magic(magic)`; the closed version set unchanged.
  - `crates/ade_runtime/src/network/n2n_listener.rs` (RED): `our_supported` field type `&'static [(u16, VersionData)]` → `Arc<[(u16, VersionData)]>` in `N2nListenerConfig` + `PerPeerSessionConfig`; the per-accept clone + the `run_n2n_handshake_responder` borrow.
  - `crates/ade_node/src/produce_mode.rs` (RED): serve listener `our_supported` from parsed-genesis `network_magic`.
  - `crates/ade_node/src/node_lifecycle.rs` (RED): `run_node_serve_task` takes `network_magic`; On-arm passes `cli.network_magic`.
  - `ci/ci_check_serve_listener_magic_aware.sh` (NEW gate, RED).
- **State machines affected:** none. The version *set* + negotiation logic are unchanged; only the advertised magic is parameterized.
- **Persistence / network-visible:** the serve handshake now advertises the configured magic (a correctness fix — previously always mainnet). No new wire message; no version added/removed.
- **Out of scope:** the dialer (`build_n2n_version_table` — already magic-aware); the S3 runbook/operator harness; any serve-dispatch / authority / canonical-type change; any RO-LIVE flip.

## 6. Execution Boundary
- **BLUE (additive — the ONLY approved BLUE touch in G-H):** `ade_network::handshake::version_table::n2n_supported_for_magic` — a pure, deterministic builder over the **existing closed version set**, magic parameterized. Not a new authority; not a canonical type; no version widening; the closed set byte-equivalent to `N2N_SUPPORTED`'s. **Any other BLUE change remains forbidden in this cluster.**
- **GREEN:** none new.
- **RED:** `ade_runtime::network::n2n_listener` (config field type + plumbing); `ade_node::{produce_mode, node_lifecycle}` (build from configured magic); the new gate.

## 7. Invariants Preserved
- The **closed, enumerated** N2N version negotiation (the handshake grammar) — the version *set* (V11..=V16) + `VersionData` shape are unchanged; only `network_magic` is parameterized.
- `DC-NODE-07` (S1/S2) serve clause + `ci_check_single_serve_dispatch_authority.sh` — the serve-dispatch core + single-authority are untouched.
- `DC-CONS-17` / `DC-CONS-18` / `CN-WIRE-08` — served bytes/header/serializer unchanged.
- `CN-NODE-02` containment; `ci_check_node_run_loop_containment.sh`, `ci_check_node_path_fidelity.sh`, `ci_check_served_chain_handoff_fence.sh` — byte-unchanged.
- `produce_mode` serve semantics otherwise unchanged (only the advertised magic is corrected to its actual network).

## 8. Invariants Strengthened or Introduced
- **`DC-NODE-07` — serve-handshake-magic clause (strengthened; no new rule).** Recorded wording: *"Node-spine and producer serve listeners must advertise N2N versions using the configured network magic, not a static mainnet magic table. Version negotiation remains closed and enumerated; only the `network_magic` field is derived from configured network identity."* Binding test/gate: `node_spine_serve_loopback_*` (now magic-42) + `n2n_supported_for_magic_*` unit tests + `ci_check_serve_listener_magic_aware.sh`. Recorded in `evidence_notes` now (incl. the shared produce_mode correction); final binding + the `declared → enforced` flip at the G-H close.

> Single invariant family: "serve listeners advertise the configured network magic." S2b covers exactly that — kept cohesive under `DC-NODE-07` (a dedicated handshake-magic rule would be too granular unless this becomes a wider cross-cluster issue).

## 9. Design Summary
- **`n2n_supported_for_magic(magic)`** (BLUE): `vec![(11, n2n(magic)), (12, n2n(magic)), …, (16, n2n(magic))]` — same closed set as `N2N_SUPPORTED`, magic injected. If a tiny refactor avoids duplication, `N2N_SUPPORTED` may be re-expressed as the mainnet specialization sharing this set; the version set must be byte-identical either way.
- **Owned `our_supported`:** `Arc<[(u16, VersionData)]>` so the spawned per-peer session owns it (no `'static`); cloned per accept (cheap Arc clone); `run_n2n_handshake_responder(&mut bt, &our_supported)`.
- **Live serve sites build from configured magic:** produce_mode from parsed-genesis `network_magic`; `run_node_serve_task(network_magic, …)` from `cli.network_magic`.
- **Gate** `ci_check_serve_listener_magic_aware.sh`: no `our_supported: N2N_SUPPORTED` in `produce_mode.rs` / `node_lifecycle.rs` (the live serve sites) — they must use `n2n_supported_for_magic`. (Test/dialer uses of `N2N_SUPPORTED` for version-number enumeration are unaffected.)

## 10. Changes Introduced
### Types
- `N2nListenerConfig.our_supported` + `PerPeerSessionConfig.our_supported`: `&'static [(u16, VersionData)]` → `Arc<[(u16, VersionData)]>`. No new canonical type (the 456 count is unchanged; `Arc<[…]>` of an existing type).
### State Transitions / Persistence
- None.
### Removal / Refactors
- The two live serve listeners stop using the static `N2N_SUPPORTED`; `run_node_serve_task` gains a `network_magic` param. `N2N_SUPPORTED`/`n2n()` unchanged unless a tiny de-duplication refactor (version set byte-identical).

## 11. Replay, Crash, and Epoch Validation
- **Replay:** no authoritative state; the version set is deterministic; `n2n_supported_for_magic(m)` is a pure function of `m`. No new replay corpus.
- **Crash/epoch:** not applicable.

## 12. Mechanical Acceptance Criteria
- [ ] `n2n_supported_for_magic_produces_configured_magic` (`ade_network`): `n2n_supported_for_magic(1)` / `(42)` / `(MAINNET_NETWORK_MAGIC)` each yield a table whose every `VersionData.network_magic == m`, and whose version-number set is **exactly** `N2N_SUPPORTED`'s (closed; no widening).
- [ ] `node_spine_serve_loopback_follower_fetches_self_accepted_block` updated to use a **non-mainnet magic (42)** end-to-end (serve listener built from magic 42; follower `build_n2n_version_table(42)`) — handshake succeeds + the block is fetched. (Proves C1 magic 42 over the real socket.)
- [ ] `node_serve_start_failure_is_surfaced_not_silent` — still green.
- [ ] `ci_check_serve_listener_magic_aware.sh` — green: no `our_supported: N2N_SUPPORTED` at the live serve sites (`produce_mode.rs`, `node_lifecycle.rs`); smoke-tested fail-closed on an injected `our_supported: N2N_SUPPORTED`.
- [ ] `produce_loopback` (4/4), `ci_check_single_serve_dispatch_authority.sh`, `ci_check_node_run_loop_containment.sh`, `ci_check_node_path_fidelity.sh`, `ci_check_served_chain_handoff_fence.sh` — green / byte-unchanged.
- [ ] `cargo test -p ade_node` + `cargo test -p ade_runtime` + targeted `ade_network` server **and** handshake tests — green.

## 13. Failure Modes
- A live serve listener built with the wrong magic → a real follower refuses the handshake (deterministic refuse; surfaced as a session error). The gate + the magic-42 loopback prevent regressing to mainnet-only.
- `cli.network_magic` absent on the serve path → the node already requires `--network-magic` when serving a live feed; serving without it fails-fast (carried).

## 14. Hard Prohibitions
### Inherited Cluster-Level Prohibitions
All PHASE4-N-F-G-H prohibitions apply **except** the narrow, approved additive BLUE touch in §6 (the magic-aware version-table builder). No `--mode produce` switch; no private-only path; no relay-loop serve mutation; no second serve authority / serializer; no proactive `advance_tip`; no new `--mode node` flag; no RO-LIVE flip.
### Slice-Specific Prohibitions
- **No version widening / no version-set change** — only the `network_magic` field is parameterized; V11..=V16 set unchanged.
- **No new authority / canonical type / variant** — `n2n_supported_for_magic` is a pure builder over the existing set; it is the ONLY approved BLUE touch; any other BLUE change is forbidden.
- **No private-only magic branch** — the magic comes from the standard configured network identity (same source the dialer uses).
- **No serve-dispatch / authority / serializer change** — `DC-NODE-07` core + CN-WIRE-08 untouched.

## 15. Explicit Non-Goals
This slice MUST NOT: write the S3 runbook / operator harness; change the serve-dispatch core or any canonical serve/forge surface; add a `--mode node` flag; widen/alter the N2N version set; introduce any BLUE change beyond `n2n_supported_for_magic`; claim peer acceptance / BA-02 / RO-LIVE.

## 16. Completion Checklist
- [ ] `n2n_supported_for_magic` added (closed set, magic param); version set byte-identical to `N2N_SUPPORTED`'s.
- [ ] Live serve listeners (node + produce) build from configured magic; gate green + fail-closed smoke.
- [ ] Magic-42 socket loopback green; `node_serve_start_failure_*` green; `produce_loopback` green.
- [ ] Containment / path-fidelity / handoff / single-serve-dispatch gates green / byte-unchanged.
- [ ] `cargo test -p {ade_node, ade_runtime}` + targeted `ade_network` green.
- [ ] `DC-NODE-07` `evidence_notes` record S2b (incl. the shared produce_mode correction); binding + flip at the G-H close.

## 17. Review Notes
- **Approved BLUE exception:** `n2n_supported_for_magic` in BLUE `version_table.rs` is the correct single-source location (the version table is part of the closed handshake grammar). Reconstructing it in RED would duplicate version authority and risk drift — worse. The touch is additive, pure, deterministic, closed-set-preserving, not a new canonical type, no version widening, parameterizing only `network_magic`. **S2b is the ONLY approved BLUE touch in G-H; any other BLUE change remains forbidden.**
- **Registry:** strengthen `DC-NODE-07` (kept cohesive); no new rule.
- **Why this is shared-path progress, not a workaround:** the C1 dry-run found a real mismatch (mainnet-only serve handshake) before we spent a preprod cycle on it — the fix corrects both serve listeners for *every* configured network. S3 then runs the real Haskell follower harness against a correctly-magic'd serve.
