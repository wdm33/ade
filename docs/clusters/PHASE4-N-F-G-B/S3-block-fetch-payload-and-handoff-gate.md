# PHASE4-N-F-G-B — Slice S3: Block-fetch payload proof + served-chain handoff CI gate

> **Status:** slice doc (IDD Part IV). Companion to `cluster.md` (S3 row + CE-G-B-3; closes CE-G-B-2).
> Code-verified against HEAD `a5af52e8` (S1 + S2 merged).
>
> **Slice S3 in one line:** serve the **S2-admitted** self-accepted block via the existing
> `producer_block_fetch_serve` over `ServedChainLookups{ S2 ServedChainView }`, prove (hermetic
> loopback) that the served `MsgBlock` payload is the self-accept input bytes under the single
> CN-WIRE-08 tag-24 envelope, and add an additive **served-chain handoff fence** CI gate. **No real
> peer / socket-accept (G-C); no live feed / WirePump / dialer; no containment relaxation.**

## 1. Slice identity
- **Cluster:** PHASE4-N-F-G-B. S1 = fence (merged); S2 = admit half (merged); **S3 = block-fetch
  serve payload proof + handoff gate** (closes the cluster's serve authority).
- **Modules:** **RED** node-spine serve wiring + S3 tests (`node_lifecycle`); **reused deterministic
  serve machinery (consumed, not modified):** `ade_network::block_fetch::server::
  producer_block_fetch_serve` + `ade_runtime::producer::served_chain_lookups::ServedChainLookups` +
  the BLUE `ade_codec::cbor::tag24` envelope; **CI** new `ci/ci_check_served_chain_handoff_fence.sh`.
  **No BLUE change; no new `CoordinatorEvent` variant.**

## 2. Cluster Exit Criteria addressed (verbatim)
- **CE-G-B-3 (block-fetch payload/envelope + handoff gate)** — candidate tests
  `block_fetch_payload_is_self_accepted_bytes`, `block_fetch_tag24_round_trips_to_self_accept_input`;
  candidate gate `ci_check_served_chain_handoff_fence.sh` (bans raw-bytes / failed-outcome / flag /
  verdict serve-ingress); existing `ci_check_node_run_loop_containment.sh` still unchanged.
  *(`DC-NODE-06` flips declared→enforced when CE-G-B-1..3 are all green.)*
- **CE-G-B-2 (sibling serve task; containment unchanged)** — *closed here*: the S2 admit sibling now
  **serves** the admitted block via `push_atomic`-only mutation; relay-loop containment
  semantically unchanged. (S2 contributed the admit half; S3 completes the serve.)

(CE-G-B-1 = S1 fence, merged.)

## 3. Intent (invariant impact)
Make **"a block served from the `--mode node` served chain is anything other than the BLUE
self-accepted forged block, byte-identically, under the single tag-24 envelope"** unrepresentable.
The serve path reads ONLY the S2 `ServedChainView` (populated solely via `push_atomic ←
into_accepted()`), and the wire payload is `producer_block_fetch_serve`'s tag-24 wrap of the
`AcceptedBlock` bytes — so the served bytes provably trace to `self_accept` (CN-FORGE-01), and the
serve ingress cannot be fed raw bytes, a failed forge outcome, a self-declared flag, or a peer
verdict (the new gate).

## 4. Pre-conditions (verified at HEAD `a5af52e8`)
- **S2 served chain:** the dispatcher's sibling task holds `(ServedChainHandle, _serve_view)` and
  admits via `push_atomic(handoff.into_accepted())`; `_serve_view` is the S3 serve reader.
- **The serve reducer (reused, deterministic):** `producer_block_fetch_serve(state, RequestRange,
  &dyn ServedRangeLookup, version) -> Replies(MsgBlock…)` (`block_fetch/server.rs:136`); the served
  payload is `tag24(bytes([era,block]))` — already proven by
  `producer_block_fetch_serve_block_payload_is_tag24_wrapped_and_decodes` +
  `..._block_bytes_equal_accepted_block_as_bytes` + `..._replays_byte_identical_over_corpus`.
- **The production lookup adapter (reused):** `ServedChainLookups{ snap: &ServedChainSnapshot }`
  (`served_chain_lookups.rs:81`) impls `ServedRangeLookup` — the same adapter produce_mode serves
  over (produce_mode.rs:~1395-1420).
- **The single envelope authority:** `ade_codec::cbor::tag24` / `decompose_blockfetch_block`
  (CN-WIRE-08); Origin endpoints → `NoBlocks` (no genesis serve).
- **The containment gate:** `ci_check_node_run_loop_containment.sh` — must stay green + script
  byte-unchanged (S3 touches the dispatcher's sibling task, never the `run_relay_loop` body).

## 5. The fix (serve the S2 view; prove the payload; add the fence gate)
1. **In-process node-spine block-fetch serve (RED wiring over a consumed reducer):** the dispatcher's
   sibling task exposes the S2 `ServedChainView` to `producer_block_fetch_serve` via
   `ServedChainLookups{ &*view.borrow() }` — the same reducer + adapter + tag-24 authority
   produce_mode serves over. A served `RequestRange` yields the admitted block as a tag-24
   `MsgBlock`; an absent range → `NoBlocks` (no fabrication).
2. **Hermetic loopback proof:** admit a self-accepted block via the S2 path (`push_atomic(handoff.
   into_accepted())`), then drive `producer_block_fetch_serve(RequestRange{that (slot,hash)})` over
   `ServedChainLookups{view}`; assert the `MsgBlock` payload starts with tag-24 (`0xd8 0x18`) and
   `decompose_blockfetch_block(payload)` == the self-accept input bytes byte-for-byte.
3. **Served-chain handoff fence gate (CI, additive):** `ci/ci_check_served_chain_handoff_fence.sh`
   greps the node-spine serve path and proves serve admission is fed ONLY by `into_accepted()` (the
   sole `push_atomic` site) — banning raw forge bytes, a failed forge outcome, a self-declared flag,
   and a peer-verdict substitute from the serve ingress. Additive (a NEW gate); the containment gate
   is untouched.

## 6. TCB color (execution boundary)
- **RED (new wiring):** the in-process node-spine block-fetch serve path over the S2 view + the S3
  tests (`node_lifecycle`).
- **GREEN/BLUE consumed reducer, RED wiring:** `producer_block_fetch_serve` and `ServedChainLookups`
  are reused deterministic serve machinery; S3 only wires/tests them over the node-spine
  `ServedChainView` (it does not re-implement or extend the reducer).
- **RED (reuse, no change):** `ServedChainHandle::push_atomic` (the S2 mutation authority; S3 only
  *reads* the view).
- **BLUE (consume only):** `ade_codec::cbor::tag24` (envelope), `ade_ledger::producer::{served_chain,
  AcceptedBlock}`. **No BLUE change.**
- **CI:** new `ci_check_served_chain_handoff_fence.sh` (additive); existing
  `ci_check_node_run_loop_containment.sh` / `ci_check_no_independent_forge_codepath.sh` unchanged.

## 7. Invariants preserved (must not weaken) — by registry ID
- `CN-WIRE-08` — single tag-24 wire-envelope authority; S3 serves through it (no parallel serializer).
- `CN-CONS-07` / `CN-FORGE-01` — only `AcceptedBlock` (from `self_accept`) enters the served chain;
  the serve reads it back, adding no second producer / raw-bytes path.
- `CN-NODE-02` / `DC-SYNC-02` / `DC-NODE-05` — `run_relay_loop` body unchanged; the durable tip
  advances only via `run_node_sync`; serve happens in the dispatcher's sibling task.
- `CN-PROD-04` — `push_atomic` stays the single served-chain mutation authority (S3 only *reads* the
  view).
- The closed `CoordinatorEvent` surface — no new variant/field.

## 8. Invariants strengthened (one family: node-spine serve payload is self-accepted bytes, fenced)
**Family:** *a block served from the `--mode node` served chain is byte-identically the BLUE
self-accepted forged block under the single CN-WIRE-08 tag-24 envelope, and the serve ingress is
fed only by the S2 handoff's `into_accepted()` — never raw bytes, a failed outcome, a flag, or a
peer verdict.*
- `DC-NODE-06` — S3 wires the **block-fetch payload + serve-ingress-fence** clauses and adds the
  binding gate `ci_check_served_chain_handoff_fence.sh`. **At G-B close** (CE-G-B-1..3 all green),
  `DC-NODE-06` flips `declared`→`enforced` (the registry status flip + `CN-PROD-04`/`CN-CONS-07`
  `strengthened_in += "PHASE4-N-F-G-B"` are recorded at `/cluster-close`, not in this slice).

## 9. Slice-entry decisions (settled)
- **D-1 — serve mechanism (DECIDED: reuse `producer_block_fetch_serve` over `ServedChainLookups{ S2
  view }`).** The serve reducer + lookup adapter + tag-24 authority already exist and are proven;
  S3 wires them over the node-spine view. No new serializer (CN-WIRE-08), no second serve authority.
- **D-2 — NO real n2n_listener socket / peer connection in S3 (DECIDED: hermetic loopback).** S3
  proves the serve via an in-process loopback (`producer_block_fetch_serve` driven directly over the
  node-spine `ServedChainView`). Binding a real listener socket + accepting a real peer's fetch is
  **G-C** (operator pass) — it touches "live feed / peer acceptance / RO-LIVE", explicitly out of
  bounds here. **Rejected:** mirroring produce_mode's full `n2n_listener` + per-peer outbound serve
  loop into the node binary (that is the G-C live wiring).
- **D-3 — serve ingress fence (DECIDED: gate the node-spine serve path).** The gate proves the only
  feed to the served chain is `into_accepted()` (S2's `push_atomic` site) and the serve reads only
  the `ServedChainView` — no raw-bytes/failed/flag/verdict ingress.

## 10. Replay / determinism obligations
`producer_block_fetch_serve` is a pure reducer (deterministic in `(snapshot, RequestRange,
version)`); `..._replays_byte_identical_over_corpus` holds. Same admitted block + same range ⇒
byte-identical served `MsgBlock`. No new canonical type (the payload is the existing tag-24 wrap of
the canonical block), no WAL/checkpoint change, no new corpus entry.

## 11. Replay / crash / epoch validation (tests by name)
- **New:**
  - `block_fetch_payload_is_self_accepted_bytes` — admit a self-accepted block via the S2 path, serve
    its `(slot,hash)` range over the node-spine `ServedChainView`, assert
    `decompose_blockfetch_block(MsgBlock.bytes)` == the self-accept input bytes.
  - `block_fetch_tag24_round_trips_to_self_accept_input` — the served payload starts with tag-24
    (`0xd8 0x18`) and its inner bytes decode via the canonical block-envelope authority back to the
    self-accept input (wrap↔decode symmetry on the node spine).
- **Preserved:** the S2 admit tests; `relay_loop_containment_semantics_unchanged_with_serve_sibling`;
  the existing `producer_block_fetch_serve_*` payload/replay tests; full ade_node suite.
- **CI:** `ci_check_served_chain_handoff_fence.sh` (new, additive) green;
  `ci_check_node_run_loop_containment.sh` green + script byte-unchanged.

## 12. Mechanical acceptance criteria
- [ ] `cargo test -p ade_node` (+ touched `ade_runtime`/`ade_network`) — the two S3 tests green; the
      preserved tests still green.
- [ ] `ci_check_served_chain_handoff_fence.sh` green (the new fence gate).
- [ ] `ci_check_node_run_loop_containment.sh` green + **script byte-unchanged**;
      `ci_check_no_independent_forge_codepath.sh` green.
- [ ] `grep` proof: the node-spine serve reads only the `ServedChainView` and serves via
      `producer_block_fetch_serve` (no parallel serializer); the only served-chain feed is
      `into_accepted()`.
- [ ] `cargo build` + `cargo clippy` clean on touched crates; `rustfmt` on changed files only.
- [ ] No new `CoordinatorEvent` variant/field; no real `n2n_listener` socket / peer-accept wired
      into the node binary (diff inspection — that is G-C).
- [ ] No `DC-NODE-06` registry status flip in this slice (that is `/cluster-close`).

## 13. Failure modes
All **fail-closed**: an absent/Origin range → `NoBlocks` (no fabricated/zeroed block); a serve over an
empty view → `NoBlocks`; the served payload is always the tag-24 wrap (the bare `[era,block]` is
never placed on the wire). No panic (pure reducer; `Result` surfaced).

## 14. Hard prohibitions (inherits the cluster list)
- **No real `n2n_listener` socket / peer connection / outbound dispatch wired into the node binary**
  (G-C). **No live feed / `WirePump` / `n2n_dialer`.**
- **No peer-acceptance / BA-02 / RO-LIVE claim** — S3's proof is hermetic loopback; serving ≠ a peer
  acceptance verdict; peer acceptance is G-C and must be proven only from peer logs through
  `ba02_evidence::correlate`.
- **No raw-byte serve ingress** — the serve reads only the `ServedChainView`; the only feed is
  `into_accepted()`.
- **No relay-loop containment relaxation** (`ci_check_node_run_loop_containment.sh` script
  byte-unchanged); no serve/tip token in the `run_relay_loop` body.
- **No second serve authority / parallel serializer** (only `producer_block_fetch_serve` +
  CN-WIRE-08 tag-24).
- **No new `CoordinatorEvent` variant/field; no BLUE authority / canonical type / WAL.**
- **Hard line:** if the serve needs a BLUE change, a real peer/socket, a second serializer, or a
  containment relaxation — **stop and re-scope.**

## 15. Explicit non-goals
- No real network peer / operator pass / socket-accept (G-C). No live feed / dialer.
- No `DC-NODE-06` registry flip (that is `/cluster-close`, when CE-G-B-1..3 are all green).
- No grounding-doc regeneration (that is `/cluster-close`).

## 16. Completion checklist
- [ ] In-process node-spine block-fetch serve path exercised over the S2 `ServedChainView` via
      `ServedChainLookups` + `producer_block_fetch_serve`; hermetic loopback proves the payload ==
      self-accept input under tag-24; `ci_check_served_chain_handoff_fence.sh` added + green.
- [ ] All §12 tests/gates green; containment gate green & script-unchanged; clippy clean; changed
      files rustfmt'd.
- [ ] Slice doc committed standalone (`docs:`) before implementation; impl committed after green,
      model-attribution trailer. **No registry edit** (DC-NODE-06 flip at G-B close).

## Authority
Registry IDs `DC-NODE-06` (wires the payload + serve-ingress-fence clauses + the binding gate;
registry flip **at G-B close**); `CN-WIRE-08` / `CN-CONS-07` / `CN-FORGE-01` / `CN-NODE-02` /
`DC-SYNC-02` / `DC-NODE-05` / `CN-PROD-04` (preserved). The cluster doc + invariant registry are
authoritative; this slice doc refines, it does not override.
