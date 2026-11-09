# PHASE4-N-S — Cluster & slice plan

> **Predecessor invariants:** [`phase4-n-s-invariants.md`](./phase4-n-s-invariants.md).
> **3 sub-clusters:** N-S-A, N-S-B, N-S-C.
>
> **Hard guardrail:** N-S-A and N-S-B MUST close mechanically
> in CI without operator action. Only N-S-C is operator-
> dependent. This prevents the cluster from turning into
> an operator-blocked mega-slice. If A or B start drifting
> toward "needs the operator," halt and revise scope.
>
> **OQ1 finding (pre-confirmed at plan time):**
> `ade_core::consensus::kes_check::verify_header_kes` already
> consumes `&kes.header_body_bytes` (line 106) as the KES
> signing input. The pre-image bytes are the CBOR encoding
> of `ShelleyHeaderBody` (the header's first element in the
> outer `[header_body, kes_signature]` array). N-S-A's
> canonical recipe is therefore "CBOR-encode
> `ShelleyHeaderBody` → `UnsignedHeaderPreImage`"; the
> reference fixture compares Ade's recipe output against
> `header_input::decode_block`'s extracted `header_body_bytes`
> for a corpus block.

## §1 Sub-cluster scope summary

### PHASE4-N-S-A — KES signs real unsigned-header pre-image

**Closes:** I1, I2, I3, N1, N2, N3, D1, D2, R1.
**Mechanical closure** — all exit gates are unit tests +
fixture byte-match + an integration test asserting
`run_real_forge` reaches `ForgeSucceeded` against a
synthetic-stake corpus.

### PHASE4-N-S-B — MuxPump outbound relay

**Closes:** I4, I5, N4, N5, N8, D3, D4.
**Mechanical closure** — all exit gates are unit tests +
a loopback integration test (synthetic dialer ↔ Ade
listener exchange frames; reply bytes byte-identical) + a
CI grep gate forbidding `produce_mode` from writing
directly into `MuxTransportHandle::outbound`.

### PHASE4-N-S-C — Paired acceptance evidence

**Closes:** I6, N6, N7, R2; conditionally strengthens /
narrows `CN-CONS-06` and `RO-LIVE-01`.
**Operator-dependent** — C1 (private testnet) requires
the operator running a hermetic private-testnet harness;
C2 (preprod) requires preprod stake to be provisioned for
the operator's cold key.

C is the only sub-cluster that may be blocked at the
boundary. A and B MUST close before C begins.

## §2 Slice index

