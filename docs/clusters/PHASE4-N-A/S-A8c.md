# Slice S-A8c — Version table alignment with cardano-node 11.0.1

> **Status**: Merged
> **Cluster**: [PHASE4-N-A](cluster.md)
> **Augments**: S-A3 version-table pinning

## 2. Slice Header

**Slice Name**: N2N + N2C supported version tables extended to cardano-node 11.0.1 + `perasSupport` field (BLUE)

**Cluster Exit Criteria Addressed**: none directly. Prerequisite for CE-N-A-5 (live interop) — without these extensions, handshake against a cardano-node 11.0.1 peer cannot negotiate any version above V14 on N2N or V20 on N2C, and the V16 N2N peer would offer the new `perasSupport` field which we'd silently drop.

**Defect classification**: forward-compatibility gap in S-A3 + version drift. S-A3 pinned N2N V11..V14 and N2C V15..V20 against cardano-node 10.6.2's advertised set. cardano-node 11.0.1 advertises:
- N2N: V14, V15, V16 (V11..V13 dropped). V16 adds `perasSupport: Bool` to `NodeToNodeVersionData`.
- N2C: V16..V23 (V9..V15 dropped, V21..V23 added).

**Slice Dependencies**: S-A3 (version table; will be extended).

## 4. Intent

`DC-PROTO-05` (version negotiation closure for cardano-node supported version tables) gains its cardano-node 11.0.1 extension. The S-A3 closure stays sound for 10.6.2 peers; this slice broadens the supported set to negotiate successfully with 11.0.1+ peers.

`DC-PROTO-06` (no ambient session state in BLUE transitions) is preserved — `perasSupport` is a typed field on the explicit `VersionData` input, never read from ambient state.

## 5. Scope

**Modules to amend**:
- `crates/ade_network/src/handshake/state.rs` — add `peras_support: bool` field to `VersionData`
- `crates/ade_network/src/handshake/version_table.rs` — extend N2N_SUPPORTED to V15+V16, extend N2C_SUPPORTED to V21+V22+V23, drop N2C V15
- `crates/ade_network/src/handshake/transition.rs` — tests: update VersionData constructions
- `crates/ade_network/tests/handshake_version_negotiation.rs` — integration test: extend negotiation matrix to include new versions

**Documentation sweep** (cosmetic):
- All "10.6.2" comments → "10.6.2 / 11.0.1" or "cardano-node 11.0.1 (10.6.2 forward-compatible)"
- S-A10 Docker pin reference updated

