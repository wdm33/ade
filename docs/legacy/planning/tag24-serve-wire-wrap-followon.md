# Follow-on: serve-side tag-24 block-fetch wire-wrap (handoff seed)

> Seed for a NEW session to scope via `/invariants` → `/cluster-plan` →
> `/cluster-doc`. NOT yet scoped. Cluster ID TBD (next free `PHASE4-N-<letter>`;
> N-U is reserved for forged-block durability, so this is likely a distinct
> letter — let `/cluster-plan` assign it). Pick-up HEAD: `fea32c0` (pushed).

## Pick-up state
- HEAD `fea32c0`, pushed to `origin/main`. Tree clean. PHASE4-N-W closed
  (producer Praos VRF; CN-FORGE-04 enforced; in-process forge→self-accept
  ForgeSucceeded). All four grounding docs current at `fea32c0`;
  `head_deltas_baseline = 01e7e08`; registry 291 rules.
- **Read the four grounding docs end-to-end FIRST** (lesson:
  [[feedback-read-grounding-docs-first]]). The tag-24 item is named in
  `ade-SEAMS.md` §7 item 1 and in the registry as `CN-FORGE-03.open_obligation`.

## What this is (plain terms)
N-V/N-W made the node forge a Conway block its **own** validator accepts
(storage form `[era, block]`, round-trips through `decode_block`). But a **real
cardano-node peer** does not accept that raw storage form over block-fetch — the
Ouroboros block-fetch `MsgBlock` carries the block as **CBOR-in-CBOR**: the block
is serialised, then wrapped in a CBOR **tag 24** byte-string (and the chain-sync
header is wrapped similarly). Our serve path currently hands a peer the bare
storage bytes; it needs to emit the wire wrap.

`CN-FORGE-03.open_obligation` (verbatim): "Serve-side tag-24 wire-wrap
([serialisationInfo, tag24([era,block])]) so a live cardano-node peer accepts the
served block over block-fetch is a NAMED FOLLOW-ON, not N-V."

## Key leverage — the receive side already does the inverse
**PHASE4-N-M-FRAG** (closed, `4d3dc98`) shipped the **receive-side tag-24
unwrap** (incoming wire blocks are CBOR-in-CBOR; the session reducer unwraps
tag-24 before `decode_block`). See [[project-phase4-n-m-frag-closed]]. So the
codebase already models the wire⇄storage boundary on the way IN — this follow-on
is the **encode/serve inverse** of an existing decode path. Grep starting points:
`crates/ade_codec/src/cbor/mod.rs` (tag(24) = bytes `0xd8 0x18`, ~line 685);
the N-M-FRAG receive unwrap; `block_fetch` real-capture corpus tests.

## Authority surface (verify against the tree)
- `ade_network::block_fetch::server::producer_block_fetch_serve` (BLUE,
  `server.rs:134`) — currently serves `AcceptedBlock.as_bytes()` raw (pinned by
  test `producer_block_fetch_serve_block_bytes_equal_accepted_block_as_bytes`).
  This is where (or just below where) the wire-wrap must be applied — or, more
  IDD-correct, a single BLUE codec chokepoint owns the wrap and the server calls
  it.
- `ade_codec::cbor::envelope::{encode,decode}_block_envelope` (`envelope.rs:105/35`)
  — the storage-form pair (N-V). The wire wrap is a layer ABOVE this; consider a
  symmetric `encode_block_wire`/`decode_block_wire` (tag-24) pair so serve and
  receive share one wrap authority (mirror the N-V single-encoder discipline +
  the N-W single-VRF-authority discipline).
- Chain-sync header serve (`chain_sync::server`) likely needs the analogous
  header wrap — confirm whether block-fetch alone is in scope or headers too.

## Proof obligations the new session must nail (entry, not footnotes — [[feedback-proof-discipline]])
1. **Exact wire shape, verified against a real peer**, not assumed. Confirm the
   block-fetch `MsgBlock` CDDL (tag-24 byte-string of the serialised block; is
   the outer `[serialisationInfo, …]` actually present or is that storage-DB
   framing, not wire framing?). Check the receive-side unwrap (N-M-FRAG) for the
   precise shape we already accept, and the `block_fetch` real-capture corpus
   from the docker preprod node.
2. **Wrap⇄unwrap symmetry (in-process):** serve-wrap then our own receive-unwrap
   must round-trip byte-identically to the storage form (a BLUE round-trip test,
   like N-V's `ci_check_forge_decode_round_trip`).
3. **Single wrap authority** (no second wrapper; no both-forms fallback) — same
   discipline as CN-FORGE-03 (single encoder) and CN-FORGE-04 (single VRF auth).
4. **Live acceptance is a SEPARATE leg.** The in-process symmetry is provable
   here; a REAL cardano-node accepting the served block over block-fetch needs an
   operator live-pass (RO-LIVE-01 / CN-CONS-06 territory,
   `blocked_until_operator_pass_executed`) against the docker preprod peer
   ([[reference-local-preprod-docker-cardano-node]]). Do NOT conflate "wrap is
   byte-correct" with "live peer accepted" — split the evidence along the layer
   boundary ([[feedback-shell-must-not-overstate-semantic-truth]]).

## Likely registry shape
Strengthen `CN-FORGE-03` (its open_obligation is exactly this) and/or add a new
`CN-FORGE-0x`/`CN-WIRE-*` rule for the wrap authority. No new rule invented here —
that's the `/invariants` step's call.

## Suggested opener for the new session
> Pick up at HEAD `fea32c0`. Read `docs/planning/tag24-serve-wire-wrap-followon.md`
> and the four grounding docs end-to-end first. Scope the serve-side tag-24
> block-fetch wire-wrap via `/invariants` (it's CN-FORGE-03's open_obligation;
> the receive side already unwraps tag-24 per N-M-FRAG, so this is the
> encode/serve inverse). Confirm the exact wire shape against the receive-side
> unwrap + the real-capture corpus before sketching invariants. Keep the live
> peer-acceptance leg explicitly separate from the in-process wrap-symmetry proof.
