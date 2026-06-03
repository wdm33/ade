# Live-Pass Path Fidelity — the C1/C2 accepted-block path

> **Read this before scoping, planning, or explaining any live pass (C1 private
> rehearsal or C2 preprod/preview bounty pass).** It exists because the same wrong
> turn keeps recurring — *in reasoning and writeups*, upstream of the CI gate that
> already forbids it in code. The gate (`ci_check_node_path_fidelity.sh`) catches
> the bad *code*; this guide catches the bad *thinking* before you waste a cluster
> on it.

---

## 0. The one rule

**C1 and C2 use the SAME `--mode node` accepted-block path. The ONLY differences
are operator-controlled INPUTS and the evidence LABEL — never code.**

The path, identically for both venues:

```
N-M-C extraction (build_consensus_inputs_bundle.sh against the venue's Haskell node)
  → import_live_consensus_inputs  (the SOLE consensus-input authority)
  → forge → self-accept → sibling-serve → block-fetch
  → peer log → ba02_evidence::correlate
```

- **C1 vs C2 differ in:** (1) which Haskell node you extract from + serve to, (2) a
  private genesis whose **stake allocation** makes Ade win slots fast, (3) the
  evidence label (C1 = non-promotable rehearsal; C2 = bounty). **Nothing else.**
- **If a condition would fail on C2/preprod, it is a SHARED-PATH BUG to fix in the
  shared path — never special-cased for C1.**

This is `CN-REHEARSAL-FIDELITY-01` clause 1 (release tier, enforced).

---

## 1. The recurring failure mode (the seductive wrong turn)

The mistake always arrives dressed as reasonable engineering. It goes:

> "C1 is a *private* net. Ade must be a slot leader. The Haskell peer derives
> `eta0`/stake/ASC/VRF from the shared genesis. So Ade should **build its consensus
> inputs from the genesis too**, to match the peer."

**This is wrong.** The forbidden line — the one to never write — is:

> ~~"Build Ade's consensus inputs from the genesis."~~

It re-introduces a **from-genesis consensus-input constructor**, which was
deliberately **deleted** in the earlier C1 correction. It creates a **private-only
path** that diverges from the C2/preprod accepted-block path, which makes the
rehearsal **misleading** (it can pass while the real bounty path is broken).

Why the reasoning *feels* right but isn't: see §3.

---

## 2. Red flags — STOP immediately if you catch any of these

In a plan, slice doc, scoping pass, explanation, or code:

- ❌ "build / construct / derive consensus inputs **from genesis**"
- ❌ "from-genesis bundle constructor / helper"
- ❌ "`eta0` / stake / ASC / VRF **from genesis**" (as an Ade-side source)
- ❌ "a `--private-net` / `--from-genesis` / `--devnet` / `--rehearsal` flag"
- ❌ "a C1-only bootstrap / branch / path"
- ❌ "make C1 work" / "C1-specific extraction" / "special-case for the private net"
- ❌ a fn named `*genesis*consensus*` or `*consensus*genesis*`

Any of these = you have drifted off the shared path. Re-read §0 and §3.

---

## 3. The correct mental model — consistency is FREE

