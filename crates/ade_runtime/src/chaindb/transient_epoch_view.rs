// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! EPOCH-CONSENSUS-VIEW Slice 1: the transient (GREEN, non-authoritative)
//! disk-backed replay-window lifecycle over the dormant redb [`UtxoAnchor`].
//!
//! ===========================================================================
//! BINDING CLASSIFICATION (GREEN / non-authoritative):
//! Transient replay storage is GREEN execution support. It may accelerate or
//! enable bounded materialization, but it may NOT survive as authority,
//! influence BLUE outputs directly, or become a fallback source for follow,
//! forge, recovery, or snapshot activation. It is RED-spawned execution support
//! governed by a GREEN contract: never an authoritative output, never outliving
//! its window.
//! ===========================================================================
//!
//! Three NET-NEW surfaces (the rest is wiring over the existing anchor):
//! - **D1** [`transient_root`] — a fixed, owned subtree under the node's data
//!   root (`<data root>/transient-epoch-view/`). Derived from the existing
//!   storage config, NEVER under WAL / snapshots / ChainDb / any durable-artifact
//!   directory. No runtime `--transient-view-dir` flag exists. A test-only
//!   override is permitted ONLY through [`transient_root_for_test`], which is
//!   `#[cfg(test)]` and therefore impossible to reach in a production build.
//! - **D2** [`window_key`] — a deterministic Blake2b key (no rand / uuid):
//!   `epochview-window-<hex(blake2b(network ‖ era ‖ epoch ‖ point ‖
//!   commitment))>.redb`. [`is_valid_window_key`] is the exact validator.
//! - **D3** [`purge_transient_root`] — fail-closed purge-on-startup: enumerate
//!   ONLY the owned subtree, validate every candidate is a valid D2 key, delete
//!   all, fsync the parent, continue ONLY when empty. ANY failure (delete /
//!   dir-fsync / name-validation) is a STRUCTURED TERMINAL failure — never
//!   best-effort, never continue with stale material.
//!
//! redb's default durability is `Immediate` (fsync-per-commit, checksummed dual
//! slots, auto-repair on reopen); this lifecycle NEVER weakens it for the
//! transient anchor.

#![allow(dead_code)] // GREEN execution support spawned only inside a bounded proof context

use std::fs;
use std::path::{Path, PathBuf};

use super::error::ChainDbError;
use super::utxo_anchor::{AnchorPosition, UtxoAnchor};

/// The single owned subtree name. A FIXED constant, never a CLI/runtime flag.
pub const TRANSIENT_SUBTREE: &str = "transient-epoch-view";

/// The deterministic window-key prefix and suffix (the D2 form).
const KEY_PREFIX: &str = "epochview-window-";
const KEY_SUFFIX: &str = ".redb";
/// The hex digest is `blake2b_256` => 32 bytes => 64 lowercase hex chars.
const KEY_HEX_LEN: usize = 64;

/// A structured, TERMINAL failure of the transient-store lifecycle. A transient
/// store is by definition not authority and not resumable, so every recovery is
/// unconditional purge — these are halt conditions, never best-effort skips.
#[derive(Debug)]
pub enum TransientViewError {
    /// The owned subtree could not be enumerated/created (purge precondition).
    EnumerateFailed { kind: std::io::ErrorKind },
    /// A candidate name in the owned subtree is NOT a valid deterministic D2
    /// window key. The subtree is owned, so a foreign name means something other
    /// than this lifecycle wrote there — halt rather than blindly delete it.
    ForeignArtifact { name: String },
    /// A candidate window key could not be deleted during purge.
    DeleteFailed { name: String, kind: std::io::ErrorKind },
    /// The parent directory could not be `fsync`'d after deletion (the deletion
    /// is not durable until the directory entry is synced).
    DirSyncFailed { kind: std::io::ErrorKind },
    /// After purge the subtree is not empty — refuse to continue with stale
    /// transient material lying around.
    NotEmptyAfterPurge { remaining: usize },
    /// The underlying redb anchor surfaced a storage error.
    Anchor(ChainDbError),
}

impl std::fmt::Display for TransientViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransientViewError::EnumerateFailed { kind } => {
                write!(f, "transient-epoch-view: cannot enumerate the owned subtree: {kind:?}")
            }
            TransientViewError::ForeignArtifact { name } => write!(
                f,
                "transient-epoch-view: foreign artifact {name:?} in the owned subtree \
                 (not a valid deterministic window key) -- refusing to continue"
            ),
            TransientViewError::DeleteFailed { name, kind } => {
                write!(f, "transient-epoch-view: cannot delete {name:?}: {kind:?}")
            }
            TransientViewError::DirSyncFailed { kind } => write!(
                f,
                "transient-epoch-view: cannot fsync the parent directory after purge: {kind:?}"
            ),
            TransientViewError::NotEmptyAfterPurge { remaining } => write!(
                f,
                "transient-epoch-view: subtree still holds {remaining} entr(ies) after purge"
            ),
            TransientViewError::Anchor(e) => write!(f, "transient-epoch-view anchor: {e}"),
        }
    }
}

