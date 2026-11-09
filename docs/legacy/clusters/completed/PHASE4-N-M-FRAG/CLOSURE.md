# PHASE4-N-M-FRAG — Closure record

**Closed:** 2026-05-27
**Closure HEAD:** (set on commit)
**Predecessor HEAD:** `222292c` (finding doc commit), built on top
of `0016722` (A1.1 follow-ups).

## Goal

Close the gate
`blocked_until_session_reducer_per_protocol_reassembly` that
holds `DC-EVIDENCE-01` + `RO-LIVE-05` at `enforced_scaffolding`
after the A1.1 closure unmasked it. Per
[[project-phase4-n-m-frag-next]] / commit `222292c`: the
session reducer's `drain_connected_frames` emits one
`DeliverPeerFrame` per mux frame, but cardano-node fragments
large block-fetch responses (Conway blocks exceeding the
65535-byte mux payload limit) across multiple mux frames bearing
the same protocol id. The block-fetch codec received a truncated
CBOR item and failed → wire pump exited with `BlockFetchDecode`.

## Outcome

**FRAG code-level gate CLOSED.** Plus two adjacent operator-pass
unblockers that the FRAG fix unmasked.

| Gate | Origin | Status |
|---|---|---|
| F1 session reducer per-mini-protocol reassembly | Commit `222292c` finding | **closed** |
| Tag-24 unwrap before `decode_block` | Surfaced post-FRAG by `admission_decode_block_failed` diagnostic | **closed** |
| `chain_dep.epoch_nonce` wiring from imported bundle | Surfaced post-FRAG + post-unwrap by `admission_admit_rejected: VrfCert(VerificationFailed)` diagnostic | **closed** |
| Consensus-inputs Eta0 extraction (correct leader-election nonce in bundle) | Surfaced after epoch_nonce wiring fix; VrfCert(VerificationFailed) still fires | **OPEN** — next slice `PHASE4-N-M-NONCE` |

## What shipped

### BLUE code

- `crates/ade_network/src/session/state.rs`:
  - Added `ProtoBuffers` struct with one `Vec<u8>` field per
    `AcceptedMiniProtocol` variant (closed-sum-indexed; no
    `HashMap`).
  - `ProtoBuffers::get_mut(&mut self, p) -> &mut Vec<u8>` is the
    closed-sum dispatch.
  - `ConnectedState` gained a `proto_buffers: ProtoBuffers`
    field, initialized empty in `ConnectedState::new`.

- `crates/ade_network/src/session/event.rs`:
  - `SessionError::ProtocolPayloadMalformed { protocol: u16,
    detail: &'static str }` new closed-sum variant.

- `crates/ade_network/src/session/core.rs`:
  - `drain_connected_frames` now appends each mux frame's
    payload to `connected.proto_buffers.get_mut(proto)` instead
    of emitting `DeliverPeerFrame` directly.
  - New `drain_protocol_items(proto, buf, effects)` — peels
    complete CBOR items off the head of `buf` via
    `ade_codec::cbor::skip_item`. Truncated tails buffered;
    malformed items return `SessionError::ProtocolPayloadMalformed`.
  - New `codec_error_detail(&CodecError)` — closed `&'static str`
    mapping over `CodecError` variants so the SessionError
    detail field stays `&'static str`-only.

### RED / GREEN adjacent fixes

- `crates/ade_runtime/src/network/mux_pump.rs`:
  `session_err_to_halt` extended with the new
  `ProtocolPayloadMalformed` variant (maps to
  `ChainSyncDecodeError`, same bucket as other
  reducer-level mux faults).

- `crates/ade_node/src/admission/runner.rs`:
  - `unwrap_block_fetch_envelope(bytes)` — strips the
    `tag(24, bytes(.cbor era_block))` envelope before handing
    to `decode_block`. The BlockFetch protocol delivers each
    block payload wrapped in tag-24; `decode_block` expects
    the unwrapped inner bytes.
  - `process_block`:
    - Now calls `unwrap_block_fetch_envelope` first; failure →
      `ProcessedBlock::Undecodable`.
    - Hands the unwrapped bytes to both `decode_block` and
      `admit_via_block_validity`.
    - Adds RED operator diagnostics on failure paths:
      `admission_decode_block_failed: ... prefix=... error=...`
      on Undecodable; `admission_admit_rejected: slot=... error=...`
      on Invalid.

- `crates/ade_runtime/src/admission/wire_pump.rs`:
  `finalize` prints `admission_wire_pump: peer=<addr>
  exit=<AdmissionWirePumpResult>` to stderr on any pump exit
  (small RED diagnostic). Already shipped at `222292c`; mentioned
  here for completeness.

- `crates/ade_node/src/admission/bootstrap.rs`:
  Builds `chain_dep` AFTER `import_live_consensus_inputs`
  returns the canonical bundle, using
  `PraosChainDepState::genesis(canonical.epoch_nonce.clone())`
  instead of `Nonce::ZERO`. The genesis seed for
  `seed_to_snapshot` stays `Nonce::ZERO` (the BootstrapAnchor
  binds the imported UTxO + initial_ledger_fp; chain_dep nonce
  is a separate axis supplied by the bundle).

### Tests (8 net new)

In `crates/ade_network/src/session/core.rs::tests`:

