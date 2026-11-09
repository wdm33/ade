# Follow-on: operator-pass live leg on a C1 private testnet (handoff seed)

> Seed for a NEW session. **This is a SCOPING pass first**, not the live run:
> produce an honest "mechanically-ready vs needs-operator-setup" inventory + a
> concrete C1 (private testnet) recipe, THEN let `/invariants` / `/cluster-plan`
> decide whether a thin wiring cluster is owed before execution. Pick-up HEAD:
> `86ddc4d` (pushed). Tree clean.

## What this is (plain terms)

We have built a node that forges a Conway block its **own** validator accepts, signs
it with real KES over the real header pre-image, and now serves it in the **exact
tag-24 wire shape** a real cardano-node expects (PHASE4-N-X). The one thing never
proven end-to-end: **a real Haskell cardano-node peer fetching an Ade-forged block
and accepting it** as a valid block from a legitimately-elected leader. That is the
bounty's block-production deliverable (`CN-CONS-06` / `RO-LIVE-01`).

It has never been done because it needs Ade to **be an elected slot leader**, which
needs real operator keys + stake. The **C1 path** removes the stake prerequisite:
stand up a **private testnet whose genesis gives Ade's pool ~all the stake**, so Ade
is elected (almost) every slot and a private Haskell peer pulls Ade's chain.

## Pick-up state — what landed since the live leg was last blocked

When `CN-CONS-06.open_obligation` was written (PHASE4-N-Q), the live half had **two**
named blockers. Re-check both — the first is very likely now CLOSED:

1. `blocked_until_real_forge_handler_lands` — N-Q's `produce_mode` had a **stub** forge
   handler. Since then the real composition shipped: **N-R-A** `run_real_forge` (BLUE
   leader-check) + **N-S-A** KES signs the real `unsigned_header_pre_image` + **N-S-B**
   typed `OutboundCommand` relay through MuxPump + **N-W** producer Praos VRF authority
   (`CN-FORGE-04`) + **N-X** the serve-side tag-24 wire-wrap (`CN-WIRE-08`). **The
   scoping pass must confirm `produce_mode::run_real_forge` is the live path (no stub)
   and that the per-peer block-fetch dispatch is wired** (N-R-B).
2. `blocked_until_operator_stake_available` — this is what C1 solves (Ade owns the
   genesis stake). Still real; it's the operator-setup half this pass scopes.

## Authority surface to read end-to-end FIRST (lesson: [[feedback-read-grounding-docs-first]])