**Out of scope**: any other protocol; N2N V17+ or N2C V24+ when they ship (separate future slice); wire-level encoding of `perasSupport` field in VersionParams CBOR (that's S-A9 session-composer territory — the codec stays opaque-bytes; the state machine just carries the typed local representation).

## 6. Execution Boundary

| Module | Color |
|---|---|
| `ade_network::handshake::state` | **BLUE** |
| `ade_network::handshake::version_table` | **BLUE** |
| `tests/handshake_version_negotiation.rs` | **BLUE** |

## 7. Invariants Preserved

`T-DET-01`, `T-CORE-01..03`, `T-INGRESS-01`, `T-CI-01`, `T-BUILD-01`, `T-KEY-01`, `T-BOUND-02`, `T-ERR-01`, `T-ENC-03`, `DC-CORE-01`, `CN-WIRE-07`, `DC-PROTO-01/02/03/04/06`.

All 16+1 CI scripts continue to PASS.

## 8. Invariants Strengthened

- **`DC-PROTO-05`** — N2N V15, V16 + N2C V21, V22, V23 added to the closed supported set; N2C V15 dropped (no longer advertised by cardano-node).

Registry strengthenings: `DC-PROTO-05` `code_locus` unchanged (same files); `tests` array gains the new matrix-extension test names. `strengthened_in` already contains `PHASE4-N-A` — no duplicate. No status flips (real-capture verification remains S-A9 obligation).

## 9. Design Summary

### VersionData (state.rs)

```rust
pub struct VersionData {
    pub network_magic: u32,
    pub initiator_only_diffusion: bool,
    pub peer_sharing: PeerSharingFlag,
    pub query: bool,
    pub peras_support: bool,   // NEW. Present in V16+; default false for V11..V15.
}
```

Field default for V11..V15 entries: `peras_support: false`. For V16+ we default to `false` until Peras consensus integration lands in a future cluster (the field's wire-level effect is consensus-policy, not network-policy).

### N2N_SUPPORTED (version_table.rs)

```rust
pub const N2N_SUPPORTED: &[(u16, VersionData)] = &[
    (11, VersionData { network_magic: MAINNET_NETWORK_MAGIC, initiator_only_diffusion: false, peer_sharing: PeerSharingFlag::NoPeerSharing, query: false, peras_support: false }),
    (12, /* same */),
    (13, /* same */),
    (14, /* same */),
    (15, /* same */),  // NEW
    (16, VersionData { network_magic: MAINNET_NETWORK_MAGIC, initiator_only_diffusion: false, peer_sharing: PeerSharingFlag::NoPeerSharing, query: false, peras_support: false }),  // NEW
];
```

V11..V13 retained for backward-compat headroom (a relay running a pinned old release could still negotiate). V14..V16 are the actively supported set against cardano-node 11.0.1.

### N2C_SUPPORTED (version_table.rs)

```rust
pub const N2C_SUPPORTED: &[(u16, N2cVersionData)] = &[
    // V15 DROPPED — cardano-node 10.2+ no longer advertises it.
    (16, N2cVersionData { network_magic: MAINNET_NETWORK_MAGIC, query: false }),
    (17, /* same */),
    (18, /* same */),
    (19, /* same */),
    (20, /* same */),
    (21, /* same */),  // NEW
    (22, /* same */),  // NEW (adds LSQ SRV records support — opaque at this layer)
    (23, /* same */),  // NEW
];
```

`N2cVersionData` is unchanged — still `network_magic + query`. The new versions add LSQ query payload variants which we keep opaque per cluster TCB rule on the n2c module.

## 10. Changes Introduced

### Types (modified)
- `VersionData` gains `peras_support: bool` field

### Const tables (extended)
- `N2N_SUPPORTED`: 4 entries → 6 entries (added V15, V16)
- `N2C_SUPPORTED`: 6 entries → 8 entries (dropped V15, added V21, V22, V23)

## 11. Replay, Crash, and Epoch Validation

**Unit tests** (`handshake::transition::tests`):
- existing tests pass with `peras_support: false` added to VersionData constructions
- new tests:
  - `n2n_v15_happy_path` — propose V15, accept V15
  - `n2n_v16_happy_path_with_peras_support_field` — propose V16, accept V16; verify `peras_support` field is part of VersionData
  - `n2c_v21_happy_path`
  - `n2c_v22_happy_path`
  - `n2c_v23_happy_path`
  - `n2c_v15_no_longer_advertised` — verify V15 is not in N2C_SUPPORTED
  - `n2n_overlap_picks_v16_when_both_offer_v14_to_v16` — selection rule

**Integration test** (`tests/handshake_version_negotiation.rs`):
- `version_negotiation_across_supported_table` — matrix expanded to include all current N2N + N2C versions

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build -p ade_network --all-targets` — clean
- [ ] `cargo test -p ade_network --lib handshake::` — all PASS including new version tests
- [ ] `cargo test -p ade_network --test handshake_version_negotiation` — PASS
- [ ] `cargo clippy -p ade_network --all-targets -- -D warnings` — clean
- [ ] All 8 named CI scripts PASS
- [ ] Registry DC-PROTO-05: `tests` array gains the 6 new version tests. No status flips.
- [ ] All "10.6.2" comment references updated to "10.6.2 + 11.0.1" or "11.0.1 (10.6.2 forward-compatible)"

## 13. Failure Modes

`HandshakeError` shape unchanged. New version variants exercise the same error paths.

## 14. Hard Prohibitions

Inherited cluster + slice-specific:
- All cluster prohibitions
- `peras_support` MUST be a typed field on `VersionData`, never read from ambient state (DC-PROTO-06)
- No `#[non_exhaustive]` on `VersionData`
- No wire-level encoding of `perasSupport` at the codec layer — codec stays opaque-bytes; only state.rs and version_table.rs change

## 15. Explicit Non-Goals

- N2N V17+ or N2C V24+ when they ship
- Peras consensus integration (consensus is a future cluster)
- Wire-level CBOR encoding of `perasSupport` field (S-A9 / consensus-codec slice)
- Other protocols' version tables (LocalChainSyncVersion etc. stay at their MAX_*_VERSION = 100 sentinels)

## 16. Completion Checklist

- [ ] `VersionData.peras_support` field present
- [ ] N2N_SUPPORTED has V14, V15, V16
- [ ] N2C_SUPPORTED has V16..V23 (no V15)
- [ ] All construction sites updated
- [ ] 6 new version-negotiation tests
- [ ] Matrix integration test extended
- [ ] All 16+1 CI scripts PASS
- [ ] Comment sweep complete
- [ ] Registry DC-PROTO-05 tests array extended

## 17. Review Notes

**Invariant risk**:
- **V11..V13 retention is policy, not requirement**: cardano-node 11.0.1 drops them. We keep them in our advertised set for backward-compat headroom. If a future operator wants to drop them too, that's a one-line edit.
- **`peras_support: false` default**: matches Peras pre-rollout reality. When Peras consensus lands (separate future cluster), V16 peers will advertise `peras_support: true` and our state machine will need to handle the field's wire-level effect. For now it's just a typed pass-through.

**Assumptions challenged**:
- Considered dropping V11..V13 to fully match cardano-node 11.0.1's advertised set. Rejected — adds churn without benefit, and the test surface around them is already in place.

**Follow-up implied**: S-A9 session composer will need to encode/decode `perasSupport` in the VersionParams CBOR bytes when negotiating V16; S-A9 also lands real-capture corpus for the new versions.

## 18. Authority Reminder

Authority for invariants in `docs/ade-invariant-registry.toml`; mechanical acceptance in §12. Version-table authority: cardano-node 11.0.1 `ouroboros-network/cardano-diffusion/api/lib/Cardano/Network/NodeToNode/Version.hs` and `NodeToClient/Version.hs`.
