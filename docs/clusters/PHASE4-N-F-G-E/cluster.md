# Cluster PHASE4-N-F-G-E — Live-feed bounded memory

> **Status: PLANNED** (from the `/invariants` sketch `docs/planning/phase4-n-f-g-e-invariants.md`).
> Hardening follow-on to **PHASE4-N-F-G-C** (live feed + operator-gated evidence — CLOSED, `main`
> `351d46bc`), prompted by the G-C per-cluster security review (MEDIUM). Code-verified at HEAD
> `351d46bc`.
>
> **Cluster character (load-bearing — do not broaden):** a **defensive memory bound** on the live
> `--mode node` feed, applied **before authoritative decode/apply**. It bounds two pre-existing
> peer-driven memory surfaces that G-C newly *exposed* on the binary path (reused N-M-FRAG / N-M-C
> infra). **These are defensive implementation bounds, not Cardano semantic parameters. They may be
> tightened by a future hardening slice, but no runtime option may disable them or set them to
> unbounded.**
>
> **Hard line:** **NO BLUE change**; no semantic truncation; no swallowing malformed/incomplete input
> as success; no unbounded or operator-disable option; no live-evidence / BA-02 / rehearsal claim; no
> serve / forge / containment change (`ci_check_node_run_loop_containment.sh` +
> `ci_check_served_chain_handoff_fence.sh` byte-unchanged). If a slice needs any of these — **stop and
> re-scope.**

## Primary invariant
`DC-LIVEMEM-01` (declared; `introduced_in = "PHASE4-N-F-G-E"`; *tier: derived — operational-hardening,
NOT BLUE consensus law*) — peer-driven live-feed memory is **bounded before authoritative decode/apply**:
a per-mini-protocol reassembly buffer over **16 MiB** fails closed with a structured `SessionError`; the
WirePump lookahead stops opportunistic draining at **256** blocks (the existing bounded mpsc, cap 64,
back-pressures the pump). The bounds are **closed constants** — no runtime / CLI / env / config disable
or unbounded option. *(Cited, not restated — see the registry entry + the invariants sketch.)*

## Invariants strengthened / introduced (at close)
- `DC-LIVEMEM-01` — `declared` → **enforced** (CE-G-E-1 green).
- Carried unchanged (not weakened): `CN-SESS-04` / `DC-SESS-06` (the session reassembly contract — the
  cap is an additive bound on it), `DC-SYNC-01` / `DC-SYNC-02` (the feed → tip path),
  `DC-NODE-06` / `CN-NODE-02` (the serve handoff + relay-loop containment — gates byte-unchanged),
  the closed verdict-decoupled `NodeBlockSource` contract.

## Normative anchors
- `docs/planning/phase4-n-f-g-e-invariants.md` (the `/invariants` sketch — 1 CE, 1 slice).
- The PHASE4-N-F-G-C per-cluster security review (MEDIUM finding) + SEAMS §7 candidate #6.
- Registry: `DC-LIVEMEM-01` (declared) + the carried rules above.

## Entry conditions (what prior clusters guarantee)
- **G-C (closed, `351d46bc`):** the live `NodeBlockSource::WirePump` feed is wired on the `--mode node`
  On arm (`dial_for_admission → run_admission_wire_pump → from_wire_pump`); the WirePump lookahead +
  the session reassembly path are now reachable on the binary path. This cluster bounds them.
- **N-M-FRAG (`CN-SESS-04` / `DC-SESS-06`):** the session reducer's per-mini-protocol payload
  reassembly (`proto_buffers`) — G-E adds a byte cap to it (no behavioral change under the cap).
- **N-F-C/D (`DC-SYNC-01/02`):** the WirePump source + `run_node_sync → pump_block` single tip-advance
  path — G-E adds a lookahead-depth cap (the bounded channel already exists at cap 64).
- The BLUE decode path (`ade_codec`) is unchanged; the cap fires **before** it.

## Verified component inventory (read at HEAD `351d46bc`, not assumed)
| Component | Real state (verified) | Use |
|---|---|---|
| `ade_network::session::core::step` reassembly: `connected.proto_buffers.get_mut(proto).extend_from_slice(&frame.payload)` (`core.rs:231-232`) | GREEN reducer; unbounded `extend_from_slice` until a complete CBOR item is drained; `UnexpectedEof` = item not yet complete | **S1** the reassembly-tail cap site |
| `SessionError` (closed enum, `ade_network::session`) | GREEN structured error sum | **S1** gains an additive `ReassemblyBufferOverflow` variant (closed-enum extension) |
| `ade_node::node_sync` WirePump `lookahead: VecDeque<Vec<u8>>` + `pump_lookahead` (non-blocking `try_recv` drain) (`node_sync.rs:74,103`) | RED scheduling state; content-blind; unbounded depth if peer outpaces `next_block` | **S1** the lookahead-depth cap site |
| WirePump bounded mpsc (`LIVE_WIRE_PUMP_CHANNEL_CAP = 64`) | RED; already bounded (G-C) | **S1** the back-pressure mechanism once the lookahead is at cap |
| `ci_check_node_run_loop_containment.sh` + `ci_check_served_chain_handoff_fence.sh` | the containment + serve fences | **UNCHANGED** by G-E (hard line) |

