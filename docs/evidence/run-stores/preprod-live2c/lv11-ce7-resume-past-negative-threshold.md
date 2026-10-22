# LV-11 CE-LV11-7 — live: the stopped store resumes past block 5,184,855

**Result:** the LV-11 binary warm-started on a copy of the store the LV-8 run left behind, with no reset
and no rebuild. It passed slot 133,902,219, the block whose threshold of −1 had stopped that run, and
kept following the local docker `cardano-node-preprod`. Thirty minutes in, its follow tip was at
134,005,635, and its log had no `failing closed`, `panicked`, `HALTING`, `LeadershipEpochNotSealed` or
`AccumulatorWalkHalted` line.

## 1. Run

| item | value |
|---|---|
| binary | release build of `a4610e16` (LV-11), sha256 `3eaceca25e84efb4…` |
| mode | relay only: no operator keys, no genesis file; warm start (`--data-dir`, `--peer` only) |
| store | `~/.cardano-live1/ade-preprod-lv11`, a copy of the stopped `ade-preprod-lv8b` (`chain.db` sha256 `3f76c5a0266f4014caae…`, identical to the original). Store semantics 11, not bumped (CE-LV11-6) |
| peer | local docker `cardano-node-preprod` at `127.0.0.1:3001` |
| start | 2026-10-22T22:49:44Z, pid 956970 |

The original `ade-preprod-lv8b` store is untouched; it remains the LV-8 evidence store.

## 2. Recovery

The store opened at semantics 11 and recovered forward from its durable tip, with nothing reset:

```
recovery-trace: path=recovery_admit action=forward_fold reason=forward_fold_no_reset anchor_before=133574403/5170128/4a2e1872 durable_tip=133902215/5184854/9bb45a4c rollback_target=none anchor_after=133574403/5170128/4a2e1872
```

The durable tip is the parent of block 5,184,855 (LV-10 CE-LV10-1).

## 3. Past the block

The first follow-tip line after recovery was already past the failing slot:

```
follow: tip slot=133937493 …
…
follow: tip slot=133994313 behind=3033698 slots from peer
follow: tip slot=134005635 behind=3022376 slots from peer
```

The node can only extend its chain from 133,902,215 through the block whose `prev_hash` is that tip,
and on the peer's chain that block is 5,184,855 at 133,902,219. Its follow tip moving past it therefore
means the block was decoded, validated and admitted.

The log also shows LV-8's same-pass retry passing many times (`checkpoint behind the walk at …`, each
followed by a `bnd2c-transition`). That is the intended behaviour, not a new failure.

## 4. Bounds

This shows one thing: the node no longer stops on a negative `script_n_of_k` threshold, and the old store
resumes under the new rules. It says nothing about native-script verdict enforcement (no production path
reads the verdict; LV-11 §7), and nothing about stack depth (LV-12).
