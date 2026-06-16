//! MEM-OPT-UTXO-DISK S2b: the on-disk redb UTxO anchor (RED persistent storage
//! authority).
//!
//! Stores `TxIn -> TxOut` as `fixed-width key -> canonical TxOut bytes`
//! ([`super::utxo_key`] + `ade_ledger::snapshot::encode_tx_out_canonical`). This is
//! the STORAGE backend, and it deliberately does **not** implement `UtxoStore` — the
//! S2b guardrail: BLUE validation never holds a storage backend; it consumes a
//! resolved in-memory view. The RED shell pre-resolves required inputs from here
//! into an in-memory working-set (a `BTreeMap`, which IS a `UtxoStore`) before BLUE
//! runs, and commits accepted deltas back here atomically (one write transaction
//! per block — old anchor valid OR new anchor valid, never half-applied).

#![allow(dead_code)] // wired into admission (pre-resolve + commit) in the next S2b step

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};

use ade_ledger::snapshot::{decode_tx_out_canonical, encode_tx_out_canonical};
use ade_ledger::utxo::TxOut;
use ade_types::tx::TxIn;

use super::error::ChainDbError;
use super::utxo_key::{decode_utxo_key, encode_utxo_key, UTXO_KEY_LEN};

/// `TxIn (fixed-width key) -> canonical TxOut bytes`. redb iterates keys in byte-
/// sorted order, which equals canonical `TxIn` order by the fixed-width key
/// construction (DC-MEM-06).
const UTXO_TABLE: TableDefinition<&[u8; UTXO_KEY_LEN], &[u8]> =
    TableDefinition::new("utxo_anchor");

/// `anchor_position -> encoded AnchorPosition`, written in the SAME write-txn as the
/// UTxO delta so the materialized position is always consistent with the committed
/// UTxO state (atomic; never half-applied).
const ANCHOR_META: TableDefinition<&str, &[u8]> = TableDefinition::new("anchor_meta");
const ANCHOR_POSITION_KEY: &str = "anchor_position";

fn anchor_err<E: std::fmt::Display>(e: E) -> ChainDbError {
    ChainDbError::Corruption(format!("utxo anchor: {e}"))
}

/// The block the anchor has MATERIALIZED up to — written atomically with each delta
/// (MEM-OPT-UTXO-DISK S2b-2c.1a) so recovery can reconcile the anchor to the WAL (the
/// admit authority). All four fields mirror the WAL `AdmitBlock`, so a reconciliation
/// is an exact identity comparison, not a heuristic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AnchorPosition {
    pub slot: u64,
    pub block_hash: [u8; 32],
    pub prior_fp: [u8; 32],
    pub post_fp: [u8; 32],
}

/// Fixed-width encoding: slot(8 BE) || block_hash(32) || prior_fp(32) || post_fp(32).
const ANCHOR_POSITION_LEN: usize = 8 + 32 + 32 + 32;

impl AnchorPosition {
    fn encode(&self) -> [u8; ANCHOR_POSITION_LEN] {
        let mut buf = [0u8; ANCHOR_POSITION_LEN];
        buf[..8].copy_from_slice(&self.slot.to_be_bytes());
        buf[8..40].copy_from_slice(&self.block_hash);
        buf[40..72].copy_from_slice(&self.prior_fp);
        buf[72..].copy_from_slice(&self.post_fp);
        buf
    }

    fn decode(bytes: &[u8]) -> Result<Self, ChainDbError> {
        if bytes.len() != ANCHOR_POSITION_LEN {
            return Err(ChainDbError::Corruption(format!(
                "anchor position must be {ANCHOR_POSITION_LEN} bytes, found {}",
                bytes.len()
            )));
        }
        let mut slot = [0u8; 8];
        slot.copy_from_slice(&bytes[..8]);
        let mut block_hash = [0u8; 32];
        block_hash.copy_from_slice(&bytes[8..40]);
        let mut prior_fp = [0u8; 32];
        prior_fp.copy_from_slice(&bytes[40..72]);
        let mut post_fp = [0u8; 32];
        post_fp.copy_from_slice(&bytes[72..]);
        Ok(AnchorPosition {
            slot: u64::from_be_bytes(slot),
            block_hash,
            prior_fp,
            post_fp,
        })
    }
}