- The four grounding docs at `86ddc4d` (CODEMAP/SEAMS/HEAD_DELTAS/TRACEABILITY).
- Registry rules: **`RO-LIVE-01`** (status `partial` — peer RequestRange over an
  Ade-forged block passes the peer's full header+body validation), **`CN-CONS-06`**
  (status `enforced` mechanical half; live half open — read its long `open_obligation`),
  **`CN-OPERATOR-EVIDENCE-01`** (the committed evidence-manifest TOML schema — the
  contract for what a pass must produce), `CN-PROD-01/02`, `CN-FORGE-01..04`,
  `CN-WIRE-08`, `DC-CONS-16/17/18`.
- Code: `crates/ade_node/src/produce_mode.rs` (the `--mode produce` driver +
  `run_real_forge`), `crates/ade_node/src/cli.rs` (produce-mode flags, below),
  `crates/ade_runtime/src/producer/{coordinator,producer_shell,keys,opcert_envelope,
  genesis_parser,served_chain_handle}.rs`, `crates/ade_runtime/src/network/{n2n_listener,
  mux_pump,n2n_server,outbound_command}.rs`, the BLUE serve reducers
  `ade_network::{block_fetch,chain_sync}::server`.
- Operator procedures already written (READ THESE — they are 80% of the runbook):
  `docs/clusters/completed/PHASE4-N-Q/CE-N-Q-OPERATOR_PROCEDURE.md`,
  `docs/clusters/completed/PHASE4-N-G/CE-N-G-8_PROCEDURE.md`, and the
  `docs/clusters/completed/PHASE4-N-S-C/{cluster,S1,S2}.md` runbook + manifest schema.

## How `ade_node --mode produce` is invoked (from cli.rs)

Required flags: `--listen ADDR`, `--cold-skey`, `--kes-skey`, `--vrf-skey`, `--opcert`,
`--genesis-file`; plus `--peer ADDR` (repeatable) and `--snapshot-store PATH`. Ade is
**server-role**: it opens a TCP listener; the Haskell peer dials in, chain-syncs Ade's
headers, and block-fetches Ade's forged blocks. (Confirm direction in the scoping pass —
N-Q's listener has the peer dial Ade.)

## The scoping pass must produce

1. **Mechanically-ready inventory** — for the full forge→serve→peer-accept path, what is
   wired at `86ddc4d` (real forge, KES/VRF/opcert/genesis parsers, served-chain
   push_atomic, per-peer dispatch, tag-24 wrap on BOTH chain-sync header + block-fetch
   block) vs. what is still stub/missing. Cite file:line.
2. **C1 private-testnet recipe** — concrete: a Shelley+Conway genesis where Ade's pool
   holds ~all active stake (guaranteed leadership per active-slots-coeff); cardano-cli
   generation of cold/opcert/KES/VRF keys consumable by Ade's parsers (note: N-O/N-P —
   Ade imports cardano-cli's 608-byte expanded `Sum6KES` skey); a private Haskell
   cardano-node 11.0.1 configured to peer FROM Ade and pull its chain; how the peer's
   acceptance is observed (peer logs → `acceptance_keyword_match`).
3. **Evidence contract** — what a committed pass must contain per `CN-OPERATOR-EVIDENCE-01`
   (manifest TOML + raw peer log + sha256). Decide the evidence path/filename convention
   for the C1 venue (analogous to `CE-N-S-LIVE_YYYYMMDD-<commit>.toml`).
4. **Gap → plan** — does the live leg need a thin **wiring cluster** first (e.g. a
   produce-mode driver flag, a private-genesis loader path, an evidence-emitter), or is
   it pure operator execution against today's binary? If a cluster is owed, name its
   invariants; if not, write the runbook and flip the relevant rules to
   `blocked_until_operator_pass_executed` with the C1 procedure committed.

## Hard cautions (carry into the pass)

- **Scope is the SCOPING pass.** Do not attempt the full live run until the inventory +
  recipe exist and the user green-lights execution.
- **Live acceptance is only proven by committed operator evidence** — wire success ≠
  admission ≠ peer-accept ([[feedback-shell-must-not-overstate-semantic-truth]]). A peer
  merely *connecting* or *fetching* is not *accepting*; the proof is the peer's own
  validation log (`acceptance_keyword_match`) over the manifest schema.
- **C1 is a real Cardano private testnet, not a mock.** The Haskell peer runs real
  ledger+consensus validation; Ade must be a genuinely-elected leader under the shared
  genesis/VRF, not a spoofed one. C1 only removes the *stake-provisioning* prerequisite,
  not the *validity* requirement.
- **Tip-following / a peer fetching bytes is a bounded smoke-test, never the deliverable**
  ([[feedback-bounded-smoke-slices]]). The deliverable is peer-accepts-forged-block.
- Don't commit private keys / genesis secrets to the repo ([[feedback-no-credential-leaks]]).

## Suggested opener for the new session

> Pick up at HEAD `86ddc4d`. Read `docs/planning/operator-pass-live-leg-c1-followon.md`
> and the four grounding docs end-to-end first. Do the SCOPING pass for the operator-pass
> live leg (`CN-CONS-06` / `RO-LIVE-01`) on a **C1 private testnet**: (1) confirm whether
> `blocked_until_real_forge_handler_lands` is now closed by N-R/N-S/N-W/N-X and inventory
> what's mechanically ready in `produce_mode` vs needs operator setup, file:line; (2)
> write the concrete C1 recipe (private genesis with Ade-controlled stake, cardano-cli
> key generation, a private Haskell peer pulling Ade's chain); (3) map the evidence
> contract to `CN-OPERATOR-EVIDENCE-01`; (4) decide whether a thin wiring cluster is owed
> before execution. Keep it a scoping pass — do not run the live nodes until I green-light.
