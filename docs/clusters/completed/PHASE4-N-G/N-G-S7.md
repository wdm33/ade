# Invariant Slice — PHASE4-N-G S7

## Slice Header

**Slice Name:** Mechanical cross-impl adapter + operator-action `live_block_fetch_session` evidence binary
**Cluster:** PHASE4-N-G
**Status:** In Progress
**CEs addressed:** CE-N-G-7, CE-N-G-8 (conditional)
**Registry effect on merge:** `RO-LIVE-01` → `enforced` if operator-stamped evidence is committed in this slice; else → `partial` with `open_obligation = "blocked_until_operator_peer_available"`. (At HEAD, no private Haskell peer is provisioned, so this slice ships in conditional mode.)
**Dependencies:** N-G-S1..S6

---

## Intent

Close N-G's bounty-evidence half:

* **CE-N-G-7 (mechanical):** drive the full producer-side server
  pipeline against captured N-C corpus and verify the served bytes
  decode + pass body-hash binding through Ade's own validator stack —
  independent of any network.
* **CE-N-G-8 (live):** operator-action binary `live_block_fetch_session`
  that runs against a real cardano-node N2N peer and captures a
  `CE-N-G-LIVE_<date>.log` showing the peer accepting our served
  block bytes.

---

## The change

### 1. New integration test `crates/ade_runtime/tests/cross_impl_server_pipeline.rs`

Drives the full S5 pipeline over the Conway-576 corpus, asserts:
- Every served `Block { bytes }` decodes via
  `ade_codec::cbor::envelope::decode_block_envelope` cleanly.
- For every block decoded via `decode_block`, the recomputed
  `computed_body_hash` equals the header's `body_hash` field.
- The served bytes equal `self_accept`'s input bytes byte-identically.

This is the mechanical pre-condition that proves the bytes we will
serve are validator-acceptable independently of any external Haskell
peer.

### 2. New binary `crates/ade_core_interop/src/bin/live_block_fetch_session.rs`

Mirrors `live_block_production_session.rs` shape:
- Hermetic default mode: prints readiness and exits (for the
  `#[ignore]`'d build-and-start test).
- `--connect` mode: opens N2N session to operator-specified
  cardano-node target, drives the server-side reducers via
  `ade_runtime::network::n2n_server`, writes one JSONL evidence
  record per RequestRange responded.

### 3. New procedure doc `docs/clusters/PHASE4-N-G/CE-N-G-8_PROCEDURE.md`

Documents how an operator runs the binary to capture
`CE-N-G-LIVE_<date>.log` against a private cardano-node peer; records
the `blocked_until_operator_peer_available` status with re-open
obligation if no peer is available at cluster close.

### 4. CI gate `ci/ci_check_server_paths_corpus_present.sh`

Positive presence: the mechanical adapter test exists and is
discoverable; the binary file exists.

---

## §12 Mechanical Acceptance Criteria (named tests)

In `crates/ade_runtime/tests/cross_impl_server_pipeline.rs`:
- `cross_impl_server_pipeline_request_range_returns_decodable_bytes` —
  for every served block, `decode_block_envelope` + `decode_block`
  succeed; recomputed body-hash matches header field.
- `cross_impl_server_pipeline_request_range_byte_identical_to_self_accept_input`
  — served bytes equal corpus-block bytes byte-identically.

In `crates/ade_core_interop/tests/live_block_fetch_session_builds.rs`
(integration; mirrors PHASE4-N-C's `live_block_production_session`
build test):
- `live_block_fetch_session_hermetic_default_prints_readiness` —
  `cargo run --bin live_block_fetch_session` exits 0 in default
  (hermetic) mode and emits the readiness banner.

CI:
- `ci/ci_check_server_paths_corpus_present.sh` (new).

---

## §14 Hard Prohibitions

- The binary's hermetic default MUST NOT open any sockets / read
  operator key material. Live mode is gated behind `--connect`.
- The binary must not depend on `producer::signing` — server pump
  only handles AcceptedBlock-derived bytes. Re-enforced by the
  S6 CI gate (same module tree).
- The procedure doc must not embed operator credentials, peer IPs,
  or private network topology. Public-repo discipline (per
  `feedback-no-credential-leaks`).

---

## §15 Explicit Non-Goals

- Wiring the binary to a specific cardano-node instance — operator-
  scope. The binary takes `--target host:port`.
- Full mux-level back-pressure / multiplexed mini-protocol
  scheduling — narrow scope: single-peer single-protocol drive.

---

## Replay obligations

The mechanical adapter test uses the same corpus + reducer pipeline
as S5; no new corpus.

---

## Authority reminder

If this slice conflicts with the project's normative specifications
or the invariant registry, those win.
