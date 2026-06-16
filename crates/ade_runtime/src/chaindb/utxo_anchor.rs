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

fn anchor_err<E: std::fmt::Display>(e: E) -> ChainDbError {
    ChainDbError::Corruption(format!("utxo anchor: {e}"))
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

    /// Apply one block's accepted deltas in a SINGLE write transaction: remove the
    /// spent inputs, insert the produced outputs, commit. Atomic — a crash leaves
    /// the old anchor valid OR the new anchor valid, never a half-applied state.
    pub(crate) fn commit_block(
        &self,
        spent: &[TxIn],
        produced: &[(TxIn, TxOut)],
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
        txn.commit().map_err(anchor_err)?;
        Ok(())
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
    use std::collections::BTreeMap;
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
            anchor.commit_block(spent, produced).expect("commit");
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
                .commit_block(&[], &[(txin(0xab, 3), out(42, 0xab))])
                .expect("commit");
        }
        let reopened = UtxoAnchor::create(&path).expect("reopen");
        assert_eq!(reopened.read(&txin(0xab, 3)).expect("read"), Some(out(42, 0xab)));
        assert_eq!(reopened.len().expect("len"), 1);
    }
}