impl std::error::Error for TransientViewError {}

impl From<ChainDbError> for TransientViewError {
    fn from(e: ChainDbError) -> Self {
        TransientViewError::Anchor(e)
    }
}

/// **D1** — the fixed, owned transient subtree, derived from the node's existing
/// data root (the `--snapshot-dir` data root that already holds `chain.db`).
///
/// `<data_root>/transient-epoch-view/`. NEVER under WAL / snapshots / ChainDb /
/// any directory scanned for durable artifacts: it is a sibling subtree the
/// durable-artifact scanners never look in. There is deliberately NO
/// `--transient-view-dir` flag — a configurable consensus-adjacent lifecycle
/// would invite semantic divergence.
pub fn transient_root(data_root: &Path) -> PathBuf {
    data_root.join(TRANSIENT_SUBTREE)
}

/// Test-only override of the transient root. `#[cfg(test)]` so it is IMPOSSIBLE
/// to reach in a production build — it is not a runtime/semantic feature flag.
#[cfg(test)]
pub fn transient_root_for_test(explicit: &Path) -> PathBuf {
    explicit.join(TRANSIENT_SUBTREE)
}

/// **D2** — the deterministic window key (Blake2b, never random).
///
/// `epochview-window-<hex(blake2b(network ‖ era ‖ epoch ‖ source_chain_point ‖
/// checkpoint_commitment))>.redb`. The bindings are exactly the design record's
/// "bound-activation-only" set, so the store's identity is tied to the view it
/// forms (a correct distribution against the wrong fork/epoch/nonce is inert).
/// No `rand`/`uuid` dependency exists.
pub fn window_key(
    network: u32,
    era: u16,
    epoch: u64,
    source_chain_point: &[u8],
    checkpoint_commitment: &[u8],
) -> String {
    let mut preimage = Vec::new();
    preimage.extend_from_slice(&network.to_be_bytes());
    preimage.extend_from_slice(&era.to_be_bytes());
    preimage.extend_from_slice(&epoch.to_be_bytes());
    // Length-prefix the variable-length bindings so distinct (point, commitment)
    // splits cannot alias to the same preimage.
    preimage.extend_from_slice(&(source_chain_point.len() as u64).to_be_bytes());
    preimage.extend_from_slice(source_chain_point);
    preimage.extend_from_slice(&(checkpoint_commitment.len() as u64).to_be_bytes());
    preimage.extend_from_slice(checkpoint_commitment);
    let digest = ade_crypto::blake2b_256(&preimage);
    format!("{KEY_PREFIX}{digest}{KEY_SUFFIX}")
}

/// The exact D2 validator: `epochview-window-` ++ 64 lowercase hex ++ `.redb`.
/// Used by the fail-closed purge to distinguish an own window key (deletable)
/// from a foreign artifact (halt). Anything not matching this exact shape is
/// rejected — the validation is the gate, not a heuristic.
pub fn is_valid_window_key(name: &str) -> bool {
    let Some(rest) = name.strip_prefix(KEY_PREFIX) else {
        return false;
    };
    let Some(hex) = rest.strip_suffix(KEY_SUFFIX) else {
        return false;
    };
    hex.len() == KEY_HEX_LEN && hex.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// `fsync` a directory entry so a deletion/creation within it is durable.
fn fsync_dir(dir: &Path) -> Result<(), std::io::ErrorKind> {
    let handle = fs::File::open(dir).map_err(|e| e.kind())?;
    handle.sync_all().map_err(|e| e.kind())
}

/// **D3** — fail-closed purge-on-startup. Run BEFORE any materialization.
///
/// 1. enumerate ONLY the owned `transient-epoch-view/` subtree;
/// 2. verify every candidate name is a valid deterministic window key (D2);
/// 3. delete all candidates;
/// 4. `fsync` the parent directory;
/// 5. continue ONLY when the subtree is empty.
///
/// Any failure (deletion, directory `fsync`, or name validation) is a STRUCTURED
/// TERMINAL failure. A transient store is not authority and not resumable, so
/// recovery is unconditional purge, never reconcile.
pub fn purge_transient_root(root: &Path) -> Result<(), TransientViewError> {
    // The subtree is owned; ensure it exists so enumeration is well-defined.
    fs::create_dir_all(root).map_err(|e| TransientViewError::EnumerateFailed { kind: e.kind() })?;

    let entries = fs::read_dir(root).map_err(|e| TransientViewError::EnumerateFailed { kind: e.kind() })?;

    // First pass: validate EVERY name before deleting ANY — a foreign artifact
    // halts the purge with nothing destroyed (never blindly delete the unknown).
    let mut candidates: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| TransientViewError::EnumerateFailed { kind: e.kind() })?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !is_valid_window_key(&name) {
            return Err(TransientViewError::ForeignArtifact { name });
        }
        candidates.push(entry.path());
    }

    // Second pass: delete every validated own window key.
    for path in &candidates {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        fs::remove_file(path).map_err(|e| TransientViewError::DeleteFailed { name, kind: e.kind() })?;
    }

    // The deletion is not durable until the directory entry is synced.
    fsync_dir(root).map_err(|kind| TransientViewError::DirSyncFailed { kind })?;

    // Continue ONLY when the subtree is empty.
    let remaining = fs::read_dir(root)
        .map_err(|e| TransientViewError::EnumerateFailed { kind: e.kind() })?
        .count();
    if remaining != 0 {
        return Err(TransientViewError::NotEmptyAfterPurge { remaining });
    }
    Ok(())
}

