# Cluster PHASE4-N-F-G-B — Self-accept→serve handoff

> **Status: PLANNED** (from committed plan `docs/planning/phase4-n-f-g-cluster-slice-plan.md` +
> declared `DC-NODE-06`). Second sub-cluster of **PHASE4-N-F-G** (RO-LIVE-01). Predecessor:
> **PHASE4-N-F-G-A** (forge fidelity — CLOSED, origin/main `62cb8718`). Code-verified at HEAD
> `1806584c`.
>
> **Cluster character (load-bearing — do not broaden):** serve **authority** only. Prove that
> **only a BLUE self-accepted forged artifact can be served**, via a **sibling** serve task,
> **without relaxing relay-loop containment**. *Not* a live-feed cluster (G-C), *not* a
> peer-evidence cluster (G-C), *not* a forge-validity cluster (G-A, closed).
>
> **Hard line:** a slice may **ADD** a served-chain handoff gate but must **NOT relax**
> `ci_check_node_run_loop_containment.sh` (byte-/semantically unchanged). No live-feed /
> `WirePump` / `n2n_dialer` wiring into the binary (G-C). No BA-02 / RO-LIVE / peer-acceptance
> claim. No new BLUE authority. If a slice needs any of these — **stop and re-scope.**

## Primary invariant
`DC-NODE-06` (declared; `introduced_in = "PHASE4-N-F-G-B"`) — self-accept→serve handoff on the
`--mode node` relay spine: only a BLUE self-accepted forged artifact may enter the sibling
served-chain serve task, via a typed constructor-fenced handoff whose only provenance is the
`AcceptedBlock` produced by BLUE `self_accept` during the forge path that emits `ForgeSucceeded`;
served-chain mutation happens only via the single `ServedChainHandle::push_atomic`; the relay-loop
body performs no serve/admit/gossip/block-fetch/durable-tip mutation; the containment gate stays
semantically unchanged. *(Cited, not restated — see the registry entry.)*

## Invariants strengthened (at close)
- `DC-NODE-06` — `declared` → **enforced** (CE-G-B-1, CE-G-B-2, CE-G-B-3 green).
- `CN-PROD-04`, `CN-CONS-07` — `strengthened_in += "PHASE4-N-F-G-B"` (serve-side producer/consensus
  surfaces wired on the node spine).
- Carried unchanged (not weakened): `CN-FORGE-01` (the `AcceptedBlock` provenance fence),
  `CN-NODE-02`, `CN-WIRE-08`, `DC-NODE-05`, and the now-enforced `DC-EPOCH-03`.

## Normative anchors
- `docs/planning/phase4-n-f-g-cluster-slice-plan.md` (G-B section — 3 CEs, 3 slices).
- `docs/planning/phase4-n-f-g-invariants.md` (the `/invariants` sketch; OQ2 = shape-B sibling serve
  task).
- Registry: `DC-NODE-06` (declared) + the carried/strengthened rules above.

## Entry conditions (what prior clusters guarantee)
- **G-A (closed):** the `--mode node` forge produces a genesis-consistent, slot-aligned,
  epoch-bounded **self-accepted** artifact — i.e. a `ForgeSucceeded` outcome exists to serve. (G-B
  depends on G-A only for a *valid artifact*; the serve **mechanism** is provable on any
  self-accepted artifact.)
- **`CN-FORGE-01`:** `self_accept` (BLUE) is the **sole** producer of `AcceptedBlock` (private-field
  constructor fence); `ForgeSucceeded` is emitted only when that BLUE accept passes.
- **N-R-B / N-S (producer-mode):** `ServedChainHandle::push_atomic`, `producer_block_fetch_serve`,
  and the `n2n_listener`/`n2n_server`/`mux_pump` serve infra already exist — G-B **reuses** them,
  wiring a sibling serve task off the node spine (no new serve authority).
- **N-F-E / N-F-F:** the relay-loop forge tick is **self-accept-only** (`hermetic_forge_outcomes`,
  advances no durable tip, serves nothing); `ci_check_node_run_loop_containment.sh` enforces it.
- **`CN-WIRE-08`:** a single BLUE tag-24 wire-envelope authority (`ade_codec::cbor::tag24`).

