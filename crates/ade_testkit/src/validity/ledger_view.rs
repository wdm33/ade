// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN helper: build a production `PoolDistrView` from the committed
//! Conway-576 corpus for the B1-S2 acceptance tests.
//!
//! Non-authoritative test infrastructure. It performs the one normalization the
//! corpus requires: `cardano-cli` emits each pool's `individualPoolStake` as a
//! *reduced* fraction, so per-pool denominators differ. They all divide the
//! shared `pdTotalActiveStake`, so the common-denominator active stake is
//! `numer * (pd_total / denom)` — exact integer arithmetic, no float.

use std::collections::BTreeMap;

use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
use ade_types::{EpochNo, Hash28, Hash32};

use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;

use super::corpus::ConwayValidityCorpus;

/// Failure to project the corpus onto a common-denominator `PoolDistrView`.
#[derive(Debug, PartialEq, Eq)]
pub enum CorpusViewError {
    /// A pool's reduced `sigma` denominator does not divide `pdTotalActiveStake`,
    /// so it cannot be normalized to the shared total without losing precision.
    DenomDoesNotDivideTotal {
        pool_id: [u8; 28],
        denom: u64,
        total: u64,
    },
    /// A pool carries a zero denominator (degenerate corpus).
    ZeroDenom { pool_id: [u8; 28] },
}

impl std::fmt::Display for CorpusViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CorpusViewError::DenomDoesNotDivideTotal {
                pool_id,
                denom,
                total,
            } => write!(
                f,
                "pool {} denom {denom} does not divide total {total}",
                hex28(pool_id)
            ),
            CorpusViewError::ZeroDenom { pool_id } => {
                write!(f, "pool {} has zero denom", hex28(pool_id))
            }
        }
    }
}

impl std::error::Error for CorpusViewError {}

fn hex28(b: &[u8; 28]) -> String {
    let mut s = String::with_capacity(56);
    for byte in b {
        s.push_str(&format!("{byte:02x}"));
    }
    s
}

/// Build a single-epoch `PoolDistrView` for `epoch` from the corpus.
///
/// Each pool's active stake is normalized to the shared `pd_total_active_stake`
/// denominator. Errors if any reduced denominator fails to divide the total.
pub fn pool_distr_view_from_corpus(
    corpus: &ConwayValidityCorpus,
    epoch: EpochNo,
) -> Result<PoolDistrView, CorpusViewError> {
    let total = corpus.pd_total_active_stake;
    let asc = ActiveSlotsCoeff {
        numer: corpus.asc.numer as u32,
        denom: corpus.asc.denom as u32,
    };

    let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
    for (pool_id, p) in &corpus.pools {
        if p.sigma.denom == 0 {
            return Err(CorpusViewError::ZeroDenom { pool_id: *pool_id });
        }
        if total % p.sigma.denom != 0 {
            return Err(CorpusViewError::DenomDoesNotDivideTotal {
                pool_id: *pool_id,
                denom: p.sigma.denom,
                total,
            });
        }
        let scale = total / p.sigma.denom;
        let active_stake = p.sigma.numer * scale;
        pools.insert(
            Hash28(*pool_id),
            PoolEntry {
                active_stake,
                vrf_keyhash: Hash32(p.vrf_keyhash),
            },
        );
    }

    Ok(PoolDistrView::new(epoch, total, asc, pools))
}