/// A bounded, disk-backed, transient replay window over the dormant redb
/// [`UtxoAnchor`]. GREEN execution support: it accelerates bounded
/// materialization but is NEVER authority and NEVER outlives its window.
///
/// Lifecycle: [`open`](Self::open) (after a fail-closed purge) → materialize on
/// disk → iterate (the aggregation-pass shape) → [`dispose`](Self::dispose).
pub struct TransientEpochViewStore {
    /// The owned window file path inside the transient subtree.
    path: PathBuf,
    /// The parent (owned) subtree, fsync'd on dispose.
    root: PathBuf,
    /// The dormant redb anchor as the transient substrate (`Immediate`
    /// durability, never weakened).
    anchor: UtxoAnchor,
}

impl TransientEpochViewStore {
    /// Open a transient window store at `root/<window_key>`. The caller is
    /// responsible for having run [`purge_transient_root`] first (fail-closed):
    /// `open` itself ensures the owned subtree exists but does not re-purge.
    pub fn open(root: &Path, key: &str) -> Result<Self, TransientViewError> {
        debug_assert!(
            is_valid_window_key(key),
            "a transient window is opened only under a valid deterministic D2 key"
        );
        fs::create_dir_all(root).map_err(|e| TransientViewError::EnumerateFailed { kind: e.kind() })?;
        let path = root.join(key);
        let anchor = UtxoAnchor::create(&path)?;
        Ok(TransientEpochViewStore {
            path,
            root: root.to_path_buf(),
            anchor,
        })
    }

    /// Materialize one batch of the replay window's UTxO on disk (the
    /// create→materialize step), stamping the window position atomically with
    /// the delta (the existing anchor invariant — never half-applied).
    pub fn materialize_batch(
        &self,
        produced: &[(ade_types::tx::TxIn, ade_ledger::utxo::TxOut)],
        position: &AnchorPosition,
    ) -> Result<(), TransientViewError> {
        self.anchor.commit_block(&[], produced, position)?;
        Ok(())
    }

    /// The number of live entries materialized on disk (`UtxoAnchor::len()`).
    pub fn len(&self) -> Result<u64, TransientViewError> {
        Ok(self.anchor.len()?)
    }

    /// Whether the window is empty.
    pub fn is_empty(&self) -> Result<bool, TransientViewError> {
        Ok(self.len()? == 0)
    }

    /// Iterate the materialized window in canonical order (the aggregation-pass
    /// shape). RED execution support, never a BLUE input.
    pub fn iter_window(
        &self,
    ) -> Result<Vec<(ade_types::tx::TxIn, ade_ledger::utxo::TxOut)>, TransientViewError> {
        Ok(self.anchor.iter_sorted()?)
    }

