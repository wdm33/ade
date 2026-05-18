# Cluster PHASE4-N-A — Scope Decisions (Locked)

> **Authority**: planning artifact, not a normative doc.
> **Status**: pre-cluster — locked before `/invariants` runs.
> **Consumer**: `/invariants` for PHASE4-N-A will read this doc as
> seed input alongside `docs/active/phase_4_cluster_plan.md`.

This file captures two design decisions locked while cluster N-D
was still in flight, so they survive the N-D → N-A transition and
are not relitigated when the cluster is formally opened.

## Slice plan (authority-surface aligned)

```
S-A1   ade_network crate scaffold + mux/framing RED shell
S-A2   Protocol message codec authority                    (BLUE)
S-A3   Version negotiation authority                       (BLUE)  ← CE-N-A-1
S-A4   Chain-sync transition authority                     (BLUE)  ← CE-N-A-2
S-A5   Block-fetch transition authority                    (BLUE)  ← CE-N-A-3
S-A6   Tx-submission2 transition authority                 (BLUE)  ← CE-N-A-4
S-A7   Keep-alive + peer-sharing transition authority      (BLUE)
S-A8   N2C transition authority (4 protocols)              (BLUE)
S-A9   Session wiring + frame corpus + replay harness      (RED + GREEN)
S-A10  Live interop release evidence                       (RED)   ← CE-N-A-5
```

Each slice owns exactly one authority claim. BLUE/RED boundary is
mechanical, not editorial.

---

## Decision 1 — S-A2 codec grouping

**Locked**: one slice for all 11 protocol message codecs. No split
between N2N and N2C.

### Rationale

The invariant is not "N2N codecs exist" or "N2C codecs exist." The
invariant is:

> Every protocol-visible message in the supported Cardano N2N/N2C
> surface decodes into exactly one closed, versioned message type
> and round-trips byte-identically.

That maps directly to the existing doctrine that network-visible
messages must be closed, versioned enums, with no runtime
negotiation of meaning. The full protocol surface must be
explicitly enumerated:

- **N2N**: Handshake, ChainSync, BlockFetch, TxSubmission2,
  KeepAlive, PeerSharing
- **N2C**: Handshake, LocalChainSync, LocalTxSubmission,
  LocalStateQuery, LocalTxMonitor

Splitting N2N and N2C codecs would create two partial closure
points and invite a bad intermediate state where the codec
authority is only half-real. That is weaker than one carefully
bounded mechanical slice. Implementation volume (~50 message
variants) is real but separate from invariant complexity — the
mitigation is per-protocol test modules inside one slice, not slice
splitting.

### S-A2 specification

**Intent**: define the closed, versioned message taxonomy and pure
CBOR codecs for all N2N and N2C mini-protocol messages spoken by
the target Haskell node version (cardano-node 10.6.2).

**Execution boundary**: BLUE or GREEN depending on crate placement,
but **sync-only, pure**, no async, no sockets, no timers, no peer
state, no mux buffering.

**Invariants strengthened**:
- `CN-WIRE-07`: each protocol-visible message decodes into one
  closed, versioned message type
- `DC-PROTO-03`: full N2N surface is represented
- `DC-PROTO-04`: full N2C surface is represented
- `T-ENC-03`: valid encodings round-trip byte-identically (existing)
- `T-ERR-01`: decode failures are structured and comparable (existing)

(CN-WIRE-07, DC-PROTO-03, DC-PROTO-04 are new registry entries
proposed in this slice; not yet in `docs/ade-invariant-registry.toml`.
`/invariants` for PHASE4-N-A is responsible for proposing them.)

**Acceptance criteria**:
1. All 11 protocols have closed message enums.
2. Every message variant has golden encode/decode tests.
3. Unknown tags fail deterministically.
4. Invalid agency/message combinations are not accepted by codec
   tests where the codec has enough context; otherwise deferred to
   the relevant state-machine slice.
5. `decode(encode(msg)) == msg` for every generated message.
6. `encode(decode(bytes)) == bytes` for curated valid bytes.
7. Malformed corpus produces structured codec errors, not strings.
8. No `HashMap`, `HashSet`, float, clock, filesystem, socket, or
   async usage in codec modules.

