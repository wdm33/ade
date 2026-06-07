# Invariant sketch — Single-producer SUSTAINED forge on the relay spine

> IDD Part I artifact (invariants phase). Pre-cluster. No clusters/slices/implementation here.
> Origin: rung-1 Finding B (2026-06-07; see `project_rung1_kickoff` memory + `~/.cardano-rung1-host/` evidence).
> Sibling guides: `docs/active/c2-preprod-tip-guide.md` §7b (the §7b ladder; Finding A recorded there as a venue constraint).

**Concept (one line):** A recovered/following `--mode node` Ade, sole producer behind a frozen
non-producing Haskell relay, must forge a **chain** of successors (N, N+1, N+2, …) — each adopted
by the relay — by recognizing catch-up to its **own** block once the relay adopts and re-announces
it. Today it stalls at exactly **one** block: after forging block N (relay adopts, pool1-correlated,
0 CandidateTooSparse) Ade emits `no_tip_available` (NotCaughtUp) indefinitely and never forges N+1;
the follow link then EOFs (`unknown_disconnected`).

**Pure-transformation framing:** the authoritative core (block admission via `pump_block`,
forge-successor position) is **already** a pure transformation and is **unchanged**. The gap is
entirely in the RED scheduling shell — the *observation* of the peer's advertised tip. The concept
is expressible with **no new authoritative nondeterminism**: the fix advances a RED admissibility
signal, never the durable/replay surface.

## 1. What must always be true
- **SF-1 (the gap):** `followed_peer_tip` MUST advance **only from a real observed peer ChainSync
  advertisement of the peer's selected tip**, **including the self-adoption echo** case where the
  advertised block is already durably held by Ade (the relay re-announcing Ade's own just-adopted
  block). The advance is an observation of the peer's *real* selection — never a locally-inferred
  catch-up.
- **SF-2 (preserve DC-NODE-15):** forge admissibility stays exactly `durable_servable_tip ==
  followed_peer_tip` (hash **and** block_no). SF-1 makes the signal *reach N truthfully*; it must not
  weaken the gate or fabricate catch-up.
- **SF-3 (preserve the admissibility-only boundary):** `followed_peer_tip` remains a forge-admissibility
  input **only** — may PREVENT a forge, never select/replace/reorder/prefer chains; never reaches
  `select_best_chain`/`chain_selector`/`fork_choice` (DC-CONS-03 stays the follow authority).
- **SF-4 (preserve DC-NODE-16):** `pump_block` stays an idempotent no-op on a re-announced already-have
  block (no reducer, no WAL, no durable-tip change). SF-1 advances the RED *scheduling* signal,
  **orthogonal** to the durable tip.
- **SF-5 (preserve DC-CONS-24 / DC-NODE-10, extended to the chain):** each forged successor builds on
  the followed tip — `prev_hash = Block(tip.hash)`, `block_no = tip.block_no + 1`, position from the
  evolved admitted spine — now **sustained across the whole chain**, not just block N.
- **SF-6 (follow-link liveness — open proof obligation, NOT a declared rule yet):** the follow link
  MUST persist (or deterministically re-establish) across the inter-forge idle interval, so Ade keeps
  observing the relay's tip instead of EOFing into `unknown_disconnected`. Held here pending OQ-2 (it
  may be a *consequence* of the SF-1 stall, not an independent gap); do not bundle into DC-NODE-17.

## 2. What must never be possible
- Forging N+1 while **not genuinely** caught up to the peer's real tip (no fabricated/locally-inferred
  catch-up; the signal must reflect a real peer advertisement).
- The followed-tip advance mutating **any** authoritative/durable state (ledger, chain_dep, WAL,
  ChainDb tip).
- The followed-tip signal influencing chain **selection** (admissibility-only).
- Re-validating / re-WAL'ing a block Ade already durably holds (DC-NODE-16 idempotency intact).

## 3. What must remain identical across executions (deterministic surface)
- The durable post-state after admitting a chain of K (own-forged + followed) blocks — ledger
  fingerprint, chain_dep, ChainDb tip, WAL contents. **Unchanged** by this concept; `followed_peer_tip`
  is *not* part of the deterministic surface.
- Forge-successor position derivation (block_no, prev_hash) from the evolved spine.

