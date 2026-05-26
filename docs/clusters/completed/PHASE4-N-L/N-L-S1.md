# Invariant Slice — PHASE4-N-L S1

**Slice Name:** CI gates for mux frame + handshake closures + closed `AcceptedMiniProtocol` enum.
**Cluster:** PHASE4-N-L
**Status:** In Progress
**CEs addressed:** CE-N-L-1 (CN-SESS-01), CE-N-L-2 (CN-SESS-02), CE-N-L-4 (DC-SESS-02).
**Registry effects on merge:**
- CN-SESS-01 → `enforced` (`ci/ci_check_mux_frame_closure.sh`).
- CN-SESS-02 → `enforced` (`ci/ci_check_handshake_closure.sh`).
- DC-SESS-02 → partial — closed enum + dispatch gate land here; runtime test in S2.

## Intent

Pin the existing single-authority surfaces (mux frame codec, handshake transition) with CI grep gates, and introduce the closed `AcceptedMiniProtocol` enum that the session core (S2) will dispatch over. No production behavior change; this slice locks the structure.

## Scope

- `crates/ade_network/src/session/event.rs` (new — partial) — `AcceptedMiniProtocol` closed enum.
- `crates/ade_network/src/session/mod.rs` — re-export.
- `ci/ci_check_mux_frame_closure.sh` — single pub `encode_frame`/`decode_frame`.
- `ci/ci_check_handshake_closure.sh` — single pub `n2n_transition` / `n2c_transition`.
- `ci/ci_check_mini_protocol_id_registry_closed.sh` — closed `AcceptedMiniProtocol` enum + closed match in dispatch site (the latter lands in S2; the gate tolerates missing dispatch site at this slice).

## §12 Mechanical Acceptance Criteria

- [ ] `ci/ci_check_mux_frame_closure.sh` passes.
- [ ] `ci/ci_check_handshake_closure.sh` passes.
- [ ] `ci/ci_check_mini_protocol_id_registry_closed.sh` passes.
- [ ] `AcceptedMiniProtocol::tests::round_trips_all_mini_protocol_ids` — round-trip MiniProtocolId ↔ AcceptedMiniProtocol for the closed set.

## §14 Hard Prohibitions

- No new pub `encode_frame`/`decode_frame`/`n2n_transition` outside the existing sites.
- No `_` wildcard accept in the dispatch match (a future-proofing wildcard would silently accept unknown ids).

## §15 Non-Goals

- No session reducer (S2). No demux (S3). No I/O.
