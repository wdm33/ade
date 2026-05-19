# Slice S-A3 — Version negotiation authority (N2N + N2C handshake)

> **Status**: Proposed
> **Cluster**: [PHASE4-N-A](cluster.md)

## 2. Slice Header

**Slice Name**: Handshake state machine + version selection (BLUE)

**Cluster**: PHASE4-N-A

**Cluster Exit Criteria Addressed**:
- **CE-N-A-1**: `cargo test -p ade_network --test handshake_version_negotiation` PASS across cardano-node 10.6.2 supported version tables.

CE-N-A-1 is closed in **two stages**: this slice ships the state machine + synthetic-vector tests derived from the Ouroboros handshake spec, closing the state-machine-correctness portion. Real-capture verification against `corpus/network/n2n/handshake/` captured frames is added in S-A9 when corpus collection runs. Tests added by S-A3 are reused unchanged at S-A9; only the input fixtures change source.

CEs out of scope: CE-N-A-2, CE-N-A-3, CE-N-A-4, CE-N-A-5.

**Slice Dependencies**: S-A1 (substrate + sync-only gate), S-A2 (codec — `HandshakeMessage`, `VersionTable`, `RefuseReason`, `*Version` newtypes already exist).

## 4. Intent

`DC-PROTO-05` (version negotiation is closed: enumerated N2N/N2C versions, explicit handshake, deterministic refusal on mismatch) becomes mechanically enforced for cardano-node 10.6.2. After this slice, the handshake state machine is a pure function `transition(state, version_table_pair, msg) → Result<(state', output), HandshakeError>` with replay-trace evidence on every supported version tuple.

`DC-PROTO-06` (BLUE transitions are pure functions of canonical inputs; no ambient session state) gains its first per-protocol enforcement.

## 5. Scope