- `fragmented_chain_sync_message_assembles_one_deliver`
- `fragmented_block_fetch_block_assembles_one_deliver` —
  with a 70 KB CBOR `bytes(...)` payload (u32-length-encoded;
  the earlier u16-encoded fixture intentionally returned a
  malformed CBOR shape, which would have caused
  `skip_item` to recurse unboundedly on the leftover bytes —
  the v2 fixture properly encodes the length).
- `interleaved_chain_sync_and_block_fetch_fragments_stay_isolated`
- `pipelined_two_chain_sync_messages_in_one_mux_frame_emit_two_delivers`
- `malformed_cbor_at_item_boundary_returns_session_error`
- `truncated_then_complete_two_step_drain`
- `proto_buffers_isolation_across_accepted_protocols`
- `fragmented_replay_equivalence_two_runs_byte_identical`
  (T-DET-01 strengthening explicit test)

Pre-existing
`session_connected_delivers_chain_sync_frame_as_effect` was
updated: the test payload `[0xDE, 0xAD, 0xBE, 0xEF]` (not
valid CBOR) was replaced with `[0x81, 0x00]` (= ChainSync
`RequestNext` wire bytes). The new reducer requires valid
CBOR — the old test was implicitly relying on bytes passing
through unchanged.

Total: 36 session-reducer-related tests green
(`cargo test -p ade_network --lib session`).

### CI

- New gate `ci/ci_check_session_proto_reassembly.sh`:
  asserts exactly one `ProtoBuffers` struct + one
  `drain_protocol_items` function + presence of
  `SessionError::ProtocolPayloadMalformed` + absence of
  `HashMap`/`HashSet` in `session/state.rs` and
  `session/core.rs`.
- Existing gates still green: `ci_check_session_core_closure.sh`,
  `ci_check_clock_seam.sh`, the seed-import gates, the
  live-operator-pass scaffold gate.

### Live evidence

The live operator pass against the fully-synced docker preprod
peer (epoch 291, peer tip slot ~124,134,xxx, syncProgress
100%) now reaches `block_received` events that match the peer's
announced tip hash. This proves the wire-protocol + block-fetch
+ tag-24 + envelope path now end-to-end works.

Captured snippet from a committed run:

```
admission_started        consensus_inputs_fingerprint=8166ba41...
bootstrap_complete       initial_ledger_fp=65461b64... chain_tip_slot=124136968
block_received           peer=127.0.0.1:3001 slot=124138101 block_hash=7850466a...
agreement_verdict        kind=diverged slot=124138101 our_hash=peer_hash=7850466a...
admission_halted         reason=diverged
```

The remaining diverged verdict comes from
`Header(VrfCert(VerificationFailed))` — see "Open obligations"
below.

## Registry effects

- **New rule `CN-SESS-04`** (release, `enforced`): session
  reducer per-mini-protocol payload reassembly authority.
- **New rule `DC-SESS-06`** (derived, `enforced`):
  replay-equivalence under fragmented inbound streams +
  closed-sum `ProtocolPayloadMalformed` error path.
- **`T-DET-01`** (true): `strengthened_in` +=
  `PHASE4-N-M-FRAG`.
- **`DC-EVIDENCE-01`** + **`RO-LIVE-05`**: `strengthened_in` +=
  `PHASE4-N-M-FRAG`. `open_obligation` re-tagged from
  `blocked_until_session_reducer_per_protocol_reassembly`
  (now satisfied) to
  `blocked_until_consensus_inputs_eta0_extraction`.
  `evidence_notes` extended with the post-FRAG event sequence
  + the new gate.

## Open obligations (post-FRAG)

- **DC-EVIDENCE-01 + RO-LIVE-05 BlockAdmitted half** — gated
  on `blocked_until_consensus_inputs_eta0_extraction`. After
  wiring `chain_dep.epoch_nonce` from
  `canonical.epoch_nonce`, the live pass STILL fails with
  `VrfCert(VerificationFailed)`. The existing positive Conway
  corpus (which exercises the SAME `verify_praos_vrf` impl
  successfully) sources `eta0` from a ledger-snapshot's
  `NewEpochState.ticknState`; cardano-cli's `query
  protocol-state` returns a different nonce. The next slice
  (provisional `PHASE4-N-M-NONCE`) needs to identify the
  correct cardano-cli (or `cardano-cli debug`) invocation
  that surfaces the current epoch's Eta0 and update
  `ci/build_consensus_inputs_bundle.sh` accordingly.

- **RO-GENESIS-REPLAY-01** — unchanged.
- **RO-MITHRIL-IMPORT-01** — unchanged.

## What's NOT in this cluster

- Outbound message fragmentation (we never send messages >
  65535 bytes today).
- Consensus-inputs Eta0 extraction fix (next slice).
- BLUE VRF impl changes (no change needed — corpus proves the
  impl works).
- Block production live pass (future cluster).

## References

- Cluster doc: `docs/clusters/PHASE4-N-M-FRAG/cluster.md`.
- Slice doc: `docs/clusters/PHASE4-N-M-FRAG/F1.md`.
- Finding doc commit: `222292c`.
- Predecessor closures: `8843e20` (N-M-C), `03d1d24` (N-M-A1.1),
  `0016722` (A1.1 follow-ups).
- Doctrine: [[feedback-shell-must-not-overstate-semantic-truth]],
  [[feedback-real-interop-finds-codec-bugs]],
  [[project-phase4-n-m-frag-next]].