| Sub-cluster | Slice | Purpose | Closes (invariant IDs) | Closure type |
|---|---|---|---|---|
| N-S-A | **A1** | Planning + 3 candidate registry entries (`CN-KES-HEADER-01`, `DC-KES-HEADER-01`, `CN-PREIMAGE-FIXTURE-01`) declared. Pre-flight: OQ1 audit recorded; OQ2 audit (forge_block construction sequence) recorded; **OQ-S-A reference fixture captured** at `crates/ade_ledger/tests/fixtures/unsigned_header_preimage/`. | — | declarative |
| N-S-A | **A2** | BLUE module `ade_ledger::block_validity::unsigned_header_pre_image` with `UnsignedHeaderPreImage(Vec<u8>)` branded type + canonical recipe `unsigned_header_pre_image(...) -> UnsignedHeaderPreImage`. **Reference fixture byte-match test**: Ade's recipe output against a corpus block's `header_input::decode_block.header_body_bytes` byte-equal. Refactor `verify_header_kes` to consume the branded type (single source of truth — grep gate). | I1, I2, N2, D1 | mechanical (cargo test) |
| N-S-A | **A3** | New RED API `producer_shell.kes_sign_header(period, &UnsignedHeaderPreImage) -> KesSignature` that wraps the existing `kes_sign_at` but accepts only the branded type. Refactor `forge_block` (or thread a new bridge) so the §5 construction sequence holds: body → body_hash → body_size → header pre-image → KES sign → signed header. Replace `run_real_forge` step 3's placeholder with the real call. | N1, N3, D2 | mechanical (cargo test) |
| N-S-A | **A4** | Integration tests + sub-cluster close. The N-R-A A4 test `full_stake_answer_reaches_self_accept_and_rejects` is renamed/inverted: with the real pre-image + KES signature, `self_accept` MUST now return `Accepted` for a full-stake answer (the structural ForgeSucceeded branch becomes reachable). Add `forge_signed_block_self_accepts_for_synthetic_full_stake_corpus`. Flip the 3 N-S-A rules to `enforced`; record strengthenings on `CN-FORGE-01` (Succeeded path closed), `DC-CONS-18` (body-hash binding under real signed header). | I3, R1; sub-cluster close | mechanical |
| N-S-B | **B1** | Planning + 3 candidate registry entries (`CN-OUTBOUND-RELAY-01`, `CN-PEER-OUTBOUND-MAP-01`, `DC-OUTBOUND-FIFO-01`) declared. OQ-S-B audit: `MuxPump::run` `tokio::select!` shape after adding outbound receiver; no double-mut-borrow of `transport.outbound`. | — | declarative |
| N-S-B | **B2** | New closed `OutboundCommand` enum + `MuxPump::outbound_relay: Option<mpsc::Receiver<OutboundCommand>>` field + `tokio::select!` integration (poll receiver alongside `transport.inbound`; route OutboundCommands through the **existing** session-aware encoder — same path `SessionEffect::SendBytes` uses). | I4, N4, N8, D3 | mechanical |
| N-S-B | **B3** | `PerPeerOutbound = Arc<RwLock<BTreeMap<PeerId, mpsc::Sender<OutboundCommand>>>>` type alias + insertion/removal in `run_per_peer_session` (on PeerConnected emit) and on `MuxPump::emit_peer_disconnected`. `dispatch_server_frame_event` extended with `&PerPeerOutbound` parameter; reducer outputs converted to `OutboundCommand::{ChainSync,BlockFetch}` and enqueued through the per-peer sender. Closed `DispatchError::{UnknownPeer, PeerOutboundMissing, SendFailure}`. | I5, N5, D4 | mechanical |
| N-S-B | **B4** | Loopback integration test (synthetic dialer issues `RequestRange` against Ade listener; received `Block(bytes)` is byte-identical to the snapshot's admitted bytes). CI grep gate `ci/ci_check_no_produce_mode_direct_transport_writes.sh`. Sub-cluster close. Flip the 3 N-S-B rules + clear `CN-PROD-01.open_obligation`'s remaining "MuxPump outbound-relay" sub-item. | I4 / I5 end-to-end; sub-cluster close | mechanical (loopback, no real peer) |
| N-S-C | **C1** | **Hermetic private testnet pass.** Bring up a single-pool private testnet (Cardano `cardano-node` in `--genesis ... --shelley-operational-certificate ... --shelley-vrf-key ... --shelley-kes-key ...` mode with all stake delegated to the operator's pool). Run `ade_node --mode produce` against it; capture Ade `evidence.jsonl` + raw `docker logs` peer log + pair manifest with `peer_log_file_sha256`. **C1's success is what flips the deferred-bridge open_obligation entries** (`CN-FORGE-01.open_obligation`, `CN-PROD-01.open_obligation` final remainder, `CN-SNAPSHOT-01.open_obligation`). | I6, N6, N7 (private-testnet path); end-to-end bridge proof | operator-witnessed (private testnet) |
| N-S-C | **C2** | **Preprod operator pass (bounty-facing strengthening).** Once the operator has provisioned preprod stake for the cold key (delegation registered, snapshot active), run the same pass against docker `cardano-node-preprod`. Capture paired evidence. **C2's success conditionally strengthens / narrows `CN-CONS-06` and `RO-LIVE-01`** — full enforcement only if all their remaining sub-items (cross-impl evidence, byte-identity served bytes, …) are covered. | R2; CN-CONS-06 / RO-LIVE-01 conditional strengthening | operator-witnessed (preprod) |
| N-S-C | **C-close** | Cluster-level close. Document remaining bounty surfaces (TxSubmission2, mempool, N2C, multi-Haskell-node) as out-of-scope-for-N-S with their own follow-on cluster targets. Update grounding docs (CODEMAP / SEAMS / HEAD_DELTAS / TRACEABILITY) at cluster close. | cluster close | mechanical (documentation) |

## §3 Dependency graph

```
A1 (planning + reference fixture)
 │
 └── A2 (BLUE branded type + recipe + byte-match fixture test)
      │
      └── A3 (RED kes_sign_header(&UnsignedHeaderPreImage) + forge_block refactor)
           │
           └── A4 (forge → Accepted integration test + N-S-A close)
                │
                └── (independent of B; A4 alone does NOT need B)
                │
                +─── B1 (planning + OQ-S-B audit)  ← can start in parallel with A2
                      │
                      └── B2 (OutboundCommand enum + MuxPump outbound_relay)
                           │
                           └── B3 (PerPeerOutbound map + dispatch wiring)
                                │
                                └── B4 (loopback test + N-S-B close)
                                     │
A4 + B4 ──> C1 (private testnet pass — operator)
             │
             └── C2 (preprod operator pass — operator, gated on stake)
                  │
                  └── C-close (cluster close + grounding-doc refresh)
```

**Hard ordering:**

- A1 ≺ A2 ≺ A3 ≺ A4 (A2's recipe needed by A3's signing; A3's
  composition tested by A4).
- B1 ≺ B2 ≺ B3 ≺ B4.
- A4 + B4 ≺ C1 (private-testnet pass needs both bridges
  landed; without B's transmit wiring peer can't fetch).
- C1 ≺ C2 (private testnet proves the bridge; preprod is
  the bounty-facing strengthening).

**Soft ordering / parallelism:**

- A1 + B1 can land in the same commit (both are
  declarative + pre-flight audits).
- A2/A3 and B2/B3 can be developed in parallel — A's work
  is in `ade_ledger` + `ade_runtime::producer`; B's is in
  `ade_runtime::network::mux_pump` + `ade_node::produce_mode`.
- C2 may be **operator-blocked indefinitely** if preprod
  stake isn't available. C1 alone is sufficient to close
  the deferred-bridge open_obligation entries from N-R.

## §4 Pre-flight proof obligations (mechanical, not design)

| ID | What | Captured under | Consumed by |
|---|---|---|---|
| **OQ1** | Confirm `verify_header_kes` consumes `&header_body_bytes` (already audited at plan time; line 106 of `kes_check.rs`). The pre-image is the CBOR encoding of `ShelleyHeaderBody`. | A1 slice doc | A2 |
| **OQ2** | Audit `forge_block`'s current construction sequence. Currently it accepts a `tick.kes_signature` as a pre-computed field. Decide: (a) refactor `forge_block` to compute the signature internally given `&KesSecret`, OR (b) thread a new bridge function in `ade_runtime::producer::scheduler` that runs the §5 sequence and feeds `forge_block` a tick with the now-real KES signature. Recommendation: (b) — minimal disruption. | A1 slice doc | A3 |
| **OQ-S-A** | Capture a reference unsigned-header pre-image fixture from a corpus block. Source: `ade_testkit::validity::corpus::ConwayValidityCorpus.blocks[i]`; extract `decode_block(bytes).header_input.header_body_bytes` via `ade_ledger::block_validity::header_input::decode_block`. Commit as `crates/ade_ledger/tests/fixtures/unsigned_header_preimage/conway_corpus_block_0.preimage.bin` + metadata. A2's byte-match test consumes it. | A1 slice doc + fixture directory | A2 |
| **OQ-S-B** | Audit `MuxPump::run` after adding `outbound_relay: Option<mpsc::Receiver<OutboundCommand>>`. Confirm: (a) `tokio::select!` polls inbound + outbound without double-mut-borrow on `transport.outbound`; (b) when outbound_relay is `None` (e.g., dialer mode), the select degrades cleanly to inbound-only. | B1 slice doc | B2 |

## §5 Design decisions (locked in invariants — restated)

| DQ / OQ | Resolution | Location |
|---|---|---|
| **Outbound channel shape** | `OutboundCommand` closed enum carrying typed `ChainSyncServerMsg` / `BlockFetchServerMsg`. **Not** `Vec<u8>`. MuxPump's session-aware encoder is the sole producer of wire-byte streams. | invariants §5 + N8 |
| **Per-peer map ownership** | `Arc<RwLock<BTreeMap<PeerId, mpsc::Sender<OutboundCommand>>>>`. Listener writes; produce_mode reads. `BTreeMap` (not `HashMap`). Structured `DispatchError::{UnknownPeer, PeerOutboundMissing, SendFailure}`. | invariants §5 + N5 |
| **KES signing input** | Branded `UnsignedHeaderPreImage(Vec<u8>)`. Only constructor: the canonical BLUE recipe function. `kes_sign_header(&UnsignedHeaderPreImage)` is the only signing entry point — arbitrary bytes structurally rejected. | invariants §5 + N1 |
| **Construction sequence** | 9-step ordering (body → body_hash → body_size → header_body → pre-image → KES sign → assemble → block → self_accept). If `forge_block` can't expose this cleanly, A3 refactors. **No placeholder/partial-block signing patches.** | invariants §5 |
| **Evidence source** | Raw `docker logs` captured output + manifest with `peer_log_capture_command`, `peer_log_filter`, `peer_log_file_sha256`. Grep is documentation, not authority. | invariants §1 + §2 |
| **N-S-C path** | C1 (private testnet, hermetic) proves the bridge end-to-end and flips deferred-bridge open_obligations. C2 (preprod) is the bounty-facing strengthening for `CN-CONS-06` and `RO-LIVE-01`. C1 is NOT a substitute for C2. | invariants §8 |

## §6 Candidate registry entries (proposed; appended at each sub-cluster's planning slice)

Per the N-R precedent, registry entries are proposed at each
sub-cluster's first slice, declared, then flipped at close.

### N-S-A introduces

- **`CN-KES-HEADER-01`** — KES signs the canonical
  unsigned-header pre-image (CBOR-encoded
  `ShelleyHeaderBody`); single source of truth shared by
  `forge_block` and `verify_header_kes`. Branded
  `UnsignedHeaderPreImage` mechanically rejects arbitrary
  byte sequences.
- **`DC-KES-HEADER-01`** — `unsigned_header_pre_image(...) -> UnsignedHeaderPreImage`
  is a pure BLUE function; same inputs → byte-identical
  output.
- **`CN-PREIMAGE-FIXTURE-01`** — Ade's recipe output is
  byte-identical to `header_input::decode_block.header_body_bytes`
  for the captured corpus reference fixture. Enforces
  cross-impl bytes-shape claim at A2 close.

### N-S-B introduces

- **`CN-OUTBOUND-RELAY-01`** — `OutboundCommand` is the
  sole channel between `produce_mode` and `MuxPump`'s
  outbound encoder; no `Vec<u8>` byte tunnel; no direct
  `MuxTransportHandle::outbound` write from `produce_mode`.
- **`CN-PEER-OUTBOUND-MAP-01`** — per-peer
  `BTreeMap<PeerId, Sender<OutboundCommand>>` enforces
  no-cross-leakage structurally; lookup failure surfaces
  as `DispatchError::{UnknownPeer, PeerOutboundMissing}`.
- **`DC-OUTBOUND-FIFO-01`** — per-peer outbound channel
  preserves FIFO order: responses queued for `PeerId(p)`
  in order O₁..Oₙ arrive at the peer's TCP socket in the
  same order.

### N-S-C introduces

- **`CN-OPERATOR-EVIDENCE-01`** — bounty-facing paired
  evidence manifest schema is enforced: every
  `CE-N-S-LIVE_*.toml` carries `peer_log_capture_command`,
  `peer_log_filter`, `peer_log_file_sha256`, and the
  asserted `acceptance_keyword_match` field.

(Carry-forward strengthenings handled at each close: A4 →
`CN-FORGE-01` / `DC-CONS-18`; B4 → `CN-PROD-01` /
`DC-CONS-17` final remainder cleared; C1 → bridge
open_obligation fields cleared; C2 → conditional flips on
`CN-CONS-06` / `RO-LIVE-01`.)

## §7 Cluster-level exit criteria

PHASE4-N-S as a whole closes when ALL of:

1. N-S-A + N-S-B sub-clusters' `cluster.md`s exist with
   their own exit criteria; all CE green; all 6 of their
   registry entries flipped to `enforced`.
2. N-S-C is one of:
   - **C1 + C2 captured** — both registry entries
     introduced (CN-OPERATOR-EVIDENCE-01 enforced) +
     `CN-CONS-06` / `RO-LIVE-01` strengthened/narrowed
     per actual evidence; OR
   - **C1 captured, C2 operator-blocked** — C1's manifest
     committed; C2 carried as
     `blocked_until_preprod_stake_available` on
     `CN-OPERATOR-EVIDENCE-01.open_obligation`. This is a
     valid close path.
3. `cargo test --workspace --lib` clean.
4. New CI gates pass: `ci/ci_check_unsigned_header_preimage_single_source.sh`
   (A2's grep gate), `ci/ci_check_no_produce_mode_direct_transport_writes.sh`
   (B4's grep gate), plus the carry-forward gates.
5. Grounding docs (CODEMAP / SEAMS / HEAD_DELTAS /
   TRACEABILITY) refreshed at cluster close.

## §8 Mechanical-vs-operator scope discipline

Per the user's guardrail: **N-S-A and N-S-B must close
mechanically in CI.** Each sub-cluster's exit criteria are
restricted to:

- Unit tests in `cargo test --workspace --lib`.
- Integration tests in `cargo test -p <crate>`.
- Loopback tests using synthetic dialer/listener pairs
  (no real cardano-node peer required).
- Fixture byte-match tests against committed corpus
  artifacts.
- CI grep gates that mechanically forbid forbidden patterns.

**N-S-C is the ONLY sub-cluster whose exit criteria require
operator action.** If C1's private-testnet pass requires
non-trivial bring-up, **A and B must already be closed
before C1 begins** — they cannot be blocked on C's progress.

## §9 Out of scope (deferred to future clusters)

| Item | Tracked under | Cluster |
|---|---|---|
| TxSubmission2 → mempool → block-inclusion path | new family TBD | TxSubmission cluster |
| N2C local-chain-sync / local-tx-submission surfaces | new family TBD | N2C cluster |
| Private-testnet two-Haskell-node bounty leg | new family TBD | multi-node cluster |
| Multi-peer concurrent forge load | — | future |
| Mlocked KES memory | OP-OPS-04 follow-on | future operational |
| Hot-key KES rotation across periods | OP-OPS-04 follow-on | future operational |

Empty-block forging remains the explicit scope inherited
from N-R. The runbook in C1/C2 will state this so
empty-block evidence is not misread as closing the broader
TxSubmission obligation.

## §10 References

- Invariants: [`phase4-n-s-invariants.md`](./phase4-n-s-invariants.md).
- Predecessor cluster: [`../clusters/PHASE4-N-R-A/cluster.md`](../clusters/PHASE4-N-R-A/cluster.md) (+
  `PHASE4-N-R-B/cluster.md`, `PHASE4-N-R-C/cluster.md`).
- N-R close: [[project-phase4-n-r-closed]].
- Bounty: [[project-bounty-requirements]].
- Doctrine:
  - [[feedback-hard-closure-gates]] — N-S-A/B exit criteria
    are hard, mechanical gates.
  - [[feedback-proof-discipline]] — OQ-S-A reference
    fixture is a proof obligation captured BEFORE A2
    implementation.
  - [[feedback-shell-must-not-overstate-semantic-truth]] —
    N-S proves the block-production acceptance leg; does
    NOT close TxSubmission/mempool/N2C/multi-node legs.
  - [[feedback-fail-closed-validation]] —
    `UnsignedHeaderPreImage` branded gate; arbitrary bytes
    structurally rejected.
  - [[feedback-bounded-smoke-slices]] — C1 (private testnet)
    and C2 (preprod) are distinct evidence legs.
  - [[reference-local-preprod-docker-cardano-node]] — C2
    target.