## 4. What must be replay-equivalent
- Same recovered checkpoint + same WAL (ordered own-forged + followed `AdmitBlock`s) → byte-identical
  post-state **and** served chain (T-REC-05 / DC-WAL-02 / DC-WAL-04). The SF-1 advance MUST append
  nothing to the WAL and change no ledger state — **replay-neutral**, exactly like DC-NODE-16. A K>1
  forged chain must warm-start-replay byte-identically (extends N-U S2's single-block proof — OQ-3).

## 5. State transitions in scope (RED scheduling; the BLUE reducer is unchanged)
- `(followed_peer_tip, real peer ChainSync advertisement of selected tip=N — even when the carried
  block is durably held by Ade) → Ok(followed_peer_tip := N)` — **SF-1**. (Today only an
  `AdmissionPeerEvent::TipUpdate` observes via `FollowedPeerTipSignal.observe`; the self-echo arrives
  as a `Block` → signal stuck < N.)
- `(durable_servable_tip = N, followed_peer_tip = N) → CaughtUp → forge N+1` — existing DC-NODE-15;
  just needs the signal to truthfully reach N.
- `(re-announced already-have block, pump_block) → Ok(None)` — existing DC-NODE-16; preserved.
- `(follow link, inter-forge idle) → link live (or deterministic re-establish)` — **SF-6**; today EOFs.

## 6. TCB color hypothesis
- **RED (the fix):** `followed_peer_tip` observation in `ade_node::node_sync` (FollowedPeerTipSignal /
  the wire pump) — advance from a real peer advertisement even when the carried block is a durable
  duplicate. Mirrors AE.F's RED-only shape: **no BLUE change**.
- **RED:** follow-link liveness (wire-pump dial/keep-alive in `ade_node`/`ade_runtime`).
- **BLUE (unchanged):** chain-sync server (DC-PROTO-10), header validity, the ledger reducer,
  `pump_block`'s BLUE chokepoint. Add **no** BLUE type/authority.
- **Open color:** whether the tip-observation refinement sits in the GREEN-by-function classifier vs
  the RED pump — `node_sync` hosts both; resolve at cluster-doc.

## 7. Open questions (slice-entry proof obligations — the live run is the arbiter)
- **OQ-1 (THE blocker; resolve FIRST, live-instrumented):** when the relay adopts block N and rolls it
  forward to Ade, does Ade's wire pump actually receive a ChainSync advertisement carrying the relay's
  selected tip = N (→ SF-1 is "observe that tip even on the duplicate")? **Or** does the relay not
  re-serve Ade's own block at all (connection idle) — so Ade can't learn N from the follow, needing a
  different mechanism? Different root causes → different fixes. Confirm by instrumenting Ade's
  `AdmissionPeerEvent` stream (or the relay's chain-sync trace). **Do not design on the hermetic
  hypothesis (the AE.B→AE.E lesson).**
- **OQ-2:** is the follow-link EOF (SF-6) a *consequence* of the stall (Ade goes quiet → relay
  idle-timeout) — so fixing SF-1 resolves it — or an independent keep-alive/reconnect gap? Only if
  independent does a later DC-NODE-18 / operational follow-link-liveness rule get declared.
- **OQ-3 (replay):** does T-REC-05/DC-WAL-04 warm-start-replay already cover a **chain** of K
  own-forged blocks byte-identically, or only the single-block case (N-U S2)?
- **OQ-4 (out of scope — Finding A):** the forecast-horizon (3k/f ≈ 300 slots) is the *peer's*
  constraint, not an Ade core invariant. The loop stays healthy while the relay keeps adopting (its tip
  tracks Ade); only a leadership dry-spell > horizon pushes the next forge beyond a frozen peer's
  horizon. **Out of scope** for this slice; recorded in c2-guide §7b as a venue/liveness constraint.
- **OQ-5:** does the gate treat an own-forged-then-adopted followed tip identically to a relay-origin
  followed tip (vs. the AE.A "recovered anchor is never a forge base" rule)?

## Proposed registry entry — DC-NODE-17 (declared-only; appended to `docs/ade-invariant-registry.toml`)

`tier = derived`, `status = declared` (NOT enforced — blocked on OQ-1). Narrow, RED-only. The sibling
follow-link liveness obligation (SF-6/OQ-2) is **not** a declared rule yet.

```toml
[[rules]]
id = "DC-NODE-17"
tier = "derived"
statement = "followed_peer_tip advances ONLY from a real observed peer ChainSync advertisement of the peer's selected tip, INCLUDING the self-adoption echo case where the advertised block is already durably held by Ade … RED admissibility-only; never durable/WAL/ledger; never fork-choice."
status = "declared"
introduced_in = "TBD"
```

### Enforcement preconditions (ALL required before DC-NODE-17 → enforced)
DC-NODE-17 is **not a chain-selection rule** — it is a RED observation rule for forge admissibility
only. The slice that implements it must prove, with mechanical gates + a committed live transcript:
1. **no WAL append** on the advance;
2. **no durable ChainDb tip mutation**;
3. **no ledger / chain_dep mutation**;
4. **no fork-choice / chain-selection influence** (never reaches `select_best_chain` /
   `chain_selector` / `fork_choice`);
5. **no bypass** of the DC-NODE-15 gate `durable_servable_tip == followed_peer_tip`;
6. **committed live evidence** that the peer actually re-announces or otherwise advertises Ade's
   adopted block (the basis for the advance — OQ-1).

## Out of scope (recorded elsewhere)
- **Finding A — forecast-horizon adoption window:** a venue/peer constraint, in `c2-guide` §7b.
- **Follow-link liveness (SF-6/OQ-2):** held above as an open proof obligation; a rule only if OQ-2
  proves it independent.
- **Multi-producer fork-choice (rung 2), preprod (rung 3):** not climbed until single-producer
  sustained works (§7b ladder discipline).