/// One WAL `AdmitBlock` point, projected to the fields reconciliation compares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WalPoint {
    pub slot: u64,
    pub block_hash: [u8; 32],
    pub post_fp: [u8; 32],
}

/// The deterministic restart decision when reconciling the anchor to the WAL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RecoveryDecision {
    /// The anchor is at the WAL tail; nothing to do.
    Consistent,
    /// The anchor is behind the WAL; replay the WAL entries from index `replay_from`
    /// (inclusive) into the anchor (re-validate + `commit_block`) to roll forward.
    RollForward { replay_from: usize },
    /// The anchor is inconsistent with the WAL (ahead, or a post_fp/identity mismatch
    /// at a shared position). Halt — never silently reconcile.
    FailClosed { reason: String },
}

/// Reconcile the anchor's materialized position to the WAL chain (the admit
/// authority). Pure + deterministic. The anchor is ALWAYS ≤ the WAL (it commits
/// strictly AFTER the WAL append), so an anchor position absent from the WAL chain is
/// "ahead or diverged" → fail closed.
pub(crate) fn reconcile(position: Option<&AnchorPosition>, wal: &[WalPoint]) -> RecoveryDecision {
    match position {
        None => {
            // A fresh anchor (no committed block): replay the whole WAL.
            if wal.is_empty() {
                RecoveryDecision::Consistent
            } else {
                RecoveryDecision::RollForward { replay_from: 0 }
            }
        }
        Some(p) => match wal
            .iter()
            .position(|w| w.post_fp == p.post_fp && w.block_hash == p.block_hash && w.slot == p.slot)
        {
            Some(i) if i + 1 == wal.len() => RecoveryDecision::Consistent,
            Some(i) => RecoveryDecision::RollForward { replay_from: i + 1 },
            None => RecoveryDecision::FailClosed {
                reason: format!(
                    "anchor position (slot {}) is not on the WAL chain -- ahead or diverged",
                    p.slot
                ),
            },
        },
    }
}

/// The on-disk UTxO anchor. NOT a `UtxoStore` (the guardrail) — a RED storage
/// authority the shell reads from / commits to.
pub(crate) struct UtxoAnchor {
    db: Database,
}

impl UtxoAnchor {
    /// Open (or create) the anchor database at `path`.
    pub(crate) fn create(path: &Path) -> Result<Self, ChainDbError> {
        let db = Database::create(path).map_err(anchor_err)?;
        Ok(UtxoAnchor { db })
    }