**Modules** (under `crates/ade_network/src/handshake/`):
- `mod.rs` — barrel
- `state.rs` — `HandshakeState` enum (Idle, Proposed, Done, Refused); `HandshakeOutput`; `HandshakeError`
- `agency.rs` — `HandshakeAgency` (per-protocol, non-interchangeable per §7 #7)
- `transition.rs` — `n2n_transition`, `n2c_transition` (pure)
- `version_table.rs` — cardano-node 10.6.2 supported version constants for N2N + N2C
- `selection.rs` — `select_version` (deterministic intersection rule)
- `crates/ade_network/tests/handshake_version_negotiation.rs` — integration test

**State machines**: introduces the handshake state machine (N2N and N2C, distinct version tables).

**Persistence / network-visible impact**: none.

**Out of scope**: every other mini-protocol state machine; session composition; real-capture corpus; live interop.

## 6. Execution Boundary

| Module | Color |
|---|---|
| `ade_network::handshake::*` | **BLUE** |
| `crates/ade_network/tests/handshake_version_negotiation.rs` | **BLUE** (test driver runs the BLUE state machine) |

No GREEN or RED additions.

## 7. Invariants Preserved

- `T-DET-01`, `T-CORE-01`, `T-CORE-02`, `T-CORE-03`, `T-INGRESS-01`, `T-CI-01`, `T-BUILD-01`, `T-KEY-01`, `T-BOUND-02`, `T-ERR-01`, `T-ENC-03`
- `DC-CORE-01` (no async in BLUE)
- `CN-WIRE-07`, `DC-PROTO-01`, `DC-PROTO-03`, `DC-PROTO-04`

All 16+1 CI scripts must continue to PASS.

## 8. Invariants Strengthened

Exactly one invariant family:

- **`DC-PROTO-05`** — version negotiation closure for cardano-node 10.6.2 supported version tables.

Corollary gaining first per-protocol enforcement: `DC-PROTO-06` (version threaded as explicit input, never ambient).

When this slice merges, `DC-PROTO-05`'s `code_locus` gains `crates/ade_network/src/handshake/*.rs`, `tests` array gains the new tests, and `strengthened_in` appends `PHASE4-N-A`. Status stays `declared` until S-A9 supplies real-capture verification.

## 9. Design Summary

**State machine shape** (pure transition):

```rust
pub fn n2n_transition(
    state: HandshakeState,
    agency: HandshakeAgency,
    supported: &VersionTable,        // local supported table; caller-provided
    msg: HandshakeMessage,
) -> Result<(HandshakeState, HandshakeOutput), HandshakeError>;
```

`HandshakeOutput` variants: `Reply(HandshakeMessage)`, `Selected(N2NVersion, VersionData)`, `Refused(RefuseReason)`, `Done`.

**State transitions** (Idle → Proposed → Done | Refused):
- Idle + ProposeVersions(table) [Client agency] → Proposed
- Proposed + AcceptVersion(v, data) [Server agency] → Done with Selected(v, data)
- Proposed + Refuse(reason) [Server agency] → Refused
- Other combinations → `HandshakeError::IllegalTransition { state, message, agency }`

**Selection rule** (`select_version`):
1. Intersect `proposed.keys()` ∩ `supported.keys()`.
2. Empty → `Refuse(VersionMismatch { proposed_set, supported_set })`.
3. Else select `max(intersection)`.
4. Return `(selected_version, supported.get(selected_version).version_data)`.

**Version tables**: cardano-node 10.6.2 published values (N2N + N2C). Implementation verifies against `ouroboros-network` Haskell source.

**Per-protocol agency** (locked §7 #7): `HandshakeAgency` is its own type, non-interchangeable with any other.

**No ambient state** (DC-PROTO-06): supported table is an input, not a global.

## 10. Changes Introduced

### Types (new)
- `HandshakeState` (4 variants)
- `HandshakeAgency` (3 variants)
- `HandshakeOutput` (4 variants)
- `HandshakeError` (4+ variants)
- `VersionData` (per cardano-node 10.6.2 handshake-data shape; verify against spec)

`VersionTable` already exists in S-A2 codec (`BTreeMap<u16, VersionData>`); reused.

### State Transitions
- N2N handshake (Idle → Proposed → Done | Refused)
- N2C handshake (same shape, distinct version table)

## 11. Replay, Crash, and Epoch Validation

**Replay tests added**:
- `handshake::tests::n2n_happy_path_each_supported_version`
- `handshake::tests::n2c_happy_path_each_supported_version`
- `handshake::tests::version_mismatch_refused`
- `handshake::tests::illegal_message_in_idle_returns_error`
- `handshake::tests::wrong_agency_returns_error`
- `handshake::tests::overlap_picks_highest_common`
- `handshake::tests::empty_intersection_refuses_deterministically`
- `handshake::tests::version_data_passed_through_byte_identical`

**Integration test**:
- `tests::handshake_version_negotiation::version_negotiation_across_supported_table` — for each (proposed, supported) tuple in a curated synthetic matrix, verify selected version is deterministic AND encoded reply byte-stream is identical across 1000 runs.

**Crash / restart**: n/a (stateless per call).
**Epoch boundary**: n/a.

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build --workspace --all-targets` — clean
- [ ] `cargo test -p ade_network --lib handshake::` — 8 unit tests PASS
- [ ] `cargo test -p ade_network --test handshake_version_negotiation` — PASS
- [ ] `cargo clippy -p ade_network --all-targets -- -D warnings` — clean
- [ ] `bash ci/ci_check_no_async_in_blue.sh` — PASS
- [ ] `bash ci/ci_check_module_headers.sh` — PASS
- [ ] `bash ci/ci_check_no_semantic_cfg.sh` — PASS
- [ ] `bash ci/ci_check_no_signing_in_blue.sh` — PASS
- [ ] `bash ci/ci_check_hash_uses_wire_bytes.sh` — PASS
- [ ] `bash ci/ci_check_ingress_chokepoints.sh` — PASS
- [ ] `bash ci/ci_check_dependency_boundary.sh` — PASS
- [ ] `bash ci/ci_check_constitution_coverage.sh` — PASS
- [ ] Registry `DC-PROTO-05` updated: `code_locus` += handshake module paths; `tests` += new test names; `strengthened_in` appends `PHASE4-N-A`. Status stays `declared`.

## 13. Failure Modes

| Variant | Cause | Recovery |
|---|---|---|
| `HandshakeError::IllegalTransition { state, message, agency }` | Wrong (state, msg, agency) triple | Fail-fast |
| `HandshakeError::UnknownVersion { version }` | Decoded version not in supported set | Fail-fast |
| `HandshakeError::VersionMismatch { proposed, supported }` | Empty intersection (also a normal `HandshakeOutput::Refused` path) | Fail-fast as error / normal as output |
| `HandshakeError::MalformedMessage` | Codec returned a structurally-valid message that fails protocol-grammar invariants | Fail-fast |

All fail-fast. None affect replay equivalence.

## 14. Hard Prohibitions

Inherited cluster-level + slice-specific:

- All cluster prohibitions (async, plugins, pallas-network, etc.)
- Reading version state from global / static / lazy structure — must be input (DC-PROTO-06)
- Cross-protocol agency type reuse (`ChainSyncAgency` ≠ `HandshakeAgency`)
- `String` errors in `HandshakeError` (structured enum, `&'static str` only)
- Mutating the supported version table at runtime (compile-time constant)
- `#[non_exhaustive]` on any handshake enum
- `HashMap` (use `BTreeMap` for deterministic order)
- Any `unwrap`/`expect`/`panic` outside `#[cfg(test)]`

## 15. Explicit Non-Goals

- No other mini-protocol state machines
- No session composition (S-A9)
- No real-capture corpus (S-A9)
- No live cardano-node interop (S-A10)
- No performance optimization
- No feature flags
- No N2N/N2C version table changes beyond cardano-node 10.6.2 supported set

## 16. Completion Checklist

- [ ] Handshake state machine pure (no ambient state, no globals)
- [ ] Per-protocol `HandshakeAgency`
- [ ] Closed enums everywhere (no `#[non_exhaustive]`)
- [ ] Structured `HandshakeError`
- [ ] N2N + N2C covered with distinct version tables
- [ ] Synthetic test vectors enumerate cardano-node 10.6.2 supported tuples
- [ ] All 16+1 CI scripts PASS
- [ ] Registry DC-PROTO-05 strengthened (code_locus + tests + strengthened_in appended; status stays declared pending S-A9)
- [ ] No TODOs / placeholders / `unimplemented!()` in BLUE

## 17. Review Notes

**Invariant risk**:
- **CE-N-A-1 closes in two stages**. This slice closes the state-machine-correctness portion against synthetic vectors derived from the Ouroboros handshake spec. Real-capture verification lands in S-A9. Honest framing: "CE-N-A-1 closed against the published spec" until S-A9.
- **Version table values** must be cross-checked against the actual `ouroboros-network` Haskell source for cardano-node 10.6.2 during implementation.
- **`VersionData` shape** is version-gated (peer-sharing flag, query flag, hard-fork combinator support). Implementation models the gated shape per spec.

**Assumptions challenged**:
- Considered generic `transition<P>()`. Rejected per locked §7 #7.
- Considered `OnceCell` for supported table. Rejected per DC-PROTO-06.

**Follow-up implied**: S-A9 adds real-capture corpus; same tests re-run with captured fixtures.

## 18. Authority Reminder

Authority for invariants belongs to `docs/ade-invariant-registry.toml`. Authority for mechanical acceptance belongs to the named tests in §12.
