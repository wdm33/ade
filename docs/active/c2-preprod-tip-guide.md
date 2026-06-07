# C2 — the preprod live-tip accepted-block path (Conway-from-Mithril, NOT from genesis)

> **Read this before scoping, planning, building a venue for, or explaining any C2
> work.** It exists because a specific wrong turn keeps recurring and burned a whole
> session: trying to manufacture a **local Conway net from genesis** so Ade can forge
> block 0, instead of **recovering an existing non-Origin Conway tip** the way a real
> node does. The correct C2 path is the same recover → forge tip+1 → Haskell-peer-adopt
> loop on two venues, **in order**: a **required private-testnet rehearsal first** (an
> already-bootstrapped private Conway chain, Ade stake active, 2 Haskell nodes), **then
> preprod-live**. This guide locks that so we build on it without regression. It is the
> C2 sibling of `docs/active/live-pass-path-fidelity-guide.md` (read that too — same
> anti-drift purpose, complementary rule).

> **MILESTONE (2026-06-07): the C2-LOCAL adoption manifest (CE-A5) is PROVEN** — a real
> cardano-node 11.0.1 relay committed an Ade-forged block (§5b; PHASE4-N-AE.A/B/C/E; the
> closer was AE.E's chain-sync-server cursor fix, DC-PROTO-10; post-adoption echo fixed by
> AE.F, DC-NODE-16). It is a **non-promotable private-venue rehearsal** — it strengthened but
> did NOT flip CN-CONS-06 / RO-LIVE-01. Next work is the **§7b robustness ladder** (rung 1
> sustained + epoch on C2-LOCAL → rung 2 multi-producer fork-choice → rung 3 preprod). Status
> detail: §5b + §7 + §7b.

---

## 0. The one rule

**C2 = recover an EXISTING non-Origin Conway tip, forge the successor on it, and have a
real Haskell peer adopt it.** The tip already exists on a real, **already-bootstrapped**
Conway chain; Ade **restores from a snapshot/extraction at that tip** (Mithril-verified
UTxO + cardano-cli extraction for consensus inputs) exactly as a real node fast-syncs.
**A node does NOT replay from slot 0, and Ade NEVER forges block 0.**

The shared shape:

    recover non-Origin Conway tip  (extract + import_live_consensus_inputs + seed_to_snapshot @ tip)
      → forge block tip+1 → self-accept → durable admit → serve
      → real Haskell peer adopts → peer log → correlate → evidence

It runs on TWO venues, **in this order**:

- **C2-LOCAL-REHEARSAL — REQUIRED, FIRST (§5b).** An already-past-bootstrap **private**
  Conway chain: a non-Origin tip, Ade's pool **active-staked**, **2 Haskell peer nodes**.
  Proves the full recover→forge→adopt loop (and BA-04's 2-Haskell-node shape) with **no
  preprod risk**. Non-promotable rehearsal — the C2 analog of the C1 genesis rehearsal.
- **C2-PREPROD-LIVE — the bounty (§6).** The real preprod Conway tip (Mithril/snapshot +
  extraction). The **identical** loop on the public chain, once Ade's pool stake is active.

**Hard invariants (both venues):** C2 never starts from genesis / slot 0; the private
chain must be **already bootstrapped past the cold-start** (§4) and sitting at a
non-Origin Conway tip with Ade stake **already active**; Ade only ever ENTERS by recover,
**never forges block 0**. The recover/import is the SAME path proven against preprod slot
124140368 (N-M-C / N-M-A1.1).

---

## 1. The recurring wrong turn (the genesis trap)

A local rehearsal **is** required before preprod (§5b) — but on an **already-bootstrapped**
private chain (non-Origin Conway tip, Ade stake active), never by **building one from
genesis**. The trap is the latter, and it arrives dressed as:

> "Spin up a fresh local private net (`create-testnet-data`, Conway-from-genesis), let it
> produce a tip, then have Ade recover from that local tip."

**This is wrong, and it is a dead end:**

- A freshly-created net's chain starts at **slot 0**. To reach a non-Origin tip it must
  be **built up from genesis** — the thing C2 must never do.
- A **Conway-from-genesis** net is **stillborn for block production** (see §4): it
  cannot produce its own first block, so it never reaches a non-Origin tip anyway.
- Even if it could, a hand-built net is **not the real Conway chain** — it's not C2.

The seductive sub-goal that drives this: "build a local net where **Ade itself holds
stake**, so we can demo forge+adopt without waiting for preprod stake." That sub-goal is
the trap. It forces a fresh genesis (slot 0) + the stillborn Conway cold-start, and it
isn't faithful C2 regardless.

---

## 2. Red flags — STOP immediately if you catch any of these

In a plan, slice doc, scoping pass, explanation, or command:

- ❌ `create-testnet-data` / `create-staked` / "fresh genesis" / "private net" as the C2 venue
- ❌ "let the (Haskell **or** Ade) node forge **block 0** / from **Origin** / from **slot 0**"
- ❌ "short epochs / small `securityParam` / small `k`" to make a local net produce faster
- ❌ "make the local **Haskell node** a producer" (it is **stillborn** on these nets — §4)
- ❌ "prime the local genesis with stake so the pool is active at epoch 0" (impossible — §4)
- ❌ "Ade forges its own base chain (block 0 → 1 → 2) and recovers from that" (own-tip
  advance is OQ-R1, **not** C2's recover-from-an-existing-foreign-tip)
- ❌ a multi-era OBFT/`d`-bridge local net just to reach a Conway tip locally

Any of these = you have drifted into the genesis trap. Re-read §0 and §3.

> **Not a red flag:** a properly-bootstrapped **multi-era `d`-bridge** producing net
> (Shelley/Babbage `d=1` → genesis-delegates forge the cold-start → fork up to Conway),
> or `cardano-testnet`, used as a **local rehearsal venue** where the Haskell node makes
> the tip and **Ade recovers** from it. That is the legitimate local pre-preprod path
> (§5b) — distinct from the stillborn `create-testnet-data` Conway-from-genesis trap.
> The line that's never crossed: **Ade enters by recover at a Conway tip, never forges
> block 0.**

---

## 3. The correct mental model — restore, don't replay

- A Cardano **genesis IS slot 0** (the origin). Only a *freshly created* net starts there.
- A **node restores from a snapshot at a non-Origin tip and continues** — it does not
  replay from genesis every boot. That is how Mithril / fast-sync / every real node works.
- So "get a non-Origin Conway tip" is **not** a build-from-genesis problem. It is a
  **restore-from-the-real-chain** action: the synced preprod node already sits at the tip;
  Mithril hands us a verifiable snapshot of it; Ade recovers from it. **No slot 0.**

One sentence: **The Conway tip already exists on preprod. Recover it. Don't grow one.**

---

## 4. Why a fresh local Conway net cannot be the venue (the stillborn cold-start)

Proven empirically this session (leadership-schedule + 6-min CaughtUp producer + zero
forge), and from first principles:

- A stake pool's stake activates **2 epochs after registration** (genesis → `mark` →
  `set` → `go`). At epoch 0 the pool has **no leader stake** (`leadership-schedule:
  "has no stake"`), so it cannot forge the first block.
- Real Cardano bridges that cold-start with the **OBFT / federated phase**:
  `decentralizationParam d = 1` at launch → **genesis-delegate keys forge the early
  blocks with no stake**, until pool stake activates, then `d → 0`.
- **Conway removed `d`** (Cardano was fully decentralized before Conway). So a
  **Conway-from-genesis net has no OBFT bridge** → nothing forges epochs 0–1 → the chain
  never reaches epoch 2 → the pool never activates → **stillborn**. No epoch-length,
  networking (`PeerSharing`/GSM), safe-zone, or stake-priming change fixes a missing
  consensus bootstrap.
- Corollary: making the local **Haskell node** a producer is stillborn for the same
  reason; only **Ade** can "forge" on such a net, and only by forging **block 0 from
  Origin** (the genesis trap), because its leadership comes from the extracted bundle.

`cardano-testnet` produces local nets only because it automates a **multi-era / `d`-bridge
bootstrap** (start pre-Conway, delegates forge, fork up). That `d`-bridge is **NOT** the
C2 path itself — it is the one-time venue **precondition** for the required C2-LOCAL
rehearsal (§5b): it produces the already-past-bootstrap private Conway chain (Ade stake
active, 2 Haskell nodes) that Ade then **recovers** from. The dead end is only
`create-testnet-data` **Conway-from-genesis** (no `d`-bridge → stillborn → pushes Ade to
forge block 0). With the `d`-bridge venue and Ade entering by **recover**, it is correct.

---

## 5. The two halves of C2 (and which gates which)

| Half | Needs Ade stake? | Status | How |
|---|---|---|---|
| **RECOVER** the real non-Origin Conway tip | **No** | **Proven** (N-M-C / N-M-A1.1; RO-LIVE-05 enforced @ slot 124140368) | Mithril snapshot (UTxO) + `build_consensus_inputs_bundle.sh` (consensus inputs) + `import_live_consensus_inputs` + admission `seed_to_snapshot` @ the tip |
| **FORGE tip+1 + real-peer ADOPT** | **Yes** | **Gated on Ade *active* stake in the target chain** | C2-LOCAL: already active by the §5b bootstrap precondition (rehearse here first). C2-preprod: register the pool + delegate + wait ~2 epochs (~10 days). |

**The stake gate is unavoidable and no snapshot bypasses it.** For a peer to *adopt*
Ade's block tip+1, it validates the VRF leader proof against Ade's pool's stake fraction
in the peer's leader-election distribution. If Ade's pool isn't active-staked on that
chain, σ=0, threshold=0, the block is rejected. Mithril hands Ade the *tip state*; it
cannot make Ade a *leader* on a chain where it isn't staked.

So: **recover is doable now; forge+adopt needs Ade *active* stake on the target chain —
rehearsed FIRST on a private chain where stake is already active (§5b), then preprod
(register + wait).** Nothing else should be invented to "get around" the stake gate.

---

## 5b. C2-LOCAL-REHEARSAL — required before preprod

A private-testnet rehearsal is **REQUIRED before touching preprod**, and it must rehearse
the **exact preprod shape**: recover a non-Origin Conway tip → forge tip+1 → a real
Haskell peer adopts → correlate. Its purpose is to de-risk the real mechanics cheaply,
off the public chain. It must **NOT** regress into fresh Conway-from-genesis bootstrapping
(§1, §4) — that tests nothing real.

### Precondition (venue setup — a one-time bootstrap, SEPARATE from the rehearsal)

An **already-bootstrapped private Conway chain** sitting at a non-Origin Conway tip, with:
- **Ade's pool already ACTIVE in the ledger stake** (past the 2-epoch activation), and
- **2 Haskell nodes** as peers — exactly BA-04's "private testnet with 2 other Haskell nodes".

Reaching that state is the cold-start bootstrap — done by the **Haskell venue**, not Ade,
and **not** by `create-testnet-data` Conway-from-genesis (stillborn, §4). The correct
bootstrap is the **multi-era `d`-bridge**: genesis in a pre-Conway era (Shelley/Babbage)
with `decentralizationParam d = 1`, so the Haskell **genesis-delegate** keys forge the
cold-start blocks (no stake needed → chain stays at wall-clock, never `CurrentSlotUnknown`);
Ade's pool stake activates (~2 epochs); ramp `d → 0`; hard-fork up to **Conway**.
`cardano-testnet` automates exactly this. **This bootstrap is a one-time venue
precondition; the rehearsal starts from its output — a non-Origin Conway tip with Ade
staked — and never re-touches genesis.**

### The rehearsal (the part that mirrors preprod — the part worth proving)

Against that already-advanced private chain + its 2 Haskell nodes:
1. **Extract** recovered-tip state from a Haskell node (`build_consensus_inputs_bundle.sh`
   + a Mithril/UTxO seed) at the current non-Origin Conway tip.
2. **Ade recovers/imports** that tip (`import_live_consensus_inputs` + `seed_to_snapshot`).
3. **Ade forges tip+1** over the evolved recovered state → self-accept → durable admit.
4. **Ade serves** the block; a **Haskell node adopts** it.
5. **`correlate`** the Haskell adoption log → non-promotable `PrivateRehearsalManifest`
   (`venue = private-testnet`, NOT bounty evidence).

This is the C2 analog of the C1 genesis rehearsal, and it exercises the **exact risky
mechanics** before preprod: recovered-tip extraction, live ledger / chain-dep import,
forge-successor over evolved state, durable admission, block serving, **real Haskell
adoption**, and BA-02 evidence correlation. Ade enters only by recover at the Conway tip
— **never forges block 0**.

### Concrete venue recipe (cardano-testnet — VALIDATED for #1–#4, 2026-06-06)

`cardano-testnet` is the venue generator (it runs the `d`-bridge bootstrap internally). It
is **not** in the cardano-node docker image — it ships in the release tarball
`cardano-node-11.0.1-linux-amd64.tar.gz` (`./bin/cardano-testnet`). Run it **inside the
matching `cardano-node:11.0.1` image** (so its child nodes get the right libs), binaries
mounted, with the env vars it requires (it fails `Could not find plan.json` without
`CARDANO_CLI`/`CARDANO_NODE`):

    docker run -d --name c2-testnet --network host \
      -v ~/.cardano-c2-testnet:/work -v ~/.cardano-testnet-bin:/ctn \
      -e PATH=/ctn/bin:/usr/bin:/bin -e HOME=/work -e TMPDIR=/work/tmp \
      -e CARDANO_CLI=/ctn/bin/cardano-cli -e CARDANO_NODE=/ctn/bin/cardano-node \
      -e CARDANO_SUBMIT_API=/ctn/bin/cardano-submit-api \
      --entrypoint /ctn/bin/cardano-testnet ghcr.io/intersectmbo/cardano-node:11.0.1 \
      cardano --num-pool-nodes 3 --testnet-magic 42 \
              --epoch-length 2000 --slot-length 1 --output-dir /work/env   # slot-length MUST be integer

It "keeps running until stopped" and reaches a **producing Conway chain within a minute or
two** — cardano-testnet **seeds the initial stake distribution active from epoch 0**
(unlike `create-testnet-data`'s `mark`-only, §4), so the SPO pools forge immediately in
Conway (validated: epoch 0, block 39, 3 nodes producing — **no epoch-2 wait, pool1 is an
active leader at once**). It writes the env to `~/.cardano-c2-testnet/env/`: 3 SPO pools
`pools-keys/pool{1,2,3}/` (cold/vrf/kes/opcert — **active-staked identities**), sockets
`socket/node{1,2,3}/sock`, logs `logs/node{N}/`, N2N ports per `node-data/*/topology.json`.
**#1–#4 validated:** `query tip` → epoch 2–3, era Conway, block 70–105, 3 nodes producing.
**`--slot-length` MUST be an INTEGER (use `1`, NOT `0.1`):** Ade's `parse_shelley_genesis`
does `require_u64("slotLength")` and rejects a fractional `slotLength` per its no-floats
determinism invariant — a fractional value fails the forge path with exit 44. With
`--slot-length 1 --epoch-length 2000` the epoch is ~33 min (a comfortable forge window),
and `slotsPerKESPeriod=129600` keeps the opcert (KES period 0) fresh.)

### The integration (#5–#9) — and the key finding from the first run

1. **Adopt pool1's identity** — `pools-keys/pool1/{cold.skey,vrf.skey,kes.skey,opcert.cert}`
   as Ade's operator keys (already active-staked — no registration wait).
2. **#5 Extract** the live tip + consensus inputs from a peer
   (`build_consensus_inputs_bundle.sh`; `ADE_LIVE_PEER_CONTAINER=c2-testnet`,
   `ADE_LIVE_PEER_SOCKET=/work/env/socket/node2/sock`, magic 42; the venue uses
   `configuration.yaml`, so feed the shelley hash via a small `{"ShelleyGenesisHash":"…"}`
   from `cardano-cli genesis hash --genesis shelley-genesis.json`); dump the seed UTxO. **DONE.**
3. **#6 Ade recovers** @ the live non-Origin tip (`--mode admission` `seed_to_snapshot`). **DONE.**
4. **#7 Ade forges** pool1's successor (`--mode node`, pool1 keys + recovered store +
   `--peer node2 --listen node1-port`). **DONE — 2 `forge_result:succeeded` pool1 blocks
   from recovered non-Origin Conway state.**