    /// Resolve a single input from disk (the pre-resolve primitive). `None` if the
    /// output is absent. A stored value that fails to decode is a deterministic
    /// corruption error — never a silent miss.
    pub(crate) fn read(&self, tx_in: &TxIn) -> Result<Option<TxOut>, ChainDbError> {
        let key = encode_utxo_key(tx_in);
        let txn = self.db.begin_read().map_err(anchor_err)?;
        let table = match txn.open_table(UTXO_TABLE) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(anchor_err(e)),
        };
        match table.get(&key).map_err(anchor_err)? {
            Some(v) => {
                let out = decode_tx_out_canonical(v.value())
                    .map_err(|e| ChainDbError::Corruption(format!("utxo anchor TxOut: {e:?}")))?;
                Ok(Some(out))
            }
            None => Ok(None),
        }
    }

    /// Apply one block's accepted deltas AND the new position in a SINGLE write
    /// transaction: remove spent, insert produced, stamp `position`, commit. Atomic —
    /// a crash leaves the old anchor (+ old position) valid OR the new anchor (+ new
    /// position) valid, never a half-applied state and never a delta whose position
    /// disagrees with it. The position is written AFTER the WAL append (the admit), so
    /// the anchor never leads the WAL.
    pub(crate) fn commit_block(
        &self,
        spent: &[TxIn],
        produced: &[(TxIn, TxOut)],
        position: &AnchorPosition,
    ) -> Result<(), ChainDbError> {
        let txn = self.db.begin_write().map_err(anchor_err)?;
        {
            let mut table = txn.open_table(UTXO_TABLE).map_err(anchor_err)?;
            for tx_in in spent {
                let key = encode_utxo_key(tx_in);
                table.remove(&key).map_err(anchor_err)?;
            }
            for (tx_in, tx_out) in produced {
                let key = encode_utxo_key(tx_in);
                let val = encode_tx_out_canonical(tx_out);
                table.insert(&key, val.as_slice()).map_err(anchor_err)?;
            }
        }
        {
            let mut meta = txn.open_table(ANCHOR_META).map_err(anchor_err)?;
            meta.insert(ANCHOR_POSITION_KEY, &position.encode()[..])
                .map_err(anchor_err)?;
        }
        txn.commit().map_err(anchor_err)?;
        Ok(())
    }

    /// The block the anchor is materialized up to (`None` on a fresh anchor). Read
    /// back for the restart reconciliation against the WAL tail.
    pub(crate) fn read_position(&self) -> Result<Option<AnchorPosition>, ChainDbError> {
        let txn = self.db.begin_read().map_err(anchor_err)?;
        let meta = match txn.open_table(ANCHOR_META) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(anchor_err(e)),
        };
        match meta.get(ANCHOR_POSITION_KEY).map_err(anchor_err)? {
            Some(v) => Ok(Some(AnchorPosition::decode(v.value())?)),
            None => Ok(None),
        }
    }

    /// The whole anchor in canonical `TxIn` order (the checkpoint / equivalence
    /// oracle — RED, never the per-block hot path). Proves redb's byte-sorted
    /// iteration equals canonical `TxIn` order via the fixed-width key.
    pub(crate) fn iter_sorted(&self) -> Result<Vec<(TxIn, TxOut)>, ChainDbError> {
        let txn = self.db.begin_read().map_err(anchor_err)?;
        let table = match txn.open_table(UTXO_TABLE) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(e) => return Err(anchor_err(e)),
        };
        let mut out = Vec::new();
        for entry in table.iter().map_err(anchor_err)? {
            let (k, v) = entry.map_err(anchor_err)?;
            let tx_in = decode_utxo_key(k.value())
                .map_err(|e| ChainDbError::Corruption(format!("utxo anchor key: {e}")))?;
            let tx_out = decode_tx_out_canonical(v.value())
                .map_err(|e| ChainDbError::Corruption(format!("utxo anchor TxOut: {e:?}")))?;
            out.push((tx_in, tx_out));
        }
        Ok(out)
    }

    /// Resolve the pre-resolve required set from the anchor into the `WorkingSet`
    /// seed (the ONLY place the anchor is read for validation). An entry absent from
    /// the anchor is simply omitted — it is either intra-block-produced (seeded into
    /// the working-set during block application) or genuinely missing (caught
    /// fail-closed by validation, never a late disk read). BLUE never sees the anchor.
    pub(crate) fn resolve_required(
        &self,
        required: &BTreeSet<TxIn>,
    ) -> Result<BTreeMap<TxIn, TxOut>, ChainDbError> {
        let mut resolved = BTreeMap::new();
        for tx_in in required {
            if let Some(tx_out) = self.read(tx_in)? {
                resolved.insert(tx_in.clone(), tx_out);
            }
        }
        Ok(resolved)
    }

    /// The number of live anchor entries.
    pub(crate) fn len(&self) -> Result<u64, ChainDbError> {
        let txn = self.db.begin_read().map_err(anchor_err)?;
        let table = match txn.open_table(UTXO_TABLE) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(0),
            Err(e) => return Err(anchor_err(e)),
        };
        table.len().map_err(anchor_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ade_ledger::snapshot::encode_utxo_state;
    use ade_ledger::utxo::UTxOState;
    use ade_types::address::Address;
    use ade_types::tx::Coin;
    use ade_types::Hash32;
    use tempfile::TempDir;

    fn txin(h: u8, i: u16) -> TxIn {
        TxIn {
            tx_hash: Hash32([h; 32]),
            index: i,
        }
    }
    fn out(c: u64, t: u8) -> TxOut {
        TxOut::Byron {
            address: Address::Byron(vec![t]),
            coin: Coin(c),
        }
    }
    fn pos(slot: u64, h: u8) -> AnchorPosition {
        AnchorPosition {
            slot,
            block_hash: [h; 32],
            prior_fp: [h.wrapping_sub(1); 32],
            post_fp: [h; 32],
        }
    }
    fn wal_point(slot: u64, h: u8) -> WalPoint {
        WalPoint {
            slot,
            block_hash: [h; 32],
            post_fp: [h; 32],
        }
    }

    /// The load-bearing S2b DC-MEM-05 proof (storage level): apply the SAME
    /// (spent, produced) per-block deltas to a BTreeMap anchor AND the redb anchor,
    /// and after every block assert: same resolved value for every key, same
    /// canonical ordered iteration, and same canonical UTxO-state encoding (hence
    /// same fingerprint). The redb anchor is a pure storage substitution.
    #[test]
    fn redb_anchor_equals_btreemap_across_block_deltas() {
        let tmp = TempDir::new().expect("tempdir");
        let anchor = UtxoAnchor::create(&tmp.path().join("utxo.redb")).expect("create");
        let mut model: BTreeMap<TxIn, TxOut> = BTreeMap::new();

        // (spent, produced) per block.
        type Block = (Vec<TxIn>, Vec<(TxIn, TxOut)>);
        let blocks: Vec<Block> = vec![
            (vec![], vec![(txin(0x01, 0), out(100, 1)), (txin(0x02, 0), out(200, 2))]),
            (vec![txin(0x01, 0)], vec![(txin(0x03, 7), out(300, 3)), (txin(0x02, 1), out(50, 9))]),
            (vec![txin(0x02, 0), txin(0x03, 7)], vec![(txin(0xff, 256), out(7, 0xaa))]),
            (vec![], vec![(txin(0x10, 0), out(1, 0x10)), (txin(0x10, 1), out(2, 0x11))]),
        ];

        for (i, (spent, produced)) in blocks.iter().enumerate() {
            anchor
                .commit_block(spent, produced, &pos(i as u64, i as u8))
                .expect("commit");
            for s in spent {
                model.remove(s);
            }
            for (k, v) in produced {
                model.insert(k.clone(), v.clone());
            }

            // (1) resolved-value equivalence for every live key + a known-absent key.
            for (k, v) in &model {
                assert_eq!(
                    anchor.read(k).expect("read").as_ref(),
                    Some(v),
                    "resolved value mismatch at block {i} key {k:?}"
                );
            }
            assert_eq!(anchor.read(&txin(0xee, 0)).expect("read"), None);

            // (2) canonical ordered iteration equivalence (DC-MEM-06 in situ).
            let anchor_set = anchor.iter_sorted().expect("iter");
            let model_set: Vec<_> = model.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            assert_eq!(anchor_set, model_set, "ordered iteration mismatch at block {i}");
            assert_eq!(anchor.len().expect("len"), model.len() as u64);

            // (3) canonical UTxO-state encoding equivalence (=> fingerprint-identical).
            let from_anchor = encode_utxo_state(&UTxOState::from_map(anchor_set.into_iter().collect()));
            let from_model = encode_utxo_state(&UTxOState::from_map(model.clone()));
            assert_eq!(from_anchor, from_model, "snapshot encoding mismatch at block {i}");
        }
    }

    /// Reopening the anchor database sees the committed state (durability).
    #[test]
    fn anchor_survives_reopen() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("utxo.redb");
        {
            let anchor = UtxoAnchor::create(&path).expect("create");
            anchor
                .commit_block(&[], &[(txin(0xab, 3), out(42, 0xab))], &pos(7, 0x07))
                .expect("commit");
        }
        let reopened = UtxoAnchor::create(&path).expect("reopen");
        assert_eq!(reopened.read(&txin(0xab, 3)).expect("read"), Some(out(42, 0xab)));
        assert_eq!(reopened.len().expect("len"), 1);
        // the position survives the reopen too (atomic with the delta).
        assert_eq!(reopened.read_position().expect("pos"), Some(pos(7, 0x07)));
    }

    /// `resolve_required` reads the present inputs into the WorkingSet seed and OMITS
    /// the absent ones (intra-block-produced or genuinely missing) — it never
    /// fabricates an entry and never fails on an absent input (that is validation's
    /// job, fail-closed). This is the sole anchor-read path for validation.
    #[test]
    fn resolve_required_reads_present_omits_absent() {
        let tmp = TempDir::new().expect("tempdir");
        let anchor = UtxoAnchor::create(&tmp.path().join("u.redb")).expect("create");
        anchor
            .commit_block(
                &[],
                &[(txin(0x01, 0), out(100, 1)), (txin(0x02, 0), out(200, 2))],
                &pos(1, 0x01),
            )
            .expect("commit");
        let required: BTreeSet<TxIn> = [txin(0x01, 0), txin(0x99, 0)].into_iter().collect();
        let resolved = anchor.resolve_required(&required).expect("resolve");
        assert_eq!(resolved.get(&txin(0x01, 0)), Some(&out(100, 1)), "present input resolved");
        assert!(
            !resolved.contains_key(&txin(0x99, 0)),
            "absent input omitted (not fabricated, not an error)"
        );
        assert_eq!(resolved.len(), 1);
    }

    #[test]
    fn anchor_position_round_trips_and_rejects_wrong_length() {
        let p = pos(123_456, 0xa5);
        assert_eq!(AnchorPosition::decode(&p.encode()).expect("decode"), p);
        assert!(AnchorPosition::decode(&[0u8; 10]).is_err(), "wrong length fails closed");
    }

    /// The delta and the position are written in ONE redb txn — they are always
    /// committed together (a torn commit cannot apply the delta without the position,
    /// or advance the position without the delta).
    #[test]
    fn commit_stamps_position_atomically_with_the_delta() {
        let tmp = TempDir::new().expect("tempdir");
        let anchor = UtxoAnchor::create(&tmp.path().join("u.redb")).expect("create");
        assert_eq!(anchor.read_position().expect("pos"), None, "fresh anchor has no position");

        anchor
            .commit_block(&[], &[(txin(0x01, 0), out(10, 1))], &pos(5, 0x05))
            .expect("commit");
        assert_eq!(anchor.read_position().expect("pos"), Some(pos(5, 0x05)));
        assert_eq!(anchor.read(&txin(0x01, 0)).expect("read"), Some(out(10, 1)));

        anchor
            .commit_block(&[txin(0x01, 0)], &[(txin(0x02, 0), out(20, 2))], &pos(6, 0x06))
            .expect("commit");
        assert_eq!(anchor.read_position().expect("pos"), Some(pos(6, 0x06)), "position advanced");
        assert_eq!(anchor.read(&txin(0x01, 0)).expect("read"), None, "delta applied with it");
    }

    /// The deterministic restart decision: consistent at the tail, roll forward when
    /// behind, replay-all when fresh, fail closed when ahead/diverged.
    #[test]
    fn reconcile_decides_consistent_rollforward_and_fail_closed() {
        let wal = vec![wal_point(1, 0x01), wal_point(2, 0x02), wal_point(3, 0x03)];
        assert_eq!(reconcile(None, &[]), RecoveryDecision::Consistent);
        assert_eq!(
            reconcile(None, &wal),
            RecoveryDecision::RollForward { replay_from: 0 },
            "fresh anchor replays the whole WAL"
        );
        assert_eq!(
            reconcile(Some(&pos(3, 0x03)), &wal),
            RecoveryDecision::Consistent,
            "anchor at the WAL tail"
        );
        assert_eq!(
            reconcile(Some(&pos(1, 0x01)), &wal),
            RecoveryDecision::RollForward { replay_from: 1 },
            "anchor behind -> roll forward from the next entry"
        );
        assert!(
            matches!(reconcile(Some(&pos(9, 0x09)), &wal), RecoveryDecision::FailClosed { .. }),
            "anchor position not on the WAL chain (ahead/diverged) -> fail closed"
        );
    }
}