## Verified component inventory (read at HEAD `1806584c`, not assumed)
| Component | Real state (verified) | Use |
|---|---|---|
| `ade_ledger::producer::self_accept(...) -> Result<AcceptedBlock, SelfAcceptError>` (`self_accept.rs:66`) | BLUE; **sole** `AcceptedBlock` producer (CN-FORGE-01) | **S1** the only valid source the fence may carry |
| `AcceptedBlock { bytes }` — **private field**, module-only constructor (`self_accept.rs`) | BLUE constructor-fence; un-fabricable from raw bytes | **S1** the typed payload the handoff carries; **S2** `push_atomic` input |
| `CoordinatorEvent::ForgeSucceeded { slot, artifact: ForgedBlockArtifact }`; `ForgedBlockArtifact { slot, hash, bytes }` (`coordinator.rs:183,197`) | the RED forge outcome (raw bytes, **not** the fenced `AcceptedBlock`) | **S1** the outcome the fence keys on — must carry the BLUE `AcceptedBlock`, not re-derive it from bytes |
| `ServedChainHandle::push_atomic(accepted: AcceptedBlock) -> Result<ServedTip, PushError>` (`served_chain_handle.rs:101`); `served_chain_admit` (`served_chain.rs:156`) | RED handle over BLUE admit; **single** served-chain mutation authority (watch channel) | **S2** the sole serve mutation |
| `producer_block_fetch_serve(...)` → `RequestRange` → `MsgBlock` (`block_fetch/server.rs:136`) | BLUE serve; private `RequestRange` field (no fabricated request) | **S3** the block-fetch payload proof |
| `ade_codec::cbor::tag24` (CN-WIRE-08) | single BLUE envelope authority | **S3** the served payload envelope |
| `n2n_listener` / `n2n_server` / `mux_pump` (`ade_runtime::network`) | RED serve infra (N-S/N-R-B, producer-mode) | **S2** the sibling serve task (reused; wired off the node spine) |
| node forge tick `hermetic_forge_outcomes.push(outcome)` (`node_lifecycle.rs:715`) | self-accept-only; no serve/tip | **S1** where the typed handoff attaches |
| `ci_check_node_run_loop_containment.sh` (N-F-E) | forbids serve/tip tokens in the loop body | **UNCHANGED** by G-B (hard line) |

## Slices (safety order)

### S1 — Typed self-accepted-artifact handoff fence *(hermetic)*
A typed, constructor-fenced handoff carrier whose **only** provenance is the `AcceptedBlock`
produced by BLUE `self_accept` during the forge path that emits `ForgeSucceeded`. No constructor
from raw `ForgedBlockArtifact.bytes`, a failed outcome (`ForgeNotLeader` / `ForgeFailed`), a
self-declared acceptance flag, or a peer-verdict substitute — the "serve an unaccepted artifact"
state is **unrepresentable**. The forge path surfaces the BLUE `AcceptedBlock` for the carrier
(rather than re-deriving it from bytes — that would breach CN-FORGE-01). **If the current
`ForgeSucceeded` event cannot safely transport the `AcceptedBlock` without widening RED evidence
semantics, S1 must introduce a separate internal handoff-only carrier adjacent to the forge path;
it must not add a raw-bytes constructor or reinterpret `ForgedBlockArtifact.bytes` as accepted.**
Addresses **CE-G-B-1**. TCB: **GREEN** (constructor fence) + consume **BLUE** (`self_accept`).
*(Slice-entry question: where the `AcceptedBlock` is captured — at `run_real_forge`'s self-accept
point vs. a dedicated handoff carrier — resolved in the slice doc; must not fabricate from
`ForgedBlockArtifact.bytes`.)*

### S2 — Sibling serve task *(hermetic)*
A sibling task (`n2n_listener` + `producer_block_fetch_serve` + `push_atomic`) consumes the S1
handoff and admits via the **single** `push_atomic`; the relay-loop body stays byte-unchanged (the
handoff is a typed channel send, not a serve token / served-chain mutation). Addresses
**CE-G-B-2**. TCB: **RED** (serve task) + consume **BLUE** (`served_chain`).

### S3 — Block-fetch payload/envelope proof + served-chain handoff gate *(hermetic)*
A loopback serve → `RequestRange` → `MsgBlock` whose **payload preserves the self-accepted forged
block bytes** under the single CN-WIRE-08 tag-24 envelope (decode round-trips to the self-accept
input); a **new additive** served-chain handoff CI gate bans raw forge bytes / failed outcomes /
self-declared acceptance / peer-verdict substitutes from the serve-task ingress. Confirms
containment unchanged on the full sub-cluster diff. Addresses **CE-G-B-3**, closes **CE-G-B-2**.
TCB: **RED** test/serve + consume **BLUE** (tag-24 / `block_fetch::server`).