> **#8 KEY FINDING (2026-06-06): node2/node3 must be NON-PRODUCING relays, not competing
> producers.** The first run had all 3 cardano-testnet nodes as **pools**. Ade `--mode node`
> forges on its **own recovered chain** (`forge_one_from_recovered`) and follows the peer
> *separately*; with node2/node3 *also* producing (pool2/pool3), Ade's pool1 branch **loses
> fork-choice** to their longer chain and Ade's extend-only receive-side **fails closed**
> (`BlockNoOutOfOrder{last:10,attempted:9}`). Ade has no fork-choice to be
> one-producer-among-several — and per the guardrail we do **not** add it. **C1 worked only
> because Ade was the SOLE producer + the peer a PURE follower.** So #8 needs that shape.

**Two-phase relay venue (the fix — venue config, NOT an Ade-semantics change):**
- **Phase 1:** cardano-testnet 3-pool cluster bootstraps to a non-Origin Conway tip (above).
- **Phase 2:** **stop the cluster**, then re-run node2/node3 as cardano-node **relays** —
  reuse their `node-data/nodeN/{db,configuration.yaml}`, point topology at Ade's listen port,
  and **omit `--shelley-kes-key`/`--shelley-vrf-key`/`--shelley-operational-certificate`** so
  they do not forge. Now Ade (pool1) is the **sole producer**; the chain only advances via Ade.
