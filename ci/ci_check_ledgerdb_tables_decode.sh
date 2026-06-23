#!/usr/bin/env bash
# ci_check_ledgerdb_tables_decode.sh -- MITHRIL-VERIFIED-ANCHOR-IMPORT Stage 2.
#
# The native cardano-node V2 (utxohd-mem) LedgerDB `tables` MemPack TxOut decoder: a deterministic,
# fail-closed, FAITHFUL decoder for the compact (non-CBOR) TxOut values. Cardano output multi-asset
# quantities are Word64 and are kept as u64 with NO i64 cast (DC-MITHRIL-02). Every unknown tag /
# address form / script language is a structured terminal error (no opaque keep-bytes); consume-
# exactly is enforced at every boundary; endianness is explicit (no host contract); the tables decode
# is era-bound to Conway from the Stage-1 `state` (PO#2, never the tables file or a flag); and the
# whole-tables commitment is a deterministic blake2b chain over the canonically-sorted UTxO.
set -euo pipefail

MOD="crates/ade_ledger/src/ledgerdb_tables.rs"
fail() { echo "FAIL (ci_check_ledgerdb_tables_decode): $1" >&2; exit 1; }
[ -f "$MOD" ] || fail "module $MOD missing"

# (A) FAITHFUL Word64 quantity -- the asset map is u64; NO i64 cast (DC-MITHRIL-02, the i64 ceiling
# is a separate downstream release blocker, never silently truncated/saturated here).
grep -Eq "BTreeMap<AssetName, u64>" "$MOD" || fail "asset quantity must be u64 (faithful Word64)"
if grep -Eq "as i64|i64::try_from|AssetName, i64" "$MOD"; then
  fail "Stage 2 must NOT cast multi-asset quantities to i64 (DC-MITHRIL-02 faithful Word64)"
fi

# (B) fail-closed: the closed terminal error set + consume-exactly + no opaque fallback.
for v in UnexpectedEof TrailingBytes BadVarLen UnsupportedTxOutTag UnsupportedAddress UnsupportedScript UnsupportedEra; do
  grep -Eq "$v" "$MOD" || fail "fail-closed variant $v missing"
done
grep -Eq "expect_consumed" "$MOD" || fail "consume-exactly enforcement missing"

# (C) explicit endianness (no host-endianness contract): explicit LE readers + the BE hash
# reconstruction (the Addr28Extra / canonical-bytes BE->LE double-flip).
grep -Eq "from_le_bytes" "$MOD" || fail "explicit little-endian reader missing"
grep -Eq "to_be_bytes" "$MOD" || fail "the big-endian hash/serialization path missing"

# (D) PO#2 era binding: the tables decode takes the Stage-1 state era and requires Conway.
grep -Eq "state_era_index" "$MOD" || fail "tables decode must take the Stage-1 `state` era"
grep -Eq "CONWAY_ERA_INDEX" "$MOD" || fail "Conway era gate missing"

# (E) deterministic whole-tables commitment over the canonically-sorted map.
grep -Eq "decode_tables_commitment" "$MOD" || fail "whole-tables commitment entry missing"
grep -Eq "ascending .canonical. order|not in ascending" "$MOD" || fail "canonical-sort assertion missing"

# (F) hermetic tests pass (primitives, PO#1 Addr28Extra round-trip, faithful u64 i64::MAX..u64::MAX,
# 6-tag dispatch, deterministic + era-bound commitment).
cargo test -p ade_ledger --lib ledgerdb_tables --quiet >/dev/null 2>&1 \
  || fail "hermetic tables-decoder tests failed (cargo test -p ade_ledger --lib ledgerdb_tables)"

echo "OK: V2 LedgerDB tables MemPack decoder is faithful-u64 + fail-closed + explicit-endian + era-bound + deterministic (MITHRIL-VERIFIED-ANCHOR-IMPORT Stage 2)"