## Slices
### S1 — Bound the live-feed memory surfaces *(hermetic)*
Add two closed-constant caps applied **before authoritative decode/apply**: (1) GREEN — a 16 MiB
per-mini-protocol reassembly-tail cap in `session::core` that returns a structured
`SessionError::ReassemblyBufferOverflow` (drop the peer) when a buffer exceeds the bound without yielding
a complete item; (2) RED — a 256-block WirePump lookahead-depth cap in `node_sync` that stops
opportunistic draining (the bounded mpsc then back-pressures the pump). No silent truncation, no partial
decode, no unbounded fallback, no runtime/CLI/env/config escape hatch. Addresses **CE-G-E-1**. TCB:
**GREEN** (`session::core` cap + `SessionError` variant) + **RED** (`node_sync` lookahead cap).

## Exit criteria (mechanical, CI-verifiable)
New test/check names are **candidate** (created by the slice); existing gates named as-is.

- **CE-G-E-1 (live-feed memory bounded + fail-closed)** — candidate tests:
  - `session_reassembly_tail_over_cap_fails_closed` — a per-protocol reassembly buffer over 16 MiB
    returns `SessionError::ReassemblyBufferOverflow` (drop), never silent truncation / partial decode.
  - `session_reassembly_tail_under_cap_still_drains_complete_item` — a normal (under-cap) multi-frame
    item still reassembles + drains unchanged.
  - `wirepump_lookahead_stops_at_cap` — `pump_lookahead` stops draining at 256 buffered blocks.
  - `wirepump_lookahead_cap_preserves_relay_behavior_under_normal_feed` — under a normal feed the cap is
    never hit and relay/sync behavior is unchanged.
  - existing `ci_check_node_run_loop_containment.sh` + `ci_check_served_chain_handoff_fence.sh`
    **byte-unchanged + green**.
  - candidate gate `ci_check_live_feed_memory_bounds.sh` (only if cheap + stable): both constants exist
    and are **not** wired to CLI / env / config.
  - `cargo test -p ade_network` + `cargo test -p ade_node` green.
  *(`DC-LIVEMEM-01` flips declared→enforced when CE-G-E-1 is green.)*

## TCB color map
- **BLUE (none — unchanged):** `ade_codec` decode path (the cap fires before it). A BLUE change is a red
  flag → reject.
- **GREEN:** `ade_network::session::core` (the 16 MiB reassembly-tail cap + the additive `SessionError`
  variant) — deterministic (a pure function of bytes seen).
- **RED:** `ade_node::node_sync` (the 256-block WirePump lookahead-depth cap) — content-blind scheduling
  state; verdict-decoupled `NodeBlockSource` contract unchanged.
- **CI:** candidate `ci_check_live_feed_memory_bounds.sh` (additive); existing
  `ci_check_node_run_loop_containment.sh` + `ci_check_served_chain_handoff_fence.sh` **unchanged**.

## Forbidden during this cluster *(slice-level prohibitions inherit)*
- **No BLUE change.**
- No **semantic truncation** (an over-cap input is an error, never a truncated-but-accepted item).
- No **swallowing malformed / incomplete input as success** (over-cap reassembly → drop, never "no more
  data").
- No **unbounded or operator-disable option** — the bounds are closed constants; no CLI / env / config
  escape hatch.
- No **live-evidence / BA-02 / rehearsal claim**; no `RO-LIVE` flip.
- No **serve / forge / containment change** (`ci_check_node_run_loop_containment.sh` +
  `ci_check_served_chain_handoff_fence.sh` byte-unchanged).
- **Hard line:** if bounding the memory needs a BLUE change, a semantic truncation, a config escape
  hatch, or a containment relaxation — **stop and re-scope.**

## Replay obligations (scoped)
No new authoritative state, no new canonical type, no WAL/checkpoint change. The caps are pre-decode
scheduling / fail-closed bounds; replay is unaffected (the same captured feed replays identically; an
over-cap input deterministically fails closed both runs). Acceptance scoped to `ade_network` + `ade_node`
— not the full `ade_testkit` corpus lane.

## Registry impact (at close)
- `DC-LIVEMEM-01` (derived) — `declared` → **enforced** across S1 (rule count 313 → 314).
- Candidate gate `ci_check_live_feed_memory_bounds.sh` bound to `DC-LIVEMEM-01` (if added).
- **Not added here:** any `RO-LIVE` flip; any live-evidence / BA-02 / rehearsal rule; any new canonical
  type.

## Non-goals
No live evidence / BA-02 / operator pass / rehearsal (that is the operator-gated G-D rehearsal +
RO-LIVE-01/06). No serve / forge / containment change. No mainnet-complete DoS hardening beyond these two
named live-feed memory surfaces (other surfaces are separate future hardening slices). No grounding-doc
regeneration (that's `/cluster-close`).
