# Getting Started: Run Ade on Cardano Preview

This guide takes you from nothing to a running **Ade** node that follows the Cardano
**preview** testnet — bootstrapped from a verified Mithril snapshot, in three commands.

Every step has two parts:

- **Run it** — the exact command and what success looks like.
- **What's happening** — a plain-English explanation of what Ade is doing, and why.

If you just want to get going, copy the three commands below. If you want to understand
them, read the sections.

---

## TL;DR — the three commands

```sh
# 1. Fetch + verify a recent, certified chain snapshot
ade mithril snapshot fetch --network preview --output-dir ./preview-snapshot

# 2. First run: bootstrap from that snapshot and start following the chain
ade node run \
  --network preview \
  --bootstrap-mithril ./preview-snapshot/manifest.json \
  --snapshot-dir ./preview-snapshot \
  --data-dir ./ade-preview \
  --peer 127.0.0.1:3002

# 3. Restart anytime — Ade recovers from its own data, no snapshot needed
ade node run \
  --network preview \
  --data-dir ./ade-preview \
  --peer 127.0.0.1:3002
```

---

## Before you start

- **Build Ade.** From the repo root: `cargo build --release`. The binary is `target/release/ade`.
  Put it on your `PATH` (the examples assume `ade` is runnable), or use the full path.
- **Install `mithril-client`.** Step 1 uses it to download and verify the snapshot. It must be on
  your `PATH`. See <https://mithril.network>. Ade is tested with `mithril-client` 0.13.x.
- **Have a preview peer.** Steps 2 and 3 connect to a Cardano **preview** node over the network
  (its node-to-node address). This can be a public preview relay (see the preview testnet topology)
  or your own preview node (for example, the `cardano-node` Docker image run with `NETWORK=preview`).
  The examples use a local node at `127.0.0.1:3002`.
- **Disk space.** The snapshot is roughly 15 GB; Ade's own data directory adds a few GB.

---

## 1. Fetch a verified Mithril snapshot

### Run it

```sh
ade mithril snapshot fetch --network preview --output-dir ./preview-snapshot
```

It downloads (this can take several minutes), then prints a receipt like:

```
=== Ade verified Mithril snapshot receipt ===
network / profile      : preview (magic 2)
shelley genesis hash   : 363498d1024f84bb39d3fa9593ce391483cb40d479b87233f868d6e57c3a400d
mithril aggregator     : https://aggregator.pre-release-preview.api.mithril.network/aggregator
certified point        : slot 115676685 / block 2e0ac2d3...
ledger state           : ./preview-snapshot/db/ledger/115676685/state (37 MB)
ledger tables          : ./preview-snapshot/db/ledger/115676685/tables (630 MB)
immutable range        : 0 .. 26777
verification           : mithril-client verified the immutable files via the certificate; ancillary IOG-signed
manifest               : ./preview-snapshot/manifest.json
```

When it finishes, `./preview-snapshot/` holds the chain database (`db/`), a `manifest.json`, and a
copy of the receipt. That's everything step 2 needs.

### What's happening

A Cardano node normally has to replay the entire chain from the very first block to learn the
current state — that can take many hours. **Mithril** is a service that publishes recent,
*cryptographically certified* snapshots of the chain, so you can skip that replay and start from a
trustworthy recent point.

This command:

1. Asks the Mithril aggregator for the latest certified preview snapshot.
2. Downloads it with `mithril-client` (including the ledger state).
3. **Verifies** it. The chain history files are checked against the Mithril certificate. The ledger
   state itself is signed with IOG keys (Mithril's certificate doesn't cover those files directly,
   so a separate signature is used — that's the "ancillary IOG-signed" note you'll see).
4. Lays the files out and writes **`manifest.json`** — a small file that records the *network*, the
   *genesis hash*, and the *certified point* (the exact slot and block the snapshot represents).

That manifest is the trust anchor for the next step: Ade only bootstraps from a snapshot whose
manifest checks out.

> **Want a specific snapshot instead of "latest"?** Pass `--certificate <hash>`.

---