**Hard prohibition**: S-A2 ships **no live protocol behavior**.
S-A2 is message byte authority only. State-machine legality
belongs in S-A3..S-A8. Live interop belongs in S-A10 as release
evidence.

---

## Decision 2 — Async runtime placement

**Locked**: tokio (and any async machinery) first appears in S-A1
and is permitted **only** in RED transport/runtime modules.

### Rationale

The core doctrine forbids BLUE code from reading OS/filesystem/
network or depending on thread scheduling effects. The shell may
observe nondeterminism but must convert it into deterministic
inputs before entering the core. Async runtime semantics are a
form of nondeterminism (task interleaving, timer fires, channel
ordering under contention) that must not leak into the
authoritative core.

### The rule (stronger than "tokio belongs in S-A1")

The following are **forbidden in BLUE authoritative crates and
modules**. Permitted only in RED transport/runtime code:

- `async fn`
- `.await`
- `tokio::` (any path)
- `async_std::` (any path)
- `Future` (the trait/type from `core::future` or `futures`)
- `futures::` (any path)
- task spawning (`tokio::spawn`, `async_std::task::spawn`,
  `futures::executor::spawn`, …)
- async channels in authoritative paths (`tokio::sync::mpsc`,
  `tokio::sync::oneshot`, `futures::channel::*`, …)
- timers (`tokio::time::sleep`, `tokio::time::timeout`,
  `tokio::time::interval`, …)

### CI enforcement

New script: `ci/ci_check_no_async_in_blue.sh`. Scans BLUE source
trees (per `core_paths` in `.idd-config.json`) for the patterns
above. First iteration is grep-based — cheap and visible. If false
positives become noisy, graduate to a syn-based scanner.

The check belongs in the same family as the existing
`ci_check_no_semantic_cfg.sh` and `ci_check_no_signing_in_blue.sh`
— pure-BLUE hygiene gates.

---

## Operational consequences for `/invariants` and `/cluster-doc`

When PHASE4-N-A is formally opened after N-D closes:

1. **Invariant registry edits** (`/invariants` proposes;
   `/cluster-doc` ratifies):
   - Add `CN-WIRE-07` strengthening of the CN-WIRE family
   - Add `DC-PROTO-03`, `DC-PROTO-04` under a new PROTO category
     within the DC family (consistent with DC-STORE-*, DC-LEDGER-*,
     DC-CRYPTO-* category prefixes)

2. **`.idd-config.json` `core_paths` additions** (when S-A1 lands):
   ```
   crates/ade_network/src/codec/
   crates/ade_network/src/handshake/
   crates/ade_network/src/chain_sync/
   crates/ade_network/src/block_fetch/
   crates/ade_network/src/tx_submission/
   crates/ade_network/src/keep_alive/
   crates/ade_network/src/peer_sharing/
   crates/ade_network/src/n2c/
   ```
   `crates/ade_network/src/mux/` and `crates/ade_network/src/session/`
   stay implicitly RED.

3. **New CI script**: `ci_check_no_async_in_blue.sh` lands with
   S-A1 (so the prohibition is enforced before S-A2 begins).

4. **Pallas-network policy**: CI oracle only, never a production
   dependency. Same quarantine pattern as
   `ci_check_pallas_quarantine.sh` already enforces for the ledger
   side.

5. **Frame corpus**: `corpus/network/` directory needs to exist
   before S-A9 becomes serious. Setup is offline, one-time, against
   a real cardano-node↔cardano-node session.

---

## What this doc is NOT

- Not the cluster doc. That is produced by `/cluster-doc PHASE4-N-A`
  after `/invariants` and `/cluster-plan`.
- Not the slice doc. Slice docs are produced by
  `/slice-doc PHASE4-N-A S-AN`.
- Not normative. The invariant registry is the normative authority
  for CN-WIRE-07, DC-PROTO-03, DC-PROTO-04 once those entries land.
- Not a substitute for the formal IDD entry points. Decisions
  recorded here feed into them.