- Ade recovers @ the frozen tip → forges pool1 successors → **#8 the relays adopt** (no
  competition) → **#9 correlate** → manifest: **`C2-LOCAL-REHEARSAL`, non-promotable, private
  Conway venue, Haskell peers (relays): node2,node3, Ade identity: pool1, forged hash ==
  adopted hash.**

> **Status (RESOLVED, 2026-06-07): #8–#9 PROVEN — the CE-A5 manifest.** A real cardano-node
> 11.0.1 relay (non-producing, per the #8 finding) **`AddedToCurrentChain`** an Ade-forged
> block: venue `c2ae18`, Ade=pool1 recovered @ block 8 → followed the relay to its frozen tip
> (block 16) → forged **block 17 @ slot 421** → served it → the relay committed it
> (`newtip db3b5675…@421`; `issuerHash a1ed4e04` = blake2b-224(pool1 cold VK); relay
> forging=0; Ade `forge_result:succeeded`=1). Evidence:
> `docs/evidence/phase4-n-ae-ce-a5-relay-adoption.{md,jsonl}`.
>
> **What closed it — PHASE4-N-AE** (and the hypothesis it corrected): AE.A (forge-on-followed-tip
> gate, DC-NODE-15) + AE.C (recover→follow WAL prior-fp, DC-WAL-02) + AE.B (recovered/forge-parent
> FindIntersect-only projection, DC-NODE-14) addressed Gaps 2a/2b — but FOUR runs still failed
> `UnexpectedBlockNo(N)(0)` until **AE.E**. Dumping the live store **DISPROVED the
> `seed_to_snapshot` hypothesis** in the superseded block below: the store was a COMPLETE
> contiguous chain (block 0–20) and `ChainDbServedSource::intersect` was correct on it. The real
> bug was the chain-sync **server** (`ade_network::chain_sync::server`): the `FindIntersect`
> handler resolved the intersect (the relay's tip) and replied `IntersectFound` but **never set
> the read cursor `last_announced`**, so the next `RequestNext` served `next_after(None)` = block 0
> → `UnexpectedBlockNo(tip+1)(0)`. AE.E sets the cursor to the resolved intersect (**DC-PROTO-10**).
> **Anti-stray lesson: the live run is the arbiter — AE.B "looked right" hermetically but did NOT
> close it; never declare a live gap closed on a hermetic fixture alone.**
>
> **Post-adoption echo fixed — AE.F (DC-NODE-16):** after adoption the relay re-announced Ade's own
> block over the follow link → Ade fail-closed `SlotBeforeLastApplied` (exit-43) AFTER the manifest.
> AE.F makes a re-announced already-have block (hash-exact) an idempotent no-op at the durable-admit
> chokepoint, so a continuous run survives its own served tip coming back (rung-1 prerequisite, §7b).
> The #8 KEY FINDING stands: relays stay **non-producing** (multi-producer fork-choice is **rung 2**
> of the §7b ladder, NOT added to Ade here). **C2-preprod-live (§6) is rung 3 of §7b.**
>
> ---
>
> **Prior-run history (2026-06-06 — SUPERSEDED by the RESOLVED status above; the `seed_to_snapshot`
> root-cause hypothesis at the END of this block was DISPROVEN by the AE.E finding):** **#1–#7 DONE** — venue validated; live tip + consensus
> inputs extracted; Ade recovered from the non-Origin tip; Ade forged **real pool1 blocks**
> (`forge_result:succeeded`) from that recovered state. **#8–#9 NOT proven** — forge success
> is local; external validity needs a Haskell peer to receive→validate→adopt→correlate. The
> two-phase relay venue **was executed** and *removed* Gap 1 (no more competing-producer
> `BlockNoOutOfOrder`), but exposed **Gap 2**: Ade's forge-tick **races the follow** — it
> forged its own block on the recovered tip instead of first adopting the relay's tip block,
> so its chain **forked** and node2-relay **rejected** the served chain
> (`UnexpectedBlockNo`). Both gaps are recorded as later invariant slices in
> `docs/planning/c2-local-discovered-gaps.md` (Gap 1: multi-producer fork-choice; Gap 2:
> forge-on-followed-tip + serve continuity). **Not** worked around by weakening Ade.
> **Recover-far-behind run EXECUTED (2026-06-06)** (anchor block 8, relays frozen at block
> 21, k=13): it **eliminated Ade's receive-side error** — Ade caught up cleanly and forged
> **BlockNo 22 = followed-tip+1, no `BlockNoOutOfOrder`** — but node2-relay **still rejected
> the served chain** (`UnexpectedBlockNo (22)(0)`). So Gap 2 splits: **2a forge-on-followed-tip**
> (met by orchestration at the BlockNo level) vs **2b serve-continuity** (Ade's durable
> served chain isn't peer-adoptable — the crux, UNMET). **Orchestration is exhausted; #8–#9
> remain unproven.** Root cause (grounded in code): `ade_node::admission::seed_to_snapshot`
> persists a ledger **snapshot** at the anchor, **not servable `StoredBlock` bytes**, so the
> recovered anchor is not a peer-intersectable servable block (`ChainDbServedSource` serves
> only servable blocks) → the served chain isn't a continuous lineage the relay can intersect.
> The next work is the **Gap 2 implementation slice — PHASE4-N-AE Slice A**, whose IDD
> invariants gate is **done** (`docs/planning/phase4-n-ae-slice-a-invariants.md`:
> recover→serve servable lineage + forge-on-followed-tip admissibility gate + structured
> `ForgeRefused::NotCaughtUp` + replay determinism); both gaps scoped in
> `docs/planning/c2-local-discovered-gaps.md`. Pick up fresh at `/cluster-doc PHASE4-N-AE`.
> Recover alone is also proven against the synced preprod node read-only. **C2-preprod-live
> (§6) runs only after Slice A closes this local loop.**

---

## 6. The recipe (commands)

Conventions: `PP=.cardano-node-preprod` (synced docker node, magic **1**, socket
`/opt/cardano/ipc/node.socket`), `ADE=/home/ts/Code/rust/ade`,
`OUT=~/.cardano-c2-preprod/ade-inputs`.

### Step 1 — confirm the real tip
```
docker exec cardano-node-preprod cardano-cli query tip \
  --testnet-magic 1 --socket-path /opt/cardano/ipc/node.socket
# expect: era Conway, syncProgress 100.00, a real non-Origin slot/block (e.g. epoch 293, slot ~125M)
```

### Step 2 — extract consensus inputs at the real tip (venue-general)
```
bash "$ADE/ci/build_consensus_inputs_bundle.sh" "$OUT/consensus-inputs.json"
# defaults = preprod. The extractor reads ASC + epochLength FROM the preprod shelley-genesis
# (1/20, 432000) — venue-general, NOT hardcoded. Confirmed: 465 pools, epoch 293, ASC 1/20.
```

### Step 3 — Mithril-verified seed UTxO
Obtain the seed UTxO from a **Mithril-verified** preprod snapshot (the trustworthy UTxO
source; see `feedback-mithril-is-peer-infra-not-ade-authority`), or a cardano-cli
`query utxo --whole-utxo` dump of the synced node, into `$OUT/utxo-seed.json`. The full
preprod UTxO (~1.9M entries) imports cleanly (N-M-A1.1).

### Step 4 — recover @ the real tip (admission)
```
ade_node --mode admission \
  --json-seed             "$OUT/utxo-seed.json" \
  --consensus-inputs-path "$OUT/consensus-inputs.json" \
  --seed-point-slot       <real tip slot> \
  --seed-block-hash       <real tip hash> \
  --peer 127.0.0.1:9 --snapshot-dir … --wal-dir … \
  --genesis-path "$PP/config" --genesis-hash <preprod ShelleyGenesisHash> --network-magic 1
# seed_to_snapshot captures a snapshot AT the real non-Origin tip. NO slot-0 replay.
```

### Step 5 — forge tip+1 + serve (needs registered stake) → adopt → correlate
`ade_node --mode node` (WarmStart the recovered store → forge tip+1 → durable admit →
serve) against the preprod peer; capture the peer's acceptance log; `ba02_evidence::correlate`
→ `Ba02Manifest` (`CN-OPERATOR-EVIDENCE-01`). **Only reachable once Ade's pool stake is
registered + active on preprod.**

---

## 7. What is proven vs owed (do not overstate)

- **Proven now:** recover the real preprod Conway tip (extraction at epoch 293 / slot
  125056777; full recover N-M-C / N-M-A1.1; RO-LIVE-05 enforced). Tip-successor **durability**
  hermetic (PHASE4-N-AD). **The C2-LOCAL adoption manifest (CE-A5, 2026-06-07): a real Haskell
  relay committed an Ade-forged block** (§5b; PHASE4-N-AE.A/B/C/E; echo fixed by AE.F) — a
  **non-promotable private-venue rehearsal** that does NOT flip any RO-LIVE rule on its own.
- **Owed — the robustness ladder (§7b), in order:** rung 1 sustained / settlement-beyond-k +
  the epoch transition (C2-LOCAL single-producer, cheap — the NEXT step); rung 2 multi-producer
  fork-choice (C2-LOCAL, a new cluster); rung 3 C2-preprod register (~2-epoch gate) + the
  operator pass → `Ba02Manifest`.
- **`CN-CONS-06` / `RO-LIVE-01` live halves flip ONLY** on committed `correlate` evidence over
  a real **preprod** peer log — never on recover/forge/serve alone, and never on the local
  rehearsal (non-promotable). The CE-A5 manifest is rehearsal evidence: it **strengthened**
  CN-CONS-06 (first live cross-impl acceptance) but did **NOT** flip it.

---

## 7b. The robustness ladder — phased de-risking after the manifest

**Principle (anti-stray): exercise each failure class in the CHEAPEST venue that can produce
it.** A bug found in C2-LOCAL costs hours; the same bug in preprod costs the ~2-epoch
(~10-day) stake-snapshot gate to retest. Climb in order. **Do NOT skip the hermetic
multi-producer rung — preprod IS multi-producer, so skipping rung 2 means first-contact with
fork-choice in the venue where every retest costs ~10 days.**

Venue constants (from the venue shelley-genesis): `epochLength 2000 × slotLength 1s` =
**epoch ≈ 33 min**; `securityParam k=5` → stability window `3k/f = 300 slots` ≈ **5 min**;
`slotsPerKESPeriod 129600` ≈ **36 h** (a KES period is **out of reach** — don't target it;
cross-period KES is already covered by N-AC / N-P).

| Rung | Venue | Failure class isolated | Likely new work | "Done" = |
|---|---|---|---|---|
| **1** | C2-LOCAL, single-producer (the existing CE-A5 harness) | operational stability · **settlement beyond k** · **epoch transition** · restart durability | maybe an epoch-transition slice | several Ade blocks settle `>k` into the relay's ImmutableDB · **1 epoch crossed** with adoption continuing · survives a **mid-run kill→warm-start** |
| **2** | C2-LOCAL, multi-producer (un-freeze a 2nd pool so it competes) | **fork-choice / rollback** — DC-CONS-03, never exercised live | a live fork-choice cluster (**real** new work) | Ade selects the best chain across a real fork **and** recovers from losing one |
| **3** | preprod | real network · stake · load · adversaries | SPO registration + ~2-epoch stake gate + operator wiring | a registered Ade SPO's block accepted on preprod → `Ba02Manifest` (flips RO-LIVE-01 / CN-CONS-06 live half) |

**Rung 1 is the cheap, high-value next step** (and validates AE.F): run the existing CE-A5
harness ~5 min (**settlement beyond k** — strictly stronger than the one-block manifest:
Ade's blocks become the relay's *immutable* history, not just its *selected* tip), then
continue past slot 2000 (one **epoch**). The epoch transition is single-producer and the path
most likely to surface a gap — Ade's `--mode node` loop has not been shown to roll
nonce / re-snapshot stake / recompute leadership live. **Find that here, not in preprod.**

**Honest scope (at any rung length):** rung 1 proves operational stability + settlement, NOT
fork-choice (single producer → one chain → DC-CONS-03 never fires) and NOT real-stake
economics. **Tie the climb to what the bounty requires** — the manifest already cleared the
*minimal* block-production bar; the ladder buys robustness/confidence, not a new acceptance
gate (only rung 3's preprod `correlate` flips an RO-LIVE rule).

---

## 8. Anti-regression checklist (the wrong turns this session, never repeat)

- ✅ The Conway tip is **recovered** from an already-bootstrapped chain (private rehearsal
  or preprod), never **grown by Ade from a local genesis**.
- ✅ Ade enters via `seed_to_snapshot` @ a **non-Origin** slot, never block-0-on-Origin.
- ✅ **Private testnet is REQUIRED before preprod, but it rehearses the recovered
  non-Origin path** (recover → forge tip+1 → 2 Haskell peers adopt → correlate, on an
  **already-bootstrapped** Conway chain). It must **not** regress into fresh
  Conway-from-genesis bootstrapping.
- ✅ No `create-testnet-data` **Conway-from-genesis** / short-epoch / Ade-forge-from-slot-0
  on any C2 venue. (The `d`-bridge multi-era net is the **required** local-rehearsal
  precondition — §5b — **not** forbidden.)
- ✅ Forge+adopt is gated on Ade **active stake in the target chain** — local rehearsal
  first (stake active by the §5b precondition), then preprod (register); never faked.
- ✅ `PrevHash::Genesis`, the BLUE Sum6KES/VRF/header rules, and the importer are unchanged.
- ✅ No RO-LIVE flip without committed `correlate` evidence over a **preprod** peer log.
- ✅ The robustness ladder (§7b) is climbed **in order** (C2-LOCAL single → C2-LOCAL
  multi-producer → preprod); never first-contact fork-choice in preprod, and never declare a
  **live** gap closed on a hermetic fixture alone (the AE.B→AE.E lesson: the live run is the arbiter).

If a plan has **Ade** forge from genesis/slot 0, or builds a **Conway-from-genesis** net
for Ade to bootstrap, it has drifted. Local block production by the **Haskell venue** (the
`d`-bridge, to reach a past-bootstrap tip) is correct — Ade only ever **recovers**.

---

## 9. Authoritative sources

- Sibling guide: `docs/active/live-pass-path-fidelity-guide.md` (extract, don't construct;
  same `--mode node` path for every venue).
- Extraction tool: `ci/build_consensus_inputs_bundle.sh` (venue-general: reads ASC +
  epochLength from the venue shelley-genesis; preprod defaults).
- Import authority: `ade_runtime::consensus_inputs::import_live_consensus_inputs`;
  recover: `ade_node::admission::seed_to_snapshot`.
- Registry: `RO-LIVE-01` (Haskell peer accepts an Ade-forged block — the bounty),
  `RO-LIVE-05` (recover/admit, enforced), `CN-OPERATOR-EVIDENCE-01`, `DC-MITHRIL-02`,
  `RO-MITHRIL-IMPORT-01` (dedicated Mithril-import slice).
- Durability: `PHASE4-N-AD` (tip-successor WAL-replay recovery), `T-REC-05`, `DC-WAL-04`.
- Memory: `project_oqr1_tipseed_durability`, `feedback_live_pass_path_fidelity`,
  `feedback_mithril_is_peer_infra_not_ade_authority`, `reference_local_preprod_docker_cardano_node`.
```
