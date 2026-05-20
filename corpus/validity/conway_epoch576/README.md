# Conway epoch-576 positive block-validity corpus

Canonical, committed inputs for PHASE4-B1 full-block-validity agreement
(`block_validity` positive corpus, replayed by B1-S6). Extracted once on
**2026-05-20**; the multi-GB S3 sources were deleted after extraction and are
**not** committed.

## Contents

- `blocks/slot_*.cbor` — 13 files holding **14** real mainnet Conway block
  envelopes (`[era_tag=7, block]`). One file (`slot_163900768.cbor`) carries two
  concatenated envelopes; the loader splits per envelope.
- `consensus_inputs.json` — the leadership-relevant inputs for epoch 576.
- this README.

## Provenance

### Blocks

Source: `s3://ade-corpus-snapshots/extracts/blocks_epoch577.tar.gz` (91 KB).

**Misnomer (load-bearing):** the tarball is named `blocks_epoch577` but holds
blocks at slots **163,900,639 – 163,900,784**, which are the **tail of mainnet
epoch 576** (epoch 577 begins at slot 163,900,800). The corpus epoch is
therefore **576**, and the matching epoch nonce is **eta0(576)**. Using
eta0(577) here would be wrong.

The 14 blocks (sorted by slot):

| slot | block_no | issuer pool_id (blake2b-224 of issuer vkey) | file |
|---|---|---|---|
| 163900639 | 12270962 | `24cc639f00790ec41c830464376b21df8602f4e7a1591ac005f8a44f` | `slot_163900639.cbor` |
| 163900647 | 12270963 | `8bf11b20369617d452af97a060247fbc361c69a7e3e7e96b03d97a42` | `slot_163900647.cbor` |
| 163900651 | 12270964 | `c0d70a23601c6a880e7a730774c0a4005e49f0eb52ea38d1e609df0d` | `slot_163900651.cbor` |
| 163900674 | 12270965 | `38ee2f8d878f7c444bd80d38c3b4cff02d3a3477c9e77d262065d9ad` | `slot_163900674.cbor` |
| 163900680 | 12270966 | `37c7da28cacc3cddc181de02a2a957396f1aebaecf89ce6b55cd1e46` | `slot_163900680.cbor` |
| 163900721 | 12270967 | `3b3327c0a885ba7c1ebeec8b44158aab79c32148d45b4c701344cd97` | `slot_163900721.cbor` |
| 163900725 | 12270968 | `abacadaba9f12a8b5382fc370e4e7e69421fb59831bb4ecca3a11d9b` | `slot_163900725.cbor` |
| 163900727 | 12270969 | `4511bcca3896bcb6d43b66fb89fe079f00f44bed505c9ca232704dc9` | `slot_163900727.cbor` |
| 163900728 | 12270970 | `4e0356a826bbf3e0525cd4612061a1304710b0c8d12df5457ac17cdf` | `slot_163900728.cbor` |
| 163900740 | 12270971 | `ac5ba151fae6889e25021acc723b0e34655828bf3328b6f1180e1d65` | `slot_163900740.cbor` |
| 163900747 | 12270972 | `b6f7db82bc0ef7491d36bc4dd2b1be43529b04e16d7a6e6137abaad7` | `slot_163900747.cbor` |
| 163900763 | 12270973 | `1ec2c7dde1ce090ee2e75c9fa618c575b8a0b77eaeec5c7266f50aca` | `slot_163900763.cbor` |
| 163900768 | 12270974 | `2a8294ad7538b15353b9ffd81e26dafe846ffc3f6b9e331d4c1dc030` | `slot_163900768.cbor` |
| 163900784 | 12270975 | `9b97ad981c8456110e17b1297a58961147209b7dc0d8e78b44ed8bcb` | `slot_163900768.cbor` |

All are Conway-era headers (array(10), Babbage-style combined VRF). Decoded with
`ade_codec::conway::decode_conway_block`; `pool_id = ade_crypto::blake2b_224(issuer_vkey)`.

### Epoch nonce — eta0(576)

```
d4d5f9dc027133f27b2551711c078e7d34575daba4aa0ee7c82e4b9dd9f55c51
```

Source: `s3://ade-corpus-snapshots/snapshots/snapshot_163468813.tar.gz :: ./state`
(epoch-576 protocol state). The `state` is a UTxO-HD `utxohd-mem`
`ExtLedgerState` CBOR. Scanning it for the byte pattern `82 01 5820 <32B>` (a
non-neutral `Nonce = [1, bytes .size 32]`) yields **exactly 5** matches at the
file tail — the `PraosState` nonces in record order
`[evolving, candidate, epoch, lab, lastEpochBlock]`. The 3rd is eta0. For epoch
576, `evolving == candidate` (`a8145a95…`), a post-boundary-reseed corroboration.

This value is **pinned** here; it is not re-derived at test time. The reusable
scanner is `ade_ledger::consensus_input_extract::extract_praos_nonces`, exercised
in `crates/ade_ledger/src/consensus_input_extract.rs` on synthetic fixtures (the
1.78 GB snapshot is not committed and not re-downloaded for tests).

### Per-pool VRF keyhash + sigma

Source: `s3://ade-corpus-snapshots/extracts/ledger_state_conway576.json`
(1.8 GB `cardano-cli query ledger-state` NewEpochState JSON; **deleted** after
extraction).

Extraction path (pretty-printed JSON, line-scoped streaming — never loaded whole
into RAM):

```
.stakeDistrib.pdTotalActiveStake          -> total active stake (denominator base)
.stakeDistrib.unPoolDistr["<pool_id>"]    -> per pool:
    .individualPoolStake.{numerator,denominator}   -> sigma fraction
    .individualPoolStakeVrf                         -> 32-byte VRF keyhash
```

(The leadership pool distribution lives under `stakeDistrib.unPoolDistr`; this
cardano-cli version names it that rather than `nesPd`/`pd`. `cardano-cli` reduces
each `individualPoolStake` fraction, so per-pool denominators differ; the stored
`sigma` preserves the reduced fraction as emitted.)

Cross-check (proves the join is correct): for every block, the header
`vrf_vkey` satisfies `blake2b_256(vrf_vkey) == unPoolDistr[pool].individualPoolStakeVrf`.
Verified for all 14 issuing pools.

`pd_total_active_stake = 21879134170304901` is recorded for reference.

### Active-slots coefficient

`active_slots_coeff = 1/20`. This is the mainnet constant since Shelley; it is a
**genesis** parameter and is **not** present in the NewEpochState ledger-state
dump, so it is taken from the documented mainnet constant rather than read from
the dump.

## Schema — `consensus_inputs.json`

```jsonc
{
  "network": "mainnet",
  "epoch": 576,
  "epoch_nonce": "<eta0(576), 32B hex>",
  "active_slots_coeff": { "numer": 1, "denom": 20 },
  "pd_total_active_stake": 21879134170304901,
  "pools": [
    { "pool_id":   "<28B hex>",
      "vrf_keyhash": "<32B hex>",
      "sigma":     { "numer": <u64>, "denom": <u64> } },
    ...
  ],
  "era_schedule_ref": "mainnet"
}
```

Loaded by `ade_testkit::validity::ConwayValidityCorpus::load`.
Self-consistency asserted by
`crates/ade_ledger/tests/consensus_input_extract.rs::corpus_loads_and_is_self_consistent`.