## 2. First run: bootstrap and follow

### Run it

```sh
ade node run \
  --network preview \
  --bootstrap-mithril ./preview-snapshot/manifest.json \
  --snapshot-dir ./preview-snapshot \
  --data-dir ./ade-preview \
  --peer 127.0.0.1:3002
```

You'll see a bootstrap receipt, then ChainSync starting:

```
=== Ade native Mithril bootstrap receipt ===
network / profile      : preview (magic 2)
shelley genesis hash   : 363498d1024f...
certified anchor point : slot 115676685 / block 2e0ac2d3...
...
ChainSync              : starting against 1 peer(s)
```

From here Ade connects to your peer and follows the chain. Leave it running.

### What's happening

This is the **first run** against a fresh `--data-dir`. Ade:

1. Reads the verified snapshot (through the manifest) and rebuilds its internal state at the
   certified point — its view of who owns what (the **ledger**) and the data it needs to check
   incoming blocks (the **consensus** state).
2. Prints a receipt so you can confirm the network, genesis, and anchor are what you expect.
3. Writes its own durable state into `--data-dir` (`./ade-preview`).
4. Connects to `--peer` and starts **ChainSync** — receiving each new block, validating it
   (including checking the block's producer was actually eligible to make it), and appending it to
   its chain.

A few things worth understanding:

- **`--network preview` resolves everything.** The genesis file, network magic, and protocol
  parameters all come from Ade's built-in preview profile. You do **not** pass a genesis file.
- **`--snapshot-dir` and `--data-dir` are different things.** `--snapshot-dir` is the *read-only
  Mithril input* you fetched in step 1. `--data-dir` is *Ade's own store* — its chain database and
  recovery journal. Keep them separate.
- **The snapshot is only needed once.** After this run has followed at least one block, everything
  Ade needs to restart already lives in `--data-dir`.

---

## 3. Restart: warm-start recovery

### Run it

Note: **no** `--bootstrap-mithril`, **no** `--snapshot-dir` — only the network, the data dir, and a peer.

```sh
ade node run \
  --network preview \
  --data-dir ./ade-preview \
  --peer 127.0.0.1:3002
```

Ade reads `./ade-preview`, recovers, and resumes following from where it stopped. There is **no**
bootstrap receipt this time — that only appears on a first run. You'll see it connect to the peer
and continue ChainSync.

### What's happening

Once Ade has bootstrapped and followed at least one block, its `--data-dir` holds everything needed
to continue: the chain tip it reached, the consensus inputs, and a recovery journal.

On restart, Ade:

1. Detects the existing durable state in `--data-dir`.
2. Classifies this as a **warm start** (not a first run).
3. Recovers the *exact same* state it had — the tip it had reached and everything behind it.
4. Resumes ChainSync from that point.

This is why you don't pass the snapshot or manifest again: **the durable store in `--data-dir` is
the source of truth.** The Mithril snapshot was just scaffolding to get started; once you're
running, Ade maintains its own state. As long as `--data-dir` is intact, you can stop and restart
Ade as often as you like and it will always pick up where it left off.

(If you start with an *empty* `--data-dir` and no snapshot flags, Ade has nothing to recover from
and will tell you a bootstrap is required — go back to step 2.)

> **Reclaim space.** After a successful first run, `./preview-snapshot` is no longer needed for
> restarts — the state is in `--data-dir`. You can delete it to free ~15 GB.

---

## What you have now

A preview node that bootstraps from a certified snapshot, follows the live chain, and survives
restarts by recovering its own durable state — the foundation for validation and, with operator
keys, block production.

## Notes and current limitations

- **Look after `--data-dir`.** It is your node's durable state. If you lose it, you simply
  re-bootstrap from a fresh snapshot (steps 1–2).
- **Point `--peer` at a preview node.** Ade pins the network through `--network preview`; a peer on
  a different network won't share a common chain and won't connect usefully.
- **Continuous operation across epoch boundaries** — the chain advancing into a new Cardano epoch
  while Ade follows — is in active development. Following and restarting *within* an epoch works as
  described here.