    /// The on-disk byte count of the window file (release-tier evidence only).
    pub fn on_disk_bytes(&self) -> u64 {
        fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0)
    }

    /// Dispose of the transient window: drop the redb handle, remove the file,
    /// and `fsync` the owned subtree so the removal is durable. Consumes `self`
    /// so the store can never be used past disposal.
    pub fn dispose(self) -> Result<(), TransientViewError> {
        let TransientEpochViewStore { path, root, anchor } = self;
        // Drop the db handle (close the file) before removing it.
        drop(anchor);
        match fs::remove_file(&path) {
            Ok(()) => {}
            // Already gone (e.g. a crash removed it) is a clean dispose, not a
            // failure — the postcondition (the window file is absent) holds.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                return Err(TransientViewError::DeleteFailed { name, kind: e.kind() });
            }
        }
        fsync_dir(&root).map_err(|kind| TransientViewError::DirSyncFailed { kind })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ade_types::address::Address;
    use ade_types::tx::{Coin, TxIn};
    use ade_types::Hash32;
    use tempfile::TempDir;

    fn txin(h: u8, i: u16) -> TxIn {
        TxIn {
            tx_hash: Hash32([h; 32]),
            index: i,
        }
    }
    fn out(c: u64, t: u8) -> ade_ledger::utxo::TxOut {
        ade_ledger::utxo::TxOut::Byron {
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

    /// **D1** — the transient root is the owned `transient-epoch-view/` subtree
    /// of the data root, and is NEVER the WAL / snapshot / ChainDb directory.
    #[test]
    fn transient_root_is_owned_subtree_of_data_root() {
        let data_root = Path::new("/var/lib/ade/data");
        let root = transient_root(data_root);
        assert_eq!(root, Path::new("/var/lib/ade/data/transient-epoch-view"));
        assert!(root.starts_with(data_root), "the subtree is under the data root");
        // It is a SIBLING of chain.db / the WAL dir, never a child of them.
        assert_ne!(root, data_root.join("chain.db"));
        assert!(!root.ends_with("wal"));
        assert!(!root.to_string_lossy().contains("snapshot"));
    }

    /// **D2** — the window key is deterministic (same bindings => same key), is
    /// distinct for distinct bindings, and never overflows length-prefix aliases.
    #[test]
    fn window_key_is_deterministic_and_binding_sensitive() {
        let k1 = window_key(1, 7, 1331, b"point-a", b"commit-a");
        let k2 = window_key(1, 7, 1331, b"point-a", b"commit-a");
        assert_eq!(k1, k2, "same bindings => byte-identical key (no rand/uuid)");
        assert!(is_valid_window_key(&k1), "the produced key is its own valid form");

        // Each binding independently changes the key.
        assert_ne!(k1, window_key(2, 7, 1331, b"point-a", b"commit-a"), "network");
        assert_ne!(k1, window_key(1, 8, 1331, b"point-a", b"commit-a"), "era");
        assert_ne!(k1, window_key(1, 7, 1332, b"point-a", b"commit-a"), "epoch");
        assert_ne!(k1, window_key(1, 7, 1331, b"point-b", b"commit-a"), "chain point");
        assert_ne!(k1, window_key(1, 7, 1331, b"point-a", b"commit-b"), "commitment");

        // Length-prefixing prevents (point ‖ commitment) split aliasing.
        assert_ne!(
            window_key(1, 7, 1331, b"ab", b"c"),
            window_key(1, 7, 1331, b"a", b"bc"),
            "a boundary shift between the two variable bindings must change the key"
        );
    }

    /// **D2 validator** — accepts exactly the deterministic form; rejects a
    /// foreign name, a wrong length, uppercase hex, a wrong suffix, a wrong
    /// prefix, and non-hex.
    #[test]
    fn window_key_validator_accepts_only_the_deterministic_form() {
        let good = window_key(764824073, 7, 1331, b"pt", b"cm");
        assert!(is_valid_window_key(&good));
        assert!(!is_valid_window_key("chain.db"), "a durable artifact name");
        assert!(!is_valid_window_key("wal-0000.bin"), "a WAL file name");
        assert!(!is_valid_window_key("epochview-window-tooshort.redb"));
        assert!(
            !is_valid_window_key(&format!("epochview-window-{}.redb", "A".repeat(64))),
            "uppercase hex is not the canonical lowercase form"
        );
        assert!(
            !is_valid_window_key(&format!("epochview-window-{}.redb", "z".repeat(64))),
            "non-hex chars rejected"
        );
        assert!(
            !is_valid_window_key(&format!("epochview-window-{}.txt", "a".repeat(64))),
            "wrong suffix"
        );
        assert!(
            !is_valid_window_key(&format!("other-prefix-{}.redb", "a".repeat(64))),
            "wrong prefix"
        );
    }

    /// **D3** — a valid-named leftover window is purged: the D3 sequence empties
    /// the owned subtree fail-closed and continues only when empty.
    #[test]
    fn purge_removes_valid_named_leftovers_and_leaves_subtree_empty() {
        let tmp = TempDir::new().expect("tempdir");
        let root = transient_root_for_test(tmp.path());
        fs::create_dir_all(&root).expect("mkroot");
        // Two valid leftovers (e.g. a prior window that crashed before dispose).
        let k1 = window_key(1, 7, 1, b"p1", b"c1");
        let k2 = window_key(1, 7, 2, b"p2", b"c2");
        fs::write(root.join(&k1), b"stale").expect("write k1");
        fs::write(root.join(&k2), b"stale").expect("write k2");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 2);

        purge_transient_root(&root).expect("purge succeeds on valid leftovers");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0, "subtree empty after purge");
    }

    /// **D3** — a foreign/invalid name in the owned subtree is a STRUCTURED
    /// TERMINAL failure: nothing is deleted (never blindly), the error names the
    /// artifact, and a valid sibling is left intact (the purge halted first).
    #[test]
    fn purge_fails_closed_on_a_foreign_artifact_and_deletes_nothing() {
        let tmp = TempDir::new().expect("tempdir");
        let root = transient_root_for_test(tmp.path());
        fs::create_dir_all(&root).expect("mkroot");
        let valid = window_key(1, 7, 1, b"p", b"c");
        fs::write(root.join(&valid), b"valid").expect("write valid");
        fs::write(root.join("intruder.dat"), b"foreign").expect("write foreign");

        let err = purge_transient_root(&root).expect_err("must fail closed");
        assert!(
            matches!(&err, TransientViewError::ForeignArtifact { name } if name == "intruder.dat"),
            "the terminal error names the foreign artifact, got {err}"
        );
        // Nothing was deleted — the purge halted before any removal.
        assert!(root.join(&valid).exists(), "the valid sibling is untouched");
        assert!(root.join("intruder.dat").exists(), "the foreign artifact is NOT blindly deleted");
    }

    /// A fresh (or already-empty) subtree purges cleanly.
    #[test]
    fn purge_is_clean_on_an_empty_or_absent_subtree() {
        let tmp = TempDir::new().expect("tempdir");
        let root = transient_root_for_test(tmp.path());
        // Absent: create + empty.
        purge_transient_root(&root).expect("absent subtree purges clean");
        // Empty: idempotent.
        purge_transient_root(&root).expect("empty subtree purges clean");
        assert!(root.exists() && fs::read_dir(&root).unwrap().count() == 0);
    }

    /// Full lifecycle: open → materialize → iterate → dispose. `len()==N` on
    /// disk, the iteration is the materialized set, and dispose removes the file
    /// leaving the owned subtree empty.
    #[test]
    fn lifecycle_open_materialize_iterate_dispose() {
        let tmp = TempDir::new().expect("tempdir");
        let root = transient_root_for_test(tmp.path());
        purge_transient_root(&root).expect("purge");
        let key = window_key(1, 7, 1331, b"src-point", b"ckpt-commit");

        let store = TransientEpochViewStore::open(&root, &key).expect("open");
        let produced = vec![
            (txin(0x01, 0), out(100, 1)),
            (txin(0x02, 0), out(200, 2)),
            (txin(0x03, 7), out(300, 3)),
        ];
        store.materialize_batch(&produced, &pos(1, 0x01)).expect("materialize");
        assert_eq!(store.len().expect("len"), 3, "all N entries on disk");
        assert!(store.on_disk_bytes() > 0, "the window file has on-disk bytes");

        let window = store.iter_window().expect("iter");
        assert_eq!(window.len(), 3);
        // Canonical order: txin(0x01) < txin(0x02) < txin(0x03).
        assert_eq!(window[0].0, txin(0x01, 0));
        assert_eq!(window[2].0, txin(0x03, 7));

        store.dispose().expect("dispose");
        assert!(!root.join(&key).exists(), "the window file is gone after dispose");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0, "subtree empty after dispose");
    }

    /// Dispose is clean even if the window file was already removed (a crash
    /// between materialize and dispose) — the postcondition (file absent) holds.
    #[test]
    fn dispose_is_clean_when_window_already_removed() {
        let tmp = TempDir::new().expect("tempdir");
        let root = transient_root_for_test(tmp.path());
        purge_transient_root(&root).expect("purge");
        let key = window_key(1, 7, 1, b"p", b"c");
        let store = TransientEpochViewStore::open(&root, &key).expect("open");
        store.materialize_batch(&[(txin(0xaa, 0), out(1, 1))], &pos(1, 0x01)).expect("mat");
        // Simulate the file vanishing out from under the store.
        fs::remove_file(root.join(&key)).expect("manual remove");
        store.dispose().expect("dispose tolerates an already-absent file");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    }
}
