# Invariant Sketch — PHASE4-N-F-G-E: Live-feed bounded memory

> **Type:** IDD invariant sketch (Part I). Planning artifact. Predecessor:
> PHASE4-N-F-G-C (live feed + operator-gated evidence — CLOSED, `main` `351d46bc`).
> Prompted by the PHASE4-N-F-G-C per-cluster security review (MEDIUM): the
> live `--mode node` feed newly **exposes** two pre-existing unbounded
> peer-driven memory surfaces (reused N-M-FRAG / N-M-C infra). This cluster
> bounds them **before a serious live/private run**.

---

## 0. Framing (read first)

The G-C live-feed wiring is a *fill* of the closed `NodeBlockSource::WirePump`
arm — it did not introduce these surfaces, it made them reachable on the
`--mode node` binary path. Two peer-driven memory surfaces grow without a
closed bound:

- **GREEN** — `ade_network::session::core`: a per-mini-protocol reassembly
  buffer (`proto_buffers[proto]`) accumulates frame payloads via
  `extend_from_slice` until a complete CBOR item is drained. A peer that
  streams frames whose item never completes grows the buffer unbounded
  (bounded-rate — each mux frame is ≤ 64 KiB).
- **RED** — `ade_node::node_sync` WirePump: `lookahead: VecDeque<Vec<u8>>` is
  filled by a non-blocking `try_recv` drain and emptied by `next_block`; if the
  peer outpaces the consumer the deque grows unbounded over time.

The prior CBOR length-overflow remote-DoS (fixed in N-X) is **not** in scope and
is **not** reintroduced — that was an in-slice decode-length bug; this is a
pre-decode reassembly/scheduling memory bound.

**These are defensive implementation bounds, not Cardano semantic parameters.
They may be tightened by a future hardening slice, but no runtime option may
disable them or set them to unbounded.**

---

## 1. What must always be true

- **A1.** Peer-driven memory on the live `--mode node` feed is **bounded before
  authoritative decode/apply**. A single per-mini-protocol reassembly buffer
  never exceeds **16 MiB**; the WirePump lookahead never exceeds **256** buffered
  blocks.
- **A2.** Over-cap **fails closed**: the reassembly-tail over-cap returns a
  structured `SessionError` (drop the peer); the lookahead at-cap **stops
  opportunistic draining**, letting the existing bounded mpsc (cap 64)
  back-pressure the pump. No silent truncation, no partial decode, no unbounded
  fallback.
- **A3.** The bounds are **closed constants** in source — not wired to CLI / env
  / config. There is no runtime escape hatch to disable them or set them
  unbounded.

## 2. What must never be possible

- **N1.** Unbounded growth of a reassembly buffer or the WirePump lookahead from
  any peer input.
- **N2.** Swallowing a malformed / endless-incomplete item as success — an
  over-cap reassembly is an **error** (drop), never a "no more data" or a
  truncated-but-accepted item.
- **N3.** A partial / truncated decode crossing into BLUE — the cap fires
  **before** authoritative decode/apply; only complete, in-bound items reach the
  BLUE decode path (unchanged).
- **N4.** A runtime option (CLI flag, env var, config field) that disables a
  bound or sets it to unbounded.
- **N5.** Any change to the relay-loop containment, the served-chain handoff
  fence, or the forge — this cluster touches only the pre-decode memory bound.

## 3. Deterministic surface

- **I1.** The reassembly-tail cap is a **pure function of the bytes seen**
  (`buf.len() > 16 MiB`) — the GREEN reducer stays deterministic; the same frame
  sequence yields the same `SessionError` / drain outcome.
- **I2.** The lookahead-depth cap is RED scheduling state (a `VecDeque` depth
  check) — it never observes block *content*, never reorders, and does not
  affect any BLUE/GREEN authoritative output (the verdict-decoupled
  `NodeBlockSource` contract is unchanged; a buffered block is still delivered
  next in arrival order once below the cap).

## 4. Replay-equivalence

- **R1.** No new authoritative state, no new canonical type, no WAL/checkpoint
  change. The caps are pre-decode scheduling/fail-closed bounds; replay is
  unaffected (the same captured ordered feed replays identically, and an
  over-cap input deterministically fails closed both runs).

## 5. State transitions in scope

| # | Transition | Color | Status |
|---|---|---|---|
| T1 | `(SessionState, frame) → Err(SessionError::ReassemblyBufferOverflow)` when `proto_buffers[proto].len() > 16 MiB` | GREEN | **NEW** — additive closed `SessionError` variant + the bound check |
| T2 | `pump_lookahead` stops draining at `lookahead.len() >= 256` → bounded channel back-pressures | RED | **NEW** — depth guard in the existing opportunistic drain |

## 6. TCB color

- **GREEN:** `ade_network::session::core` reassembly-tail cap + the additive
  `SessionError` variant.
- **RED:** `ade_node::node_sync` WirePump lookahead-depth cap.
- **BLUE:** unchanged. No BLUE crate is touched.

## 7. Registry surface (one new derived rule — proposed)

- **`DC-LIVEMEM-01`** — *tier: derived (operational-hardening; NOT BLUE
  consensus law).* Peer-driven live-feed memory is bounded before authoritative
  decode/apply: a per-mini-protocol reassembly buffer over 16 MiB fails closed
  with a structured `SessionError`; the WirePump lookahead stops opportunistic
  draining at 256 blocks (the bounded channel back-pressures). The bounds are
  closed constants — no runtime/CLI/env/config disable or unbounded option.
  Enforcement: the four named tests + (optionally) a cheap grep gate that both
  constants exist and are not wired to CLI/env/config. `introduced_in =
  PHASE4-N-F-G-E`; `declared` at sketch, `enforced` at cluster close.
  Cross-ref: `CN-SESS-04` / `DC-SESS-06` (reassembly), `DC-SYNC-01` / `DC-SYNC-02`
  (the feed → tip path), `DC-NODE-06` / `CN-NODE-02` (carried unchanged).

> **Deliberately NOT in scope:** any live evidence / BA-02 / rehearsal claim;
> any RO-LIVE flip; any serve/forge/containment change.

## 8. Generation notes

- One slice (one invariant family: "peer-driven live-feed memory is bounded
  before authoritative decode/apply"). Both surfaces are the same live-feed DoS
  boundary; the acceptance proof is small enough to close together.
- Next: `/cluster-doc PHASE4-N-F-G-E` → `/slice-doc` → implement on
  `harden/live-feed-bounded-memory`.
