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

> **Status / next step (honest):** the recover step alone is proven against the synced
> preprod node read-only today. The **C2-LOCAL rehearsal is the immediate next build** —
> it needs the already-past-bootstrap private Conway chain (via the `d`-bridge /
> `cardano-testnet`) with Ade stake active + 2 Haskell nodes, which is **not yet stood
> up**. **That rehearsal — NOT preprod stake registration — is the next step.**
> C2-preprod-live (§6) runs only after the local rehearsal loop is green.

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

- **Proven now:** recover the real preprod Conway tip (extraction works at epoch 293 /
  slot 125056777 today; full recover proven N-M-C / N-M-A1.1; RO-LIVE-05 enforced). The
  tip-successor **durability** seam is proven hermetically (PHASE4-N-AD).
- **Owed (in this order):**
  - **(a) C2-LOCAL rehearsal — the NEXT step.** Stand up an already-past-bootstrap private
    Conway chain (`d`-bridge / `cardano-testnet`) with Ade stake **active** + **2 Haskell
    nodes**, then run recover → forge tip+1 → Haskell adopt → `correlate` there (§5b).
    **This, not preprod stake registration, is what comes next.**
  - **(b) Then C2-preprod.** Register Ade's pool stake on preprod (~2-epoch gate) + the
    forge+adopt operator pass once active → `Ba02Manifest`.
  - **(c)** `CN-CONS-06` / `RO-LIVE-01` live halves flip **only** on committed `correlate`
    evidence over a real **preprod** peer log — never on recover/forge/serve alone, and
    never on the local rehearsal (non-promotable).

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