The consistency you were trying to manufacture ("derive from genesis so Ade matches
the peer") is **already guaranteed by construction — you do not build it**:

- The **Haskell node** turns genesis into `eta0`/stake/ASC/VRF. That is the node's
  job, not Ade's.
- Ade **extracts that node's already-computed view** via the shared cardano-cli
  extraction (`build_consensus_inputs_bundle.sh`), exactly as it does for preprod.
- **A view extracted from the running node cannot disagree with the node that
  produced it.** The shared-extraction path *is* the consistency guarantee.

So "make Ade's inputs match the peer" is not a problem to solve — it is the reason
the path-fidelity rule exists. Extract; do not construct.

---

## 4. Genesis: allowed vs forbidden

The private genesis is **legitimate operator configuration**. It becomes a violation
only when it turns into an Ade-side *consensus-input authority*.

| Genesis is ALLOWED as… | Genesis is FORBIDDEN as… |
|---|---|
| network magic | an `eta0` source for Ade |
| system start / slot length | a stake-distribution source for Ade |
| KES period parameters | an ASC source for Ade |
| genesis hash / network identity | a per-pool VRF-keyhash source for Ade |
| **the private stake allocation that gives Ade leader rights** | a consensus-input constructor / bundle builder |
| (i.e. operator setup that makes slots fast) | a private-only bootstrap path |

One sentence: **Genesis configures the network and stakes Ade. Genesis never feeds
Ade's consensus inputs — the running node does, through the shared extraction.**

---

## 5. The one correct flow (C1 and C2)

| Step | Owner | C1 | C2 |
|---|---|---|---|
| 1. Stake Ade + supply magic/start/slot/KES/hash | operator | private genesis (Ade ~all stake) | register an Ade pool on preprod |
| 2. Bring up the venue's Haskell node | operator | private C1 node | the preprod node |
| 3. Extract consensus inputs from that node | operational | `build_consensus_inputs_bundle.sh` (env-pointed at C1: `ADE_LIVE_PEER_CONTAINER` / `ADE_LIVE_NETWORK_MAGIC` / `ADE_LIVE_PEER_SOCKET`) | same script, default preprod env |
| 4. Dump the seed UTxO (cardano-cli) | operational | C1 node | preprod node |
| 5. Import + run | operational | `ade_node --mode node --listen … --peer <c1> --json-seed … --consensus-inputs-path … --network-magic 42` | same flags, magic 1 |
| 6. Capture peer log → `correlate` | operational | non-promotable `PrivateRehearsalManifest` | bounty `Ba02Manifest` |

Same binary, same flags (from the pinned 28-flag set), same import authority, same
forge/serve/correlate path. The deltas are only columns 1–2 (operator inputs) and
the manifest label (row 6).

---

## 6. If it fails against the venue

When extraction, import, or forge trips on a venue quirk:

- ✅ **Patch the SHARED extraction script / import / forge / serve path** so it works
  for *both* venues.
- ❌ **Never** add a C1-only branch, a private-only helper, or a from-genesis
  constructor to "make C1 work."

A condition that fails on C1 but would also fail on C2 is a **shared-path bug**. A
condition that "only matters for C1" is a smell — re-read §0.

---

## 7. Mechanical enforcement (and what it can't catch)

**Enforced in code** by `ci/ci_check_node_path_fidelity.sh` (rule
`CN-REHEARSAL-FIDELITY-01` clause 1):

- **Guard (a):** `cli.rs` flag set == the pinned closed allow-list (28 flags). Adding
  `--private-net` / `--from-genesis` / `--devnet` / `--rehearsal` trips it.
- **Guard (b):** no fn whose name contains both `genesis` and `consensus`; and
  `node_lifecycle.rs` MUST source consensus inputs via `import_live_consensus_inputs`.

**What the gate CANNOT catch (why this guide exists):**

- A wrong **scoping plan / slice doc / explanation** that *proposes* the from-genesis
  path — prose runs no CI.
- A from-genesis constructor that **evades the name check** (avoids the words
  `genesis`/`consensus` in its identifier).
- Building the equivalent logic **inside** the import flow without a tell-tale name.

So the gate is the *downstream* defense (it stops the bad code from merging); this
guide is the *upstream* defense (it stops you from spending a cluster building toward
it, and from misleading the user in a writeup). **Both are required.** If you ever
find a from-genesis path that the gate would miss, that is a candidate to strengthen
the gate — not a loophole to use.

---

## 8. Tiering

- **true (invariant):** No private-only authority path; replay inputs must be
  explicit and comparable.
- **derived:** C1 must mirror the C2/preprod accepted-block path through
  `import_live_consensus_inputs`.
- **release:** C1 is a preflight rehearsal, not bounty completion.
- **operational:** Use the private genesis to configure the net and stake Ade —
  never to bypass the shared extraction path.

---

## 9. Authoritative sources

- Registry: `CN-REHEARSAL-FIDELITY-01` (both clauses), `DC-NODE-07` (single shared
  serve path), `RO-LIVE-01` (the bounty: a Haskell peer accepts an **Ade-forged**
  block — needs a **stake** venue, which is why C1/C2 exist and preprod-without-stake
  cannot show it), `RO-MITHRIL-IMPORT-01` (a **separate, orthogonal** seed-provenance
  feature — **not** on the forge-accept path; do not conflate "the Mithril option"
  with the bounty).
- Gate: `ci/ci_check_node_path_fidelity.sh`; sibling fences
  `ci/ci_check_node_run_loop_containment.sh`, `ci/ci_check_single_serve_dispatch_authority.sh`.
- Extraction tool: `ci/build_consensus_inputs_bundle.sh` (env-parameterizable; the
  SAME tool for every venue).
- Import authority: `ade_runtime::consensus_inputs::import_live_consensus_inputs`.
- Scoping: `docs/planning/operator-pass-live-leg-c1-scoping.md` (note: its §1 gap
  list is for the older `--mode produce` HEAD; the C1 dry-run runs `--mode node`, and
  the consensus-input "gap" it lists is resolved by **extraction**, not a constructor).
- Runbooks: `docs/evidence/phase4-n-f-g-d-private-rehearsal-README.md`,
  `docs/evidence/phase4-n-f-g-h-node-serve-README.md`.