## Exit criteria (mechanical, CI-verifiable)
New test/check names are **candidate** (created by the owning slice); existing artifacts named
as-is.

- **CE-G-B-1 (handoff fence)** — only a BLUE self-accepted artifact (the `AcceptedBlock` from
  `self_accept`) enters the serve task via a typed constructor-fenced handoff; candidate tests
  `handoff_carrier_constructs_only_from_self_accepted_forge`,
  `serve_ingress_rejects_failed_forge_outcome` (or a compile-time unrepresentability assertion),
  `handoff_carrier_has_no_raw_bytes_constructor`. *(introduces the GREEN fence backing
  `DC-NODE-06`.)*
- **CE-G-B-2 (sibling serve task; containment unchanged)** — served-chain mutation only via
  `push_atomic`; relay-loop body byte-unchanged; candidate tests
  `sibling_serve_admits_via_push_atomic_only`, `relay_loop_body_unchanged_with_serve_sibling`;
  existing `ci_check_node_run_loop_containment.sh` **byte-/semantically unchanged + green**.
- **CE-G-B-3 (block-fetch payload/envelope + handoff gate)** — candidate tests
  `block_fetch_payload_is_self_accepted_bytes`, `block_fetch_tag24_round_trips_to_self_accept_input`;
  candidate gate `ci_check_served_chain_handoff_fence.sh` (bans raw-bytes / failed-outcome / flag /
  verdict serve-ingress); existing `ci_check_node_run_loop_containment.sh` still unchanged.
  *(`DC-NODE-06` flips declared→enforced when CE-G-B-1..3 are all green.)*

## TCB color map
- **BLUE (none — reuse only):** `ade_ledger::producer::{self_accept, served_chain}`,
  `ade_network::block_fetch::server`, `ade_codec::cbor::tag24`. A BLUE change is a red flag →
  reject.
- **GREEN:** the typed self-accepted-artifact **handoff fence** (constructor-gated; pure).
- **RED:** the **sibling serve task** — `ade_runtime::network::{n2n_listener, n2n_server,
  mux_pump}`, `ServedChainHandle::push_atomic` (reused); the node-spine wiring that spawns it.
- **CI:** candidate `ci_check_served_chain_handoff_fence.sh` (S3, additive); existing
  `ci_check_node_run_loop_containment.sh` (**unchanged**), `ci_check_no_independent_forge_codepath.sh`,
  the CN-WIRE-08 tag-24 gate continue to hold.

## Forbidden during this cluster *(slice-level prohibitions inherit)*
- **Do not relax `ci_check_node_run_loop_containment.sh`** (byte-/semantically unchanged); no
  serve/tip token in the loop body.
- No **live-feed / `WirePump` / `n2n_dialer` / session wiring into the binary** (that is G-C).
- No **BA-02 / RO-LIVE / peer-acceptance** claim (G-C + operator-gated).
- No **second serve authority** (only `ServedChainHandle::push_atomic`); no **parallel serializer**
  (only CN-WIRE-08 tag-24).
- No **constructor for the handoff carrier** from raw bytes, a failed outcome, a self-declared
  flag, or a peer verdict.
- No new **BLUE authority / canonical type / WAL/checkpoint format**.
- **Hard line:** if the handoff/serve needs a BLUE change, a containment relaxation, live wiring, or
  a second serve authority — **stop and re-scope.**

## Replay obligations (scoped)
No new canonical type (the served payload is the existing canonical forged bytes), no new
authoritative transition, no new corpus entry. Obligation: a given self-accepted artifact ⇒ a
deterministic served admission + deterministic block-fetch payload bytes (unit-tested, not corpus).
Acceptance scoped to touched crates (`ade_node`, `ade_runtime`, consumed
`ade_ledger`/`ade_network`/`ade_codec`) — not the full `ade_testkit` corpus lane.

## Registry impact (at close)
- `DC-NODE-06` (derived) — `declared` → **enforced** across S1–S3.
- `CN-PROD-04`, `CN-CONS-07` — `strengthened_in += "PHASE4-N-F-G-B"`.
- Candidate new gate `ci_check_served_chain_handoff_fence.sh` bound to `DC-NODE-06`.
- **Not added here:** live feed / `WirePump` (G-C); RO-LIVE-01 / BA-02 evidence (G-C,
  operator-gated).

## Non-goals
No live feed / `WirePump` / operator pass / peer acceptance (G-C, operator-gated). No
mainnet-complete serve semantics beyond the self-accepted-artifact handoff + block-fetch payload
proof. No grounding-doc regeneration (that's `/cluster-close`).
