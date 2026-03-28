use std::io::Read;
use std::path::Path;

use ade_types::Hash32;

use super::HarnessError;

/// A loaded snapshot: the raw oracle state bytes plus parsed metadata.
///
/// The raw CBOR bytes ARE the authoritative comparison surface.
/// `state_hash()` = `Blake2b-256(raw_cbor)` — no re-encoding,
/// the oracle bytes are preserved as the hash source.
///
/// This is the `reset_to` target for the differential harness:
/// load a snapshot, verify its hash, use it as the starting state
/// for boundary replay.
#[derive(Clone)]
pub struct LoadedSnapshot {
    /// Raw ExtLedgerState CBOR bytes — the oracle comparison surface.
    pub raw_cbor: Vec<u8>,
    /// Parsed header metadata.
    pub header: SnapshotHeader,
    /// Pre-computed state hash (Blake2b-256 of raw_cbor).
    pub state_hash: Hash32,
}

impl LoadedSnapshot {
    /// Load a snapshot from a tarball, parse its header, compute its hash.
    pub fn from_tarball(tarball_path: &std::path::Path) -> Result<Self, HarnessError> {
        let raw_cbor = extract_state_from_tarball(tarball_path)?;
        let header = parse_snapshot_header(&raw_cbor)?;
        let state_hash = compute_state_hash(&raw_cbor);
        Ok(Self {
            raw_cbor,
            header,
            state_hash,
        })
    }

    /// Build a minimal `LedgerState` from the snapshot metadata.
    ///
    /// Build a `LedgerState` from the snapshot, loading delegation data
    /// from the go snapshot into the cert_state.
    ///
    /// The UTxO set is empty (compact format impedance). Delegation state
    /// is populated from the oracle's go snapshot delegation map.
    pub fn to_ledger_state(&self) -> ade_ledger::state::LedgerState {
        use ade_ledger::state::{EpochState, LedgerState};
        use ade_ledger::utxo::UTxOState;
        use ade_types::{EpochNo, SlotNo};

        let era = telescope_to_era(self.header.telescope_length);

        // Load delegation data from the go snapshot + reward accounts
        let mut cert_state = self.load_delegation_state();

        // Overlay full registrations + reward balances from DState rewards map.
        // This provides ALL registered credentials (not just delegating ones)
        // which is needed for deltaT2 reward filtering.
        if let Ok(accounts) = parse_reward_accounts(&self.raw_cbor) {
            for (hash, reward) in &accounts {
                let mut cred_bytes = [0u8; 28];
                cred_bytes.copy_from_slice(&hash.0[..28]);
                let cred = ade_types::shelley::cert::StakeCredential(
                    ade_types::Hash28(cred_bytes),
                );
                // Add to registrations (all entries in rewards map are registered)
                cert_state.delegation.registrations
                    .entry(cred.clone())
                    .or_insert(ade_types::tx::Coin(0));
                // Set reward balance
                if *reward > 0 {
                    cert_state.delegation.rewards
                        .entry(cred)
                        .or_insert(ade_types::tx::Coin(0))
                        .0 = *reward;
                }
            }
        }

        // Load go snapshot stake distribution
        let snapshots = self.load_snapshot_state();

        LedgerState {
            utxo_state: UTxOState::new(),
            epoch_state: EpochState {
                epoch: EpochNo(self.header.epoch),
                slot: SlotNo(0),
                snapshots,
                reserves: ade_types::tx::Coin(self.header.reserves),
                treasury: ade_types::tx::Coin(self.header.treasury),
                block_production: {
                    use ade_types::tx::PoolId;
                    use ade_types::Hash28;
                    let bp = parse_block_production(&self.raw_cbor).unwrap_or_default();
                    bp.into_iter().map(|(h, count)| {
                        let mut pool_bytes = [0u8; 28];
                        pool_bytes.copy_from_slice(&h.0[..28]);
                        (PoolId(Hash28(pool_bytes)), count)
                    }).collect()
                },
                epoch_fees: ade_types::tx::Coin(self.header.epoch_fees),
            },
            protocol_params: self.load_protocol_params(era),
            era,
            track_utxo: true,
            cert_state,
            max_lovelace_supply: 45_000_000_000_000_000, // 45B ADA mainnet
        }
    }

    /// Load protocol parameters based on era and epoch.
    ///
    /// Uses known oracle values from protocol_params_oracle.toml keyed by
    /// the nearest HFC boundary. Falls back to defaults for unknown epochs.
    fn load_protocol_params(
        &self,
        era: ade_types::CardanoEra,
    ) -> ade_ledger::pparams::ProtocolParameters {
        use ade_ledger::pparams::ProtocolParameters;
        use ade_ledger::rational::Rational;
        use ade_types::tx::Coin;

        let epoch = self.header.epoch;
        let zero = || Rational::zero();

        // Base params common to all Shelley+ eras (oracle-confirmed values)
        let mut pp = ProtocolParameters {
            min_fee_a: Coin(44),
            min_fee_b: Coin(155_381),
            max_block_body_size: 65_536,
            max_tx_size: 16_384,
            max_block_header_size: 1_100,
            key_deposit: Coin(2_000_000),
            pool_deposit: Coin(500_000_000),
            e_max: 18,
            n_opt: 500,
            pool_influence: Rational::new(3, 10).unwrap_or_else(zero),
            monetary_expansion: Rational::new(3, 1000).unwrap_or_else(zero),
            treasury_growth: Rational::new(1, 5).unwrap_or_else(zero),
            protocol_major: 2,
            protocol_minor: 0,
            min_utxo_value: Coin(1_000_000),
            min_pool_cost: Coin(340_000_000),
            decentralization: Rational::new(1, 1).unwrap_or_else(zero),
        };

        // Try to parse d from the actual snapshot CBOR (ground truth).
        // Fall back to era-specific approximation if parsing fails.
        if let Ok((d_num, d_den)) = parse_decentralization_param(&self.raw_cbor) {
            if d_den > 0 {
                pp.decentralization = Rational::new(d_num as i128, d_den as i128)
                    .unwrap_or_else(zero);
            }
        }

        // Era/epoch-specific overrides from oracle (d is now from CBOR above)
        match era {
            ade_types::CardanoEra::Shelley => {
                pp.protocol_major = 2;
                // d decreases during Shelley. Approximate from epoch.
                // epoch 208 → d=1, epoch 234 → d~0.38
                let epochs_since_shelley = epoch.saturating_sub(208);
                let d_num = 25u64.saturating_sub(epochs_since_shelley);
                let d_num = d_num.min(25);
                pp.decentralization = Rational::new(d_num as i128, 25)
                    .unwrap_or_else(zero);
            }
            ade_types::CardanoEra::Allegra => {
                pp.protocol_major = 3;
                // oracle: d = 8/25 at epoch 236, d = 3/25 at epoch 251
                // d was decreased through governance proposals, not linearly.
                // Use oracle-confirmed values at known epochs, interpolate between.
                if epoch <= 236 {
                    pp.decentralization = Rational::new(8, 25).unwrap_or_else(zero);
                } else {
                    // Between epoch 236 (d=8/25) and epoch 251 (d=3/25):
                    // 5/25 decrease over 15 epochs = 1/75 per epoch
                    let epochs_past = epoch.saturating_sub(236);
                    // d = 8/25 - epochs_past/75 = (8*3 - epochs_past)/75 = (24 - epochs_past)/75
                    let d_num_75 = 24u64.saturating_sub(epochs_past);
                    pp.decentralization = Rational::new(d_num_75 as i128, 75)
                        .unwrap_or_else(zero);
                }
            }
            ade_types::CardanoEra::Mary => {
                pp.protocol_major = 4;
                // oracle: d = 3/25 at epoch 251, d = 0 at epoch 257
                // 3/25 decrease over 6 epochs = 1/50 per epoch
                if epoch <= 251 {
                    pp.decentralization = Rational::new(3, 25).unwrap_or_else(zero);
                } else if epoch >= 257 {
                    pp.decentralization = Rational::zero();
                } else {
                    let epochs_past = epoch.saturating_sub(251);
                    // d = 3/25 - epochs_past/50 = (6 - epochs_past)/50
                    let d_num_50 = 6u64.saturating_sub(epochs_past);
                    pp.decentralization = Rational::new(d_num_50 as i128, 50)
                        .unwrap_or_else(zero);
                }
            }
            ade_types::CardanoEra::Alonzo => {
                pp.protocol_major = 5;
                // oracle: d = 0 from Mary→Alonzo HFC (epoch 290)
                pp.decentralization = Rational::zero();
            }
            ade_types::CardanoEra::Babbage => {
                pp.protocol_major = 7;
                pp.max_block_body_size = 90_112; // increased in Babbage
                pp.decentralization = Rational::zero();
            }
            ade_types::CardanoEra::Conway => {
                pp.protocol_major = 9;
                pp.max_block_body_size = 90_112;
                pp.decentralization = Rational::zero();
            }
            _ => {}
        }

        pp
    }

    /// Load snapshot state (mark/set/go) from the oracle CBOR.
    ///
    /// Populates the go snapshot with stake distribution and pool stakes
    /// from the oracle. Mark and set start empty (they'd need the previous
    /// epoch's data which we don't have loaded).
    fn load_snapshot_state(&self) -> ade_ledger::epoch::SnapshotState {
        use ade_ledger::epoch::{
            GoSnapshot, MarkSnapshot, SetSnapshot, SnapshotState, StakeSnapshot,
        };

        let mark = self.build_stake_snapshot(0)
            .unwrap_or_else(|_| StakeSnapshot::new());
        let set = self.build_stake_snapshot(1)
            .unwrap_or_else(|_| StakeSnapshot::new());
        let go = self.build_stake_snapshot(2)
            .unwrap_or_else(|_| StakeSnapshot::new());

        SnapshotState {
            mark: MarkSnapshot(mark),
            set: SetSnapshot(set),
            go: GoSnapshot(go),
        }
    }

    /// Build a StakeSnapshot from a specific snapshot position (0=mark, 1=set, 2=go).
    fn build_stake_snapshot(
        &self,
        snapshot_index: u32,
    ) -> Result<ade_ledger::epoch::StakeSnapshot, HarnessError> {
        use ade_types::tx::{Coin, PoolId};
        use ade_types::Hash28;
        use std::collections::BTreeMap;

        let stake_entries = parse_snapshot_stake_distribution(&self.raw_cbor, snapshot_index)?;
        let delegations_raw = parse_snapshot_delegations(&self.raw_cbor, snapshot_index)?;

        // Build stake lookup
        let mut stake_map: BTreeMap<[u8; 28], u64> = BTreeMap::new();
        for (cred_hash, stake) in &stake_entries {
            let mut key = [0u8; 28];
            key.copy_from_slice(&cred_hash.0[..28]);
            stake_map.insert(key, *stake);
        }

        // Combine delegation + stake
        let mut delegations: BTreeMap<Hash28, (PoolId, Coin)> = BTreeMap::new();
        let mut pool_stakes: BTreeMap<PoolId, Coin> = BTreeMap::new();

        for (cred_hash, pool_hash) in &delegations_raw {
            let mut cred_bytes = [0u8; 28];
            cred_bytes.copy_from_slice(&cred_hash.0[..28]);
            let mut pool_bytes = [0u8; 28];
            pool_bytes.copy_from_slice(&pool_hash.0[..28]);

            let stake = stake_map.get(&cred_bytes).copied().unwrap_or(0);
            let pool = PoolId(Hash28(pool_bytes));

            delegations.insert(Hash28(cred_bytes), (pool.clone(), Coin(stake)));

            let entry = pool_stakes.entry(pool).or_insert(Coin(0));
            entry.0 = entry.0.saturating_add(stake);
        }

        Ok(ade_ledger::epoch::StakeSnapshot {
            delegations,
            pool_stakes,
        })
    }

    /// Load delegation and pool state from the go snapshot.
    fn load_delegation_state(&self) -> ade_ledger::delegation::CertState {
        use ade_ledger::delegation::{CertState, DelegationState, PoolState, PoolParams};
        use ade_types::shelley::cert::StakeCredential;
        use ade_types::tx::{Coin, PoolId};
        use ade_types::Hash28;
        use std::collections::BTreeMap;

        let delegations_raw = match parse_go_delegations(&self.raw_cbor) {
            Ok(d) => d,
            Err(_) => return CertState::new(),
        };

        let pools_raw = parse_go_pool_params(&self.raw_cbor).unwrap_or_default();

        // Build delegation state
        let mut delegations = BTreeMap::new();
        let mut registrations = BTreeMap::new();

        for (cred_hash, pool_hash) in &delegations_raw {
            let mut cred_bytes = [0u8; 28];
            cred_bytes.copy_from_slice(&cred_hash.0[..28]);
            let cred = StakeCredential(Hash28(cred_bytes));

            let mut pool_bytes = [0u8; 28];
            pool_bytes.copy_from_slice(&pool_hash.0[..28]);
            let pool = PoolId(Hash28(pool_bytes));

            delegations.insert(cred.clone(), pool);
            registrations.insert(cred, Coin(0));
        }

        // Build pool state
        let mut pools = BTreeMap::new();
        for (pool_hash, pledge, cost, margin_num, margin_den, reward_acct, owner_hashes) in &pools_raw {
            let mut pool_bytes = [0u8; 28];
            pool_bytes.copy_from_slice(&pool_hash.0[..28]);
            let pool_id = PoolId(Hash28(pool_bytes));

            pools.insert(pool_id.clone(), PoolParams {
                pool_id,
                vrf_hash: ade_types::Hash32([0u8; 32]),
                pledge: Coin(*pledge),
                cost: Coin(*cost),
                margin: (*margin_num, *margin_den),
                reward_account: reward_acct.clone(),
                owners: owner_hashes.iter()
                    .map(|h| ade_types::Hash28(*h))
                    .collect(),
            });
        }

        CertState {
            delegation: DelegationState {
                registrations,
                delegations,
                rewards: BTreeMap::new(),
            },
            pool: PoolState {
                pools,
                retiring: BTreeMap::new(),
            },
        }
    }
}

fn telescope_to_era(telescope_length: u32) -> ade_types::CardanoEra {
    use ade_types::CardanoEra;
    match telescope_length {
        1 => CardanoEra::ByronRegular,
        2 => CardanoEra::Shelley,
        3 => CardanoEra::Allegra,
        4 => CardanoEra::Mary,
        5 => CardanoEra::Alonzo,
        6 => CardanoEra::Babbage,
        7 => CardanoEra::Conway,
        _ => CardanoEra::Conway,
    }
}

/// Parsed header of an ExtLedgerState snapshot.
///
/// Extracted from the CBOR structure without fully deserializing
/// the multi-hundred-megabyte state. Provides enough information
/// to identify the era, epoch, and telescope position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotHeader {
    /// Number of eras in the telescope (1 = Byron only, 5 = Alonzo, etc.)
    pub telescope_length: u32,
    /// Era index of the current state (0-based position in telescope)
    pub current_era_index: u32,
    /// Epoch number from the NewEpochState
    pub epoch: u64,
    /// Treasury in lovelace (from AccountState)
    pub treasury: u64,
    /// Reserves in lovelace (from AccountState)
    pub reserves: u64,
    /// Accumulated fees from the epoch (SnapShots[3]).
    pub epoch_fees: u64,
    /// Total size of the state CBOR in bytes
    pub state_size: usize,
}

/// Extract the `state` file from a snapshot tarball.
///
/// Each snapshot tarball contains `./state` (ExtLedgerState CBOR),
/// `./tables/tvar` (ImmutableDB), and `./meta`.
pub fn extract_state_from_tarball(tarball_path: &Path) -> Result<Vec<u8>, HarnessError> {
    let file = std::fs::File::open(tarball_path)
        .map_err(|e| HarnessError::IoError(format!("{}: {e}", tarball_path.display())))?;

    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    for entry in archive
        .entries()
        .map_err(|e| HarnessError::IoError(format!("tar entries: {e}")))?
    {
        let mut entry =
            entry.map_err(|e| HarnessError::IoError(format!("tar entry: {e}")))?;

        let path = entry
            .path()
            .map_err(|e| HarnessError::IoError(format!("tar path: {e}")))?
            .to_path_buf();

        if path.file_name().and_then(|n| n.to_str()) == Some("state") {
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| HarnessError::IoError(format!("read state: {e}")))?;
            return Ok(buf);
        }
    }

    Err(HarnessError::ParseError(
        "state file not found in tarball".to_string(),
    ))
}

/// Parse the CBOR header of an ExtLedgerState to extract era/epoch metadata.
///
/// Does NOT fully deserialize the state — only reads enough structure
/// to identify the telescope length and current era's epoch number.
pub fn parse_snapshot_header(state_cbor: &[u8]) -> Result<SnapshotHeader, HarnessError> {
    let state_size = state_cbor.len();

    // outer: array(2) [era_index_or_state, ...]
    let (off, outer_len) = read_array_header(state_cbor, 0)?;

    if outer_len != 2 {
        return Err(HarnessError::ParseError(format!(
            "expected outer array(2), got array({outer_len})"
        )));
    }

    // Element 0: uint — era index in NS encoding
    let (off, _era_idx) = read_uint(state_cbor, off)?;

    // Element 1: array(2) [telescope, header_state]
    let (off, pair_len) = read_array_header(state_cbor, off)?;
    if pair_len != 2 {
        return Err(HarnessError::ParseError(format!(
            "expected state pair array(2), got array({pair_len})"
        )));
    }

    // Telescope: array(N) where N = number of eras
    let (off, telescope_length) = read_array_header(state_cbor, off)?;

    // Skip past eras (small) to reach current era (last element)
    let mut off = off;
    for _ in 0..telescope_length - 1 {
        off = skip_cbor(state_cbor, off)?;
    }

    // Current era: array(2) [Bound, State]
    let (off, _) = read_array_header(state_cbor, off)?;
    // Skip Bound
    let off = skip_cbor(state_cbor, off)?;

    // State: array(2) [version_uint, payload]
    let (off, _) = read_array_header(state_cbor, off)?;
    // Skip version
    let off = skip_cbor(state_cbor, off)?;

    // Payload: array(3) [WithOrigin, NewEpochState, Transition]
    let (off, _) = read_array_header(state_cbor, off)?;
    // Skip WithOrigin
    let off = skip_cbor(state_cbor, off)?;

    // NewEpochState: array(N) — first element is epoch uint
    let (off, _nes_len) = read_array_header(state_cbor, off)?;

    // NES[0]: epoch number
    let (off, epoch) = read_uint(state_cbor, off)?;

    // Skip NES[1] (nesBprev), NES[2] (nesBcur) to reach NES[3] (EpochState)
    let off = skip_cbor(state_cbor, off)?; // NES[1]
    let off = skip_cbor(state_cbor, off)?; // NES[2]

    // NES[3] = EpochState = array(4) [AccountState, LedgerState, SnapShots, NonMyopic]
    let (off, _) = read_array_header(state_cbor, off)?;

    // ES[0] = AccountState = array(2) [treasury, reserves]
    let (off, acct_len) = read_array_header(state_cbor, off)?;
    let (treasury, reserves) = if acct_len == 2 {
        let (off, treasury) = read_uint(state_cbor, off)?;
        let (_, reserves) = read_uint(state_cbor, off)?;
        (treasury, reserves)
    } else {
        (0, 0)
    };

    // Parse epoch fees separately (re-navigating is simpler than tracking offset)
    let epoch_fees = parse_epoch_fees(state_cbor).unwrap_or(0);

    Ok(SnapshotHeader {
        telescope_length,
        current_era_index: telescope_length - 1,
        epoch,
        treasury,
        reserves,
        epoch_fees,
        state_size,
    })
}

fn parse_epoch_fees(state_cbor: &[u8]) -> Result<u64, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;

    // Skip ES[0] (AccountState), ES[1] (LedgerState) to reach ES[2] (SnapShots)
    let off = skip_cbor(state_cbor, es)?; // ES[0]
    let off = skip_cbor(state_cbor, off)?; // ES[1]

    // ES[2] = SnapShots = array(4) [mark, set, go, fees]
    let (ss_inner, _) = read_array_header(state_cbor, off)?;
    let off = skip_cbor(state_cbor, ss_inner)?; // skip mark
    let off = skip_cbor(state_cbor, off)?; // skip set
    let off = skip_cbor(state_cbor, off)?; // skip go

    // SS[3] = epoch fees
    let (_, fees) = read_uint(state_cbor, off)?;
    Ok(fees)
}

/// Compute the Blake2b-256 hash of a snapshot state file.
///
/// This is the oracle comparison surface: `Blake2b-256(encodeDiskExtLedgerState)`.
/// The hash of the state file bytes IS the state hash at the snapshot slot.
pub fn compute_state_hash(state_cbor: &[u8]) -> Hash32 {
    ade_crypto::blake2b_256(state_cbor)
}

/// An oracle state hash entry from a hash file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleHashEntry {
    pub slot: u64,
    pub hash: Hash32,
    pub state_size: u64,
}

/// Parse an oracle hash file.
///
/// Format: `SlotNo <slot>|<hex_hash>|<state_size>` per line.
pub fn parse_oracle_hashes(content: &str) -> Result<Vec<OracleHashEntry>, HarnessError> {
    let mut entries = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.split('|').collect();
        if parts.len() < 2 {
            return Err(HarnessError::ParseError(format!(
                "line {}: expected slot|hash|size", i + 1
            )));
        }
        let slot_str = parts[0]
            .trim()
            .strip_prefix("SlotNo ")
            .unwrap_or(parts[0].trim());
        let slot: u64 = slot_str
            .parse()
            .map_err(|e| HarnessError::ParseError(format!("line {}: bad slot: {e}", i + 1)))?;

        let hex = parts[1].trim();
        if hex.len() != 64 {
            return Err(HarnessError::ParseError(format!(
                "line {}: hash must be 64 hex chars, got {}", i + 1, hex.len()
            )));
        }
        let mut hash_bytes = [0u8; 32];
        for j in 0..32 {
            hash_bytes[j] = u8::from_str_radix(&hex[j * 2..j * 2 + 2], 16)
                .map_err(|e| HarnessError::ParseError(format!("line {}: bad hex: {e}", i + 1)))?;
        }

        let state_size = if parts.len() >= 3 {
            parts[2]
                .trim()
                .parse()
                .unwrap_or(0)
        } else {
            0
        };

        entries.push(OracleHashEntry {
            slot,
            hash: Hash32(hash_bytes),
            state_size,
        });
    }
    Ok(entries)
}

/// Parse the go snapshot's pool params, stake distribution, and delegation
/// from the ExtLedgerState CBOR.
///
/// Returns (pool_count, stake_entry_count, delegation_entry_count).
/// This is the T-21B state-load bridge: loading delegation/pool data
/// from the oracle's on-disk format into usable counts and structures.
pub fn parse_go_snapshot_counts(state_cbor: &[u8]) -> Result<(usize, usize, usize), HarnessError> {
    // Navigate to ES[2] = SnapShots
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;

    // ES[2] = SnapShots = array(4) [mark, set, go, fee_total]
    let ss_off = skip_n_fields(state_cbor, es, 2)?; // skip ES[0], ES[1]
    let (ss_inner, _) = read_array_header(state_cbor, ss_off)?;

    // SS[2] = go snapshot = array(3) [stake_dist, delegations, pool_params]
    let go_off = skip_n_fields(state_cbor, ss_inner, 2)?; // skip mark, set
    let (go_inner, go_len) = read_array_header(state_cbor, go_off)?;
    if go_len != 3 {
        return Err(HarnessError::ParseError(format!(
            "go snapshot expected array(3), got array({go_len})"
        )));
    }

    // Count entries in each map
    let stake_count = count_indef_map(state_cbor, go_inner)?;
    let deleg_off = skip_cbor(state_cbor, go_inner)?;
    let deleg_count = count_indef_map(state_cbor, deleg_off)?;
    let pool_off = skip_cbor(state_cbor, deleg_off)?;
    let pool_count = count_indef_map(state_cbor, pool_off)?;

    Ok((pool_count, stake_count, deleg_count))
}

/// Parse the go snapshot's delegation map into a Vec of (credential_hash, pool_hash) pairs.
/// Parse the MIR (InstantaneousRewards) total from CertState[0].
///
/// Returns the total lovelace of pending MIR transfers from reserves.
/// These are applied at the epoch boundary, adding to reward accounts
/// and decreasing reserves independently of the reward pot.
pub fn parse_mir_total(
    state_cbor: &[u8],
) -> Result<u64, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;

    // ES[0] = AccountState (skip)
    let off = skip_cbor(state_cbor, es)?;
    // ES[1] = LedgerState = array(2)
    let (off, _) = read_array_header(state_cbor, off)?;
    // Skip UTxOState
    let off = skip_cbor(state_cbor, off)?;
    // CertState = array(6)
    let (off, _) = read_array_header(state_cbor, off)?;
    // CS[0] = irwd/MIR map
    let (mut co, _, _) = read_cbor_initial(state_cbor, off)?;
    let mut total = 0u64;
    while co < state_cbor.len() && state_cbor[co] != 0xff {
        co = skip_cbor(state_cbor, co)?; // skip key
        let (_, major, val) = read_cbor_initial(state_cbor, co)?;
        if major == 0 { total = total.saturating_add(val); }
        co = skip_cbor(state_cbor, co)?;
    }
    Ok(total)
}

/// Parse the rewards map from CertState[4][0] (DState rewards/registrations).
///
/// Returns Vec of (credential_hash, reward_lovelace) for ALL registered
/// credentials — not just delegating ones. This is the complete registrations
/// set needed for deltaT2 reward filtering.
///
/// Key format: array(2) [type_tag(uint), hash(bytes28)]
/// Value: uint (reward balance in lovelace)
pub fn parse_reward_accounts(
    state_cbor: &[u8],
) -> Result<Vec<(Hash32, u64)>, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;

    // ES[0] = AccountState (skip)
    let off = skip_cbor(state_cbor, es)?;
    // ES[1] = LedgerState = array(2)
    let (off, _) = read_array_header(state_cbor, off)?;
    // Skip UTxOState
    let off = skip_cbor(state_cbor, off)?;
    // CertState = array(6)
    let (off, _) = read_array_header(state_cbor, off)?;
    // Skip CS[0..3] to reach CS[4]
    let off = skip_n_fields(state_cbor, off, 4)?;
    // CS[4] = array(2) [rewards_map, pool_map]
    let (off, _) = read_array_header(state_cbor, off)?;
    // CS[4][0] = rewards map (credential → coin)
    let (mut co, _, _) = read_cbor_initial(state_cbor, off)?;

    let mut accounts = Vec::new();
    while co < state_cbor.len() && state_cbor[co] != 0xff {
        // Key: array(2) [type_tag, bytes(28)]
        let (key_inner, _, _) = read_cbor_initial(state_cbor, co)?;
        let key_end = skip_cbor(state_cbor, co)?;

        // Read credential hash (skip type tag, read hash)
        let tag_end = skip_cbor(state_cbor, key_inner)?;
        let (hash_start, _, hash_len) = read_cbor_initial(state_cbor, tag_end)?;
        let mut cred_hash = [0u8; 32];
        if hash_len >= 28 {
            cred_hash[..28].copy_from_slice(&state_cbor[hash_start..hash_start + 28]);
        }
        co = key_end;

        // Value: uint (reward balance)
        let (_, reward) = read_uint(state_cbor, co)?;
        co = skip_cbor(state_cbor, co)?;

        accounts.push((Hash32(cred_hash), reward));
    }

    Ok(accounts)
}

///
/// This loads the actual delegation data needed for reward computation.
pub fn parse_go_delegations(
    state_cbor: &[u8],
) -> Result<Vec<(Hash32, Hash32)>, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;
    let ss_off = skip_n_fields(state_cbor, es, 2)?;
    let (ss_inner, _) = read_array_header(state_cbor, ss_off)?;
    let go_off = skip_n_fields(state_cbor, ss_inner, 2)?;
    let (go_inner, _) = read_array_header(state_cbor, go_off)?;

    // Skip stake_dist to get to delegation map
    let deleg_off = skip_cbor(state_cbor, go_inner)?;

    // Parse delegation map: key = array(2)[type, hash28], value = bytes(28)
    let (map_start, _, _map_val) = read_cbor_initial(state_cbor, deleg_off)?;

    let mut delegations = Vec::new();
    let mut co = map_start;

    // Iterate map entries (works for both definite and indefinite)
    while co < state_cbor.len() && state_cbor[co] != 0xff {
        // Key: array(2) [type_tag, bytes(28)]
        let (key_inner, _, _) = read_cbor_initial(state_cbor, co)?;
        let key_end = skip_cbor(state_cbor, co)?;

        // Read credential hash (skip type tag, read hash28)
        let (_, _tag) = read_uint(state_cbor, key_inner)?;
        let tag_end = skip_cbor(state_cbor, key_inner)?;
        // Read hash bytes
        let (hash_start, _, hash_len) = read_cbor_initial(state_cbor, tag_end)?;
        let mut cred_hash = [0u8; 32];
        if hash_len >= 28 {
            cred_hash[..28].copy_from_slice(&state_cbor[hash_start..hash_start + 28]);
        }
        co = key_end;

        // Value: bytes(28) = pool hash
        let (pool_start, _, pool_len) = read_cbor_initial(state_cbor, co)?;
        let mut pool_hash = [0u8; 32];
        if pool_len >= 28 {
            pool_hash[..28].copy_from_slice(&state_cbor[pool_start..pool_start + 28]);
        }
        co = skip_cbor(state_cbor, co)?;

        delegations.push((Hash32(cred_hash), Hash32(pool_hash)));
    }

    Ok(delegations)
}

/// Parse nesBprev (block production counts per pool) from NES[1].
///
/// Returns Vec of (pool_hash, blocks_produced).
/// Pools not in this map produced zero blocks.
pub fn parse_block_production(
    state_cbor: &[u8],
) -> Result<std::collections::BTreeMap<Hash32, u64>, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;

    // NES[0] = epoch, NES[1] = nesBprev
    let bprev_off = skip_cbor(state_cbor, off)?; // skip NES[0]

    let (mut co, _, _) = read_cbor_initial(state_cbor, bprev_off)?;
    let mut production = std::collections::BTreeMap::new();

    while co < state_cbor.len() && state_cbor[co] != 0xff {
        // Key: bytes(28) pool hash
        let (key_start, key_maj, key_len) = read_cbor_initial(state_cbor, co)?;
        let mut pool_hash = [0u8; 32];
        if key_maj == 2 && key_len >= 28 {
            pool_hash[..28].copy_from_slice(&state_cbor[key_start..key_start + 28]);
        }
        co = skip_cbor(state_cbor, co)?;

        // Value: uint (block count)
        let (_, blocks) = read_uint(state_cbor, co)?;
        co = skip_cbor(state_cbor, co)?;

        production.insert(Hash32(pool_hash), blocks);
    }

    Ok(production)
}

/// Parse stake distribution from the go snapshot.
///
/// Returns Vec of (credential_hash, stake_lovelace).
pub fn parse_go_stake_distribution(
    state_cbor: &[u8],
) -> Result<Vec<(Hash32, u64)>, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;
    let ss_off = skip_n_fields(state_cbor, es, 2)?;
    let (ss_inner, _) = read_array_header(state_cbor, ss_off)?;
    let go_off = skip_n_fields(state_cbor, ss_inner, 2)?;
    let (go_inner, _) = read_array_header(state_cbor, go_off)?;

    // First map in go snapshot is stake distribution
    let (mut co, _, _) = read_cbor_initial(state_cbor, go_inner)?;
    let mut stakes = Vec::new();

    while co < state_cbor.len() && state_cbor[co] != 0xff {
        // Key: array(2) [type_tag, bytes(28)]
        let (key_inner, _, _) = read_cbor_initial(state_cbor, co)?;
        let key_end = skip_cbor(state_cbor, co)?;

        let (_, _tag) = read_uint(state_cbor, key_inner)?;
        let tag_end = skip_cbor(state_cbor, key_inner)?;
        let (hash_start, _, hash_len) = read_cbor_initial(state_cbor, tag_end)?;
        let mut cred_hash = [0u8; 32];
        if hash_len >= 28 {
            cred_hash[..28].copy_from_slice(&state_cbor[hash_start..hash_start + 28]);
        }
        co = key_end;

        // Value: uint (stake in lovelace)
        let (_, stake) = read_uint(state_cbor, co)?;
        co = skip_cbor(state_cbor, co)?;

        stakes.push((Hash32(cred_hash), stake));
    }

    Ok(stakes)
}

/// Pool params tuple: (pool_hash, pledge, cost, margin_num, margin_den, reward_account, owners).
type PoolParamsTuple = (Hash32, u64, u64, u64, u64, Vec<u8>, Vec<[u8; 28]>);

/// Parse pool params from the go snapshot.
pub fn parse_go_pool_params(
    state_cbor: &[u8],
) -> Result<Vec<PoolParamsTuple>, HarnessError> {
    parse_snapshot_pool_params(state_cbor, 2)
}

/// Parse a pool params map at the given offset.
fn parse_pool_params_map(
    state_cbor: &[u8],
    pool_off: usize,
) -> Result<Vec<PoolParamsTuple>, HarnessError> {
    let (mut co, _, _) = read_cbor_initial(state_cbor, pool_off)?;
    let mut pools = Vec::new();

    while co < state_cbor.len() && state_cbor[co] != 0xff {
        // Key: bytes(28) = pool hash
        let (key_start, key_maj, key_len) = read_cbor_initial(state_cbor, co)?;
        let mut pool_hash = [0u8; 32];
        if key_maj == 2 && key_len >= 28 {
            pool_hash[..28].copy_from_slice(&state_cbor[key_start..key_start + 28]);
        }
        co = skip_cbor(state_cbor, co)?;

        // Value: array(9) [operator, vrf, pledge, cost, margin, reward_acct, ...]
        let val_start = co;
        let (val_inner, _, val_len) = read_cbor_initial(state_cbor, co)?;

        if val_len >= 6 {
            let f2 = skip_cbor(state_cbor, val_inner)?; // skip operator
            let f2 = skip_cbor(state_cbor, f2)?; // skip vrf
            let (_, pledge) = read_uint(state_cbor, f2)?;
            let f3 = skip_cbor(state_cbor, f2)?;
            let (_, cost) = read_uint(state_cbor, f3)?;
            let f4 = skip_cbor(state_cbor, f3)?;
            // margin: tag(30, array(2, [num, den]))
            let (tag_inner, _, _) = read_cbor_initial(state_cbor, f4)?;
            let (margin_inner, _) = read_array_header(state_cbor, tag_inner)?;
            let (_, margin_num) = read_uint(state_cbor, margin_inner)?;
            let den_off = skip_cbor(state_cbor, margin_inner)?;
            let (_, margin_den) = read_uint(state_cbor, den_off)?;
            let f5 = skip_cbor(state_cbor, f4)?;
            // reward_account: bytes(29)
            let (acct_start, _, acct_len) = read_cbor_initial(state_cbor, f5)?;
            let reward_acct = state_cbor[acct_start..acct_start + acct_len as usize].to_vec();

            // [6] poolOwners: set of key hashes (28 bytes each)
            // Error-tolerant: if parsing fails, return empty owners (pledge check skipped)
            let owners = if val_len >= 7 {
                let f6 = match skip_cbor(state_cbor, f5) { Ok(v) => v, Err(_) => { pools.push((Hash32(pool_hash), pledge, cost, margin_num, margin_den, reward_acct, vec![])); co = skip_cbor(state_cbor, val_start)?; continue; } };
                parse_pool_owners(state_cbor, f6).unwrap_or_default()
            } else {
                Vec::new()
            };

            pools.push((Hash32(pool_hash), pledge, cost, margin_num, margin_den, reward_acct, owners));
        }

        co = skip_cbor(state_cbor, val_start)?;
    }

    Ok(pools)
}

/// Parse pool owners from a CBOR offset pointing to the poolOwners field.
/// Returns Vec of 28-byte owner key hashes. Error-tolerant: returns empty on failure.
fn parse_pool_owners(state_cbor: &[u8], off: usize) -> Result<Vec<[u8; 28]>, HarnessError> {
    let (own_body, own_maj, own_val) = read_cbor_initial(state_cbor, off)?;
    let (arr_body, arr_maj, arr_val) = if own_maj == 6 {
        read_cbor_initial(state_cbor, own_body)?
    } else {
        (own_body, own_maj, own_val)
    };
    let mut owners = Vec::new();
    if arr_maj == 4 {
        let mut oi = arr_body;
        for _ in 0..arr_val {
            if oi >= state_cbor.len() { break; }
            let (entry_body, entry_maj, entry_len) = read_cbor_initial(state_cbor, oi)?;
            if entry_maj == 2 && entry_len >= 28 {
                let mut owner = [0u8; 28];
                owner.copy_from_slice(&state_cbor[entry_body..entry_body + 28]);
                owners.push(owner);
            } else if entry_maj == 4 && entry_len == 2 {
                let hash_off = skip_cbor(state_cbor, entry_body)?;
                let (hash_start, hash_maj, hash_len) = read_cbor_initial(state_cbor, hash_off)?;
                if hash_maj == 2 && hash_len >= 28 {
                    let mut owner = [0u8; 28];
                    owner.copy_from_slice(&state_cbor[hash_start..hash_start + 28]);
                    owners.push(owner);
                }
            }
            oi = skip_cbor(state_cbor, oi)?;
        }
    }
    Ok(owners)
}

/// Navigate from an arbitrary offset that starts at the HFC telescope.
/// Used for .bin ExtLedgerState files where the LedgerState part starts
/// at a different offset than the tarball format.
fn navigate_to_nes_from(state_cbor: &[u8], telescope_start: usize) -> Result<usize, HarnessError> {
    let (off, telescope_len) = read_array_header(state_cbor, telescope_start)?;

    let mut off = off;
    for _ in 0..telescope_len - 1 {
        off = skip_cbor(state_cbor, off)?;
    }

    let (off, _) = read_array_header(state_cbor, off)?;
    let off = skip_cbor(state_cbor, off)?;
    let (off, _) = read_array_header(state_cbor, off)?;
    let off = skip_cbor(state_cbor, off)?;
    let (off, _) = read_array_header(state_cbor, off)?;
    let off = skip_cbor(state_cbor, off)?;
    let (off, _) = read_array_header(state_cbor, off)?;
    Ok(off)
}

fn navigate_to_nes(state_cbor: &[u8]) -> Result<usize, HarnessError> {
    let (off, _) = read_array_header(state_cbor, 0)?; // outer array(2)
    let (off, _) = read_uint(state_cbor, off)?; // era index
    let (off, _) = read_array_header(state_cbor, off)?; // state pair
    let (off, telescope_len) = read_array_header(state_cbor, off)?; // telescope

    // Skip to last telescope entry
    let mut off = off;
    for _ in 0..telescope_len - 1 {
        off = skip_cbor(state_cbor, off)?;
    }

    // Current era: array(2) [Bound, State]
    let (off, _) = read_array_header(state_cbor, off)?;
    let off = skip_cbor(state_cbor, off)?; // skip Bound
    let (off, _) = read_array_header(state_cbor, off)?; // State = array(2) [version, payload]
    let off = skip_cbor(state_cbor, off)?; // skip version
    let (off, _) = read_array_header(state_cbor, off)?; // payload = array(3)
    let off = skip_cbor(state_cbor, off)?; // skip WithOrigin

    // Now at NES
    let (off, _) = read_array_header(state_cbor, off)?;
    Ok(off)
}

fn skip_nes_to_epoch_state(state_cbor: &[u8], nes_body: usize) -> Result<usize, HarnessError> {
    // Skip NES[0] (epoch), NES[1] (bprev), NES[2] (bcur) to reach NES[3] (EpochState)
    let off = skip_cbor(state_cbor, nes_body)?; // NES[0]
    let off = skip_cbor(state_cbor, off)?; // NES[1]
    let off = skip_cbor(state_cbor, off)?; // NES[2]

    // NES[3] = EpochState = array(4)
    let (off, _) = read_array_header(state_cbor, off)?;
    Ok(off)
}

/// Navigate to DState within ES[1] (LedgerState → DPState → DState).
///
/// Path: EpochState[1] = LedgerState = array(2) [UTxOState, DPState]
///       DPState = array(2) [DState, PState]  (array(3) in Conway with VState)
///       DState varies by era — may be array or map.
///
/// Returns (offset to DState body, major_type, field_count).
/// If verbose=true, prints navigation offsets for debugging.
#[cfg(test)]
fn navigate_to_dstate_verbose(
    state_cbor: &[u8],
    verbose: bool,
) -> Result<(usize, u8, u32), HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;

    // ES[0] = AccountState (skip)
    let acct_off = es;
    let off = skip_cbor(state_cbor, es)?;
    if verbose {
        let acct_size = off - acct_off;
        eprintln!("  nav: ES[0] AccountState at {acct_off} ({acct_size}B)");
    }

    // ES[1] = LedgerState
    let ls_off = off;
    let (off, ls_len) = read_array_header(state_cbor, off)?;
    if verbose {
        eprintln!("  nav: ES[1] LedgerState at {ls_off} (array({ls_len}))");
    }

    // LS[0] = UTxOState (skip — this is the huge one)
    let utxo_off = off;
    let off = skip_cbor(state_cbor, off)?;
    if verbose {
        let utxo_size = off - utxo_off;
        eprintln!("  nav: LS[0] UTxOState at {utxo_off} ({utxo_size}B)");
    }

    // LS[1] = DPState (CertState)
    let dp_off = off;
    let (dp_inner, dp_maj, dp_val) = read_cbor_initial(state_cbor, off)?;
    if verbose {
        let dp_type = match dp_maj { 4 => "array", 5 => "map", _ => "?" };
        eprintln!("  nav: LS[1] DPState at {dp_off} ({dp_type}({dp_val}))");
    }

    // If DPState is array, skip into first element (DState)
    // If DPState is something else, the layout differs
    let ds_off = if dp_maj == 4 {
        dp_inner // first element of array
    } else {
        dp_off // treat the whole thing as DState
    };

    // DState — probe type
    let (body, major, val) = read_cbor_initial(state_cbor, ds_off)?;
    let count = if val == u64::MAX { 0 } else { val as u32 };
    if verbose {
        let ds_type = match major { 4 => "array", 5 => "map", _ => "?" };
        eprintln!("  nav: DState at {ds_off} ({ds_type}({val}))");
    }

    Ok((body, major, count))
}


#[allow(dead_code)]
fn navigate_to_protocol_params(state_cbor: &[u8]) -> Result<usize, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;

    // ES[0] = AccountState (skip)
    let off = skip_cbor(state_cbor, es)?;

    // ES[1] = LedgerState = array(2) [UTxOState, DPState]
    let (off, _ls_len) = read_array_header(state_cbor, off)?;

    // UTxOState — need to find ppups/current params inside
    // UTxOState = array(N) where structure depends on era
    let (off, _utxo_len) = read_array_header(state_cbor, off)?;

    // UTxOState[0] = UTxO map (skip — huge)
    let off = skip_cbor(state_cbor, off)?;
    // UTxOState[1] = deposited (uint)
    let off = skip_cbor(state_cbor, off)?;
    // UTxOState[2] = fees (uint)
    let off = skip_cbor(state_cbor, off)?;
    // UTxOState[3] = ppups (proposed protocol parameter updates) or GovernanceState
    // This contains the current protocol parameters in some form
    Ok(off)
}

/// Diagnostic: probe the CertState (LS[1]) structure.
/// Shows all top-level fields of whatever we find at LS[1].
pub fn probe_dstate_structure(
    state_cbor: &[u8],
) -> Result<DStateProbe, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;

    // ES[0] = AccountState (skip)
    let off = skip_cbor(state_cbor, es)?;

    // ES[1] = LedgerState = array(2) [UTxOState, CertState]
    let (off, _ls_len) = read_array_header(state_cbor, off)?;
    // Skip UTxOState
    let off = skip_cbor(state_cbor, off)?;

    // LS[1] = CertState — probe its structure
    let (body, major, val) = read_cbor_initial(state_cbor, off)?;
    let count = if val == u64::MAX { 0 } else { val as u32 };

    let mut fields = Vec::new();
    let mut current = body;

    if major == 4 {
        // Array — iterate elements
        for i in 0..count {
            let (_, fmaj, fval) = read_cbor_initial(state_cbor, current)?;
            let field_size = skip_cbor(state_cbor, current)? - current;
            fields.push(DStateFieldInfo {
                index: i,
                key_info: None,
                major_type: fmaj,
                header_value: fval,
                offset: current,
                size: field_size,
            });
            current = skip_cbor(state_cbor, current)?;
        }
    } else if major == 5 {
        // Map
        let limit = if count > 0 { count } else { 100 };
        for i in 0..limit {
            if current >= state_cbor.len() || state_cbor[current] == 0xff {
                break;
            }
            let (_, kmaj, kval) = read_cbor_initial(state_cbor, current)?;
            let key_size = skip_cbor(state_cbor, current)? - current;
            let key_info = Some((kmaj, kval, key_size));
            current = skip_cbor(state_cbor, current)?;

            let (_, vmaj, vval) = read_cbor_initial(state_cbor, current)?;
            let val_size = skip_cbor(state_cbor, current)? - current;
            fields.push(DStateFieldInfo {
                index: i,
                key_info,
                major_type: vmaj,
                header_value: vval,
                offset: current,
                size: val_size,
            });
            current = skip_cbor(state_cbor, current)?;
        }
    }

    Ok(DStateProbe {
        container_type: major,
        field_count: count,
        entry_count: fields.len() as u32,
        fields,
    })
}

/// Diagnostic info about the DState structure.
#[derive(Debug)]
pub struct DStateProbe {
    pub container_type: u8, // 4=array, 5=map
    pub field_count: u32,   // from CBOR header
    pub entry_count: u32,   // actually enumerated
    pub fields: Vec<DStateFieldInfo>,
}

/// Info about a single DState field/entry.
#[derive(Debug)]
pub struct DStateFieldInfo {
    pub index: u32,
    pub key_info: Option<(u8, u64, usize)>, // (major, val, size) — only for map entries
    pub major_type: u8,
    pub header_value: u64,
    pub offset: usize,
    pub size: usize,
}

impl DStateFieldInfo {
    pub fn type_name(&self) -> &'static str {
        match self.major_type {
            0 => "uint",
            1 => "negint",
            2 => "bytes",
            3 => "text",
            4 => "array",
            5 => "map",
            6 => "tag",
            7 => "special",
            _ => "unknown",
        }
    }
}

/// Navigate to a specific snapshot position within ES[2] (SnapShots).
///
/// SnapShots = array(4) [mark, set, go, fees]
/// Each snapshot = array(3) [stake_dist, delegations, pool_params]
///
/// snapshot_index: 0=mark, 1=set, 2=go
/// Returns offset to the body of the snapshot array (first field).
fn navigate_to_snapshot(
    state_cbor: &[u8],
    snapshot_index: u32,
) -> Result<usize, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;
    // Skip ES[0] (AccountState), ES[1] (LedgerState) to reach ES[2] (SnapShots)
    let ss_off = skip_n_fields(state_cbor, es, 2)?;
    let (ss_inner, _) = read_array_header(state_cbor, ss_off)?;
    // Skip to desired snapshot position
    let snap_off = skip_n_fields(state_cbor, ss_inner, snapshot_index)?;
    let (snap_inner, snap_len) = read_array_header(state_cbor, snap_off)?;
    if snap_len != 3 {
        return Err(HarnessError::ParseError(format!(
            "snapshot[{snapshot_index}] expected array(3), got array({snap_len})"
        )));
    }
    Ok(snap_inner)
}

/// Parse stake distribution from a specific snapshot position.
///
/// snapshot_index: 0=mark, 1=set, 2=go
pub fn parse_snapshot_stake_distribution(
    state_cbor: &[u8],
    snapshot_index: u32,
) -> Result<Vec<(Hash32, u64)>, HarnessError> {
    let snap_inner = navigate_to_snapshot(state_cbor, snapshot_index)?;
    // First map in snapshot is stake distribution
    parse_credential_coin_map(state_cbor, snap_inner)
}

/// Parse delegation map from a specific snapshot position.
///
/// snapshot_index: 0=mark, 1=set, 2=go
pub fn parse_snapshot_delegations(
    state_cbor: &[u8],
    snapshot_index: u32,
) -> Result<Vec<(Hash32, Hash32)>, HarnessError> {
    let snap_inner = navigate_to_snapshot(state_cbor, snapshot_index)?;
    // Skip stake_dist to get delegation map
    let deleg_off = skip_cbor(state_cbor, snap_inner)?;
    parse_credential_hash_map(state_cbor, deleg_off)
}

/// Parse pool params from a specific snapshot position.
///
/// snapshot_index: 0=mark, 1=set, 2=go
pub fn parse_snapshot_pool_params(
    state_cbor: &[u8],
    snapshot_index: u32,
) -> Result<Vec<PoolParamsTuple>, HarnessError> {
    let snap_inner = navigate_to_snapshot(state_cbor, snapshot_index)?;
    let deleg_off = skip_cbor(state_cbor, snap_inner)?; // skip stake_dist
    let pool_off = skip_cbor(state_cbor, deleg_off)?;   // skip delegations
    parse_pool_params_map(state_cbor, pool_off)
}

/// Parse pool params from the CURRENT PState (the live registration state,
/// NOT the go snapshot).
///
/// In the Shelley spec, reward computation uses the current PState pool params,
/// not the go snapshot's pool params. Pools that changed cost/margin/pledge
/// between the snapshot epoch and the current epoch will have different params.
///
/// Path: ES[1] → LedgerState → DPState[1] → PState → pool_params map
pub fn parse_current_pool_params(
    state_cbor: &[u8],
) -> Result<Vec<PoolParamsTuple>, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;

    // ES[0] = AccountState (skip)
    let off = skip_cbor(state_cbor, es)?;
    // ES[1] = LedgerState = array(2) [UTxOState, DPState]
    let (off, _) = read_array_header(state_cbor, off)?;
    // Skip UTxOState
    let off = skip_cbor(state_cbor, off)?;
    // DPState = array(2) [DState, PState]
    let (dp_inner, _) = read_array_header(state_cbor, off)?;
    // Skip DState to reach PState
    let pstate_off = skip_cbor(state_cbor, dp_inner)?;
    // PState = array(N) where first element is pool_params map
    let (ps_inner, _) = read_array_header(state_cbor, pstate_off)?;
    // PState[0] = pool_params map (pool_hash → pool_params)
    parse_pool_params_map(state_cbor, ps_inner)
}

/// Navigate to the DState UMap within LS[0] (CertState).
///
/// On-disk layout (verified empirically):
///   LS[0] = CertState = array(3) [VState, PState, DState]  (Conway)
///   LS[1] = UTxOState = array(6) [UTxO, deposited, fees, GovState, IncrStake, donation]
///
/// Note: disk order is CertState first, UTxOState second (opposite of Haskell type).
///
/// DState = array(4) [UMap, futureGenDelegs, genDelegs, iRewards]
/// UMap = array(2) [umElems, umPtrs]
/// umElems = map(credential → UMElem)
///
/// Returns offset to the umElems map body.
fn navigate_to_umap_elems(state_cbor: &[u8]) -> Result<usize, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;

    // ES[0] = AccountState (skip)
    let off = skip_cbor(state_cbor, es)?;

    // ES[1] = LedgerState = array(2)
    let (ls_body, _) = read_array_header(state_cbor, off)?;

    // LS[0] = CertState = array(N)
    let (cs_body, cs_len) = read_array_header(state_cbor, ls_body)?;
    // Probe both LS fields to understand the on-disk layout
    let ls1_off = skip_cbor(state_cbor, ls_body)?;
    let (_, ls1_maj, ls1_val) = read_cbor_initial(state_cbor, ls1_off)?;
    let ls1_size = skip_cbor(state_cbor, ls1_off).unwrap_or(ls1_off + 1) - ls1_off;
    let ls0_size = ls1_off - ls_body;
    eprintln!("    [navigate_to_umap_elems] LS[0] = array({cs_len}) size={}KB  LS[1] = major={ls1_maj}(val={ls1_val}) size={}KB",
        ls0_size / 1000, ls1_size / 1000);

    // Navigate to DState
    let dstate_off = if cs_len == 3 {
        // Conway: skip VState, PState → DState
        let off = skip_cbor(state_cbor, cs_body)?;
        skip_cbor(state_cbor, off)?
    } else if cs_len == 6 {
        // Pre-Conway on-disk layout: CertState = array(6) = UTxOState fields
        // In this layout, LS[0] is actually UTxOState and LS[1] is CertState
        // (or both Allegra and Conway use array(6) for LS[1])
        // Need to navigate differently — go to LS[1] instead
        return navigate_to_umap_elems_via_ls1(state_cbor, ls_body);
    } else {
        // Pre-Conway: DPState = array(2) [PState, DState] (on-disk order)
        // DState is the SECOND element (the large one with credentials)
        skip_cbor(state_cbor, cs_body)?
    };

    // DState — could be array(4) or map or other encoding
    let (_, ds_maj, _ds_val) = read_cbor_initial(state_cbor, dstate_off)?;
    eprintln!("    [navigate_to_umap_elems] DState: major={ds_maj} val={_ds_val}");

    if ds_maj == 4 {
        // DState = array(N). Find the UMap (largest map field).
        let (ds_body, ds_len) = read_array_header(state_cbor, dstate_off)?;
        eprintln!("    [navigate_to_umap_elems] DState = array({ds_len}) at offset {dstate_off}");
        // Probe fields
        {
            let mut p = ds_body;
            for fi in 0..ds_len.min(6) {
                let (_, pm, pv) = read_cbor_initial(state_cbor, p).unwrap_or((0, 99, 0));
                let end = skip_cbor(state_cbor, p).unwrap_or(0);
                let sz = if end > p { end - p } else { 0 };
                let pt = match pm { 0=>"uint", 2=>"bytes", 4=>"arr", 5=>"map", 6=>"tag", 7=>"spc", _=>"?" };
                let ss = if sz > 1_000_000 { format!("{}MB", sz/1_000_000) } else if sz > 1000 { format!("{}KB", sz/1000) } else { format!("{sz}B") };
                eprintln!("      DState[{fi}]: {pt}(val={pv}) size={ss}");
                if end == 0 { eprintln!("      DState[{fi}]: skip_cbor FAILED"); break; }
                p = end;
            }
        }
        // DState[0] is always the UMap (verified empirically for all eras)
        let (_, umap_maj, _) = read_cbor_initial(state_cbor, ds_body)?;
        if umap_maj == 4 {
            // Pre-Conway: UMap = array(2) [umElems, umPtrs]
            let (umap_body, _) = read_array_header(state_cbor, ds_body)?;
            Ok(umap_body)
        } else if umap_maj == 5 {
            // Conway: UMap encoded directly as indefinite map
            Ok(ds_body)
        } else {
            Err(HarnessError::ParseError(format!(
                "UMap at offset {}: expected array or map, got major {umap_maj}", ds_body
            )))
        }
    } else if ds_maj == 5 {
        // DState is a map — UMap might be the whole thing
        Ok(dstate_off)
    } else {
        Err(HarnessError::ParseError(format!(
            "DState at offset {dstate_off}: expected array or map, got major {ds_maj}"
        )))
    }
}

/// Alternative UMap navigation: go through LS[1] when LS[0] isn't CertState.
fn navigate_to_umap_elems_via_ls1(
    state_cbor: &[u8],
    ls_body: usize,
) -> Result<usize, HarnessError> {
    // Skip LS[0] to reach LS[1]
    let ls1_off = skip_cbor(state_cbor, ls_body)?;
    let (ls1_body, ls1_len) = read_array_header(state_cbor, ls1_off)?;

    // Determine if LS[1] is CertState or UTxOState
    // If LS[1] is array(3) in Conway, it's CertState = [VState, PState, DState]
    // If LS[1] is array(6), it's UTxOState (our initial labeling was wrong)
    if ls1_len == 3 {
        // CertState: skip VState, PState → DState
        let off = skip_cbor(state_cbor, ls1_body)?;
        let dstate_off = skip_cbor(state_cbor, off)?;
        let (ds_body, _) = read_array_header(state_cbor, dstate_off)?;
        let (_, umap_maj, _) = read_cbor_initial(state_cbor, ds_body)?;
        if umap_maj == 4 {
            let (umap_body, _) = read_array_header(state_cbor, ds_body)?;
            Ok(umap_body)
        } else {
            Ok(ds_body)
        }
    } else {
        Err(HarnessError::ParseError(format!(
            "Cannot find CertState in LedgerState: LS[0]=array(6), LS[1]=array({ls1_len})"
        )))
    }
}

/// Voting delegation statistics from the Conway DState UMap.
#[derive(Debug)]
pub struct VotingDelegationStats {
    /// Total credentials in the UMap.
    pub total_credentials: u64,
    /// Credentials with an active voting delegation (DRep, Abstain, or NoConfidence).
    pub with_voting: u64,
    /// Credentials without voting delegation (SNothing).
    pub without_voting: u64,
    /// Parse errors encountered (skipped entries).
    pub errors: u64,
}

/// Count voting delegations in the Conway DState UMap.
///
/// In Conway (CIP-1694), credentials without an active voting delegation
/// (DRep, AlwaysAbstain, or AlwaysNoConfidence) do NOT receive staking rewards.
///
/// UMElem = array(4) [rdPair, ptrs, stakePoolDelegation, voteDelegation]
/// voteDelegation: StrictMaybe DRep
///   SNothing → array(0) = 0x80
///   SJust x  → array(1) [x]
pub fn count_voting_delegations(
    state_cbor: &[u8],
) -> Result<VotingDelegationStats, HarnessError> {
    let umap_off = navigate_to_umap_elems(state_cbor)?;

    let (mut co, _major, val) = read_cbor_initial(state_cbor, umap_off)?;
    let is_indef = val == u64::MAX;

    let mut total = 0u64;
    let mut with_voting = 0u64;
    let mut without_voting = 0u64;
    let mut errors = 0u64;

    let limit = if is_indef { u64::MAX } else { val };

    for _ in 0..limit {
        if co >= state_cbor.len() {
            break;
        }
        if is_indef && state_cbor[co] == 0xff {
            break;
        }

        // Skip key (credential)
        co = skip_cbor(state_cbor, co)?;

        // Value: UMElem = array(4) [rdPair, ptrs, sPool, drep]
        let (elem_body, elem_maj, elem_val) = read_cbor_initial(state_cbor, co)?;
        let elem_end = skip_cbor(state_cbor, co)?;

        if elem_maj == 4 && elem_val >= 4 {
            // Skip rdPair, ptrs, sPool to reach drep (field 3)
            let mut field_off = elem_body;
            for _ in 0..3 {
                field_off = skip_cbor(state_cbor, field_off)?;
            }

            // drep field: NullMaybe encoding
            // null (major 7, val 22 = 0xf6) → SNothing → no voting delegation
            // anything else → SJust → has voting delegation
            let (_, drep_maj, drep_val) = read_cbor_initial(state_cbor, field_off)?;

            if drep_maj == 7 && drep_val == 22 {
                // CBOR null → no voting delegation
                without_voting += 1;
            } else {
                // Has voting delegation (DRepCredential, AlwaysAbstain, AlwaysNoConfidence)
                with_voting += 1;
            }
        } else {
            errors += 1;
        }

        total += 1;
        co = elem_end;
    }

    Ok(VotingDelegationStats {
        total_credentials: total,
        with_voting,
        without_voting,
        errors,
    })
}

/// Parse the decentralization parameter `d` from the snapshot's on-disk protocol parameters.
///
/// Path: EpochState[1] → LedgerState[1] → UTxOState[3] → GovState[2] → PParams[12]
///
/// For Shelley/Allegra/Mary: GovState = ShelleyGovState = array(5), curPParams at [2]
/// For Conway: GovState = ConwayGovState = array(7), curPParams at [3]
///
/// PParams field 12 = decentralization = tag(30, [numerator, denominator])
///
/// Returns (numerator, denominator) or None if not found/parseable.
pub fn parse_decentralization_param(
    state_cbor: &[u8],
) -> Result<(u64, u64), HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;

    // ES[0] = AccountState (skip)
    let off = skip_cbor(state_cbor, es)?;

    // ES[1] = LedgerState = array(2) [CertState, UTxOState]
    // Note: disk order is CertState first, UTxOState second
    let (ls_body, _) = read_array_header(state_cbor, off)?;

    // LS[0] = CertState (skip)
    let off = skip_cbor(state_cbor, ls_body)?;

    // LS[1] = UTxOState = array(6) [UTxO, deposited, fees, GovState, IncrStake, donation]
    let (utxo_body, _) = read_array_header(state_cbor, off)?;

    // Skip UTxO[0..2] to reach UTxO[3] = GovState
    let mut off = utxo_body;
    for _ in 0..3 { off = skip_cbor(state_cbor, off)?; }

    // GovState: array(5) for Shelley/Allegra/Mary, array(7) for Conway
    let (gs_body, gs_len) = read_array_header(state_cbor, off)?;

    // curPParams position: [2] for ShelleyGovState, [3] for ConwayGovState
    let pp_index = if gs_len >= 7 { 3u32 } else { 2 };
    let mut off = gs_body;
    for _ in 0..pp_index { off = skip_cbor(state_cbor, off)?; }

    // curPParams = array(N) where N depends on era
    let (pp_body, pp_len) = read_array_header(state_cbor, off)?;

    // For Shelley-era: d is at position 12
    // For Conway: d was removed. Return (0, 1).
    if pp_len <= 12 {
        return Ok((0, 1)); // Conway or era without d
    }

    // Skip PP[0..11] to reach PP[12] = d
    let mut off = pp_body;
    for _ in 0..12 { off = skip_cbor(state_cbor, off)?; }

    // d = tag(30, [numerator, denominator]) — Rational
    let (tag_body, maj, _) = read_cbor_initial(state_cbor, off)?;
    if maj == 6 {
        // tag(30) wrapping array(2)
        let (arr_body, _) = read_array_header(state_cbor, tag_body)?;
        let (off, num) = read_uint(state_cbor, arr_body)?;
        let (_, den) = read_uint(state_cbor, off)?;
        Ok((num, den))
    } else if maj == 0 {
        // Plain uint (d=0 stored as just 0)
        let (_, val) = read_uint(state_cbor, off)?;
        Ok((val, 1))
    } else {
        Err(HarnessError::ParseError(format!(
            "d parameter: expected tag or uint, got major {maj} at offset {off}"
        )))
    }
}

/// Parse the set of registered credential hashes from the DState UMap.
///
/// Uses navigate_to_umap_elems to handle era-specific on-disk layout.
/// The UMap umElems map keys are the registered credential hashes.
///
/// Returns the set of 28-byte credential hashes. Used for the leader reward
/// pre-filter (hardforkBabbageForgoRewardPrefilter at PV ≤ 6).
pub fn parse_registered_credentials(
    state_cbor: &[u8],
) -> Result<std::collections::BTreeSet<ade_types::Hash28>, HarnessError> {
    let umap_body = navigate_to_umap_elems(state_cbor)?;

    // umElems = map(credential → UMElem)
    // UMElem = array(4) [StrictMaybe RDPair, Set Ptr, StrictMaybe PoolHash, StrictMaybe DRep]
    // A credential is "account registered" only if UMElem[0] (RDPair) is non-null.
    // StrictMaybe encoding: SNothing = array(0), SJust x = array(1, [x])
    let (mut co, _, _) = read_cbor_initial(state_cbor, umap_body)?;
    let mut creds = std::collections::BTreeSet::new();
    while co < state_cbor.len() && state_cbor[co] != 0xff {
        // Key: array(2) [type_tag, bytes(28)]
        let (key_inner, _, _) = read_cbor_initial(state_cbor, co)?;
        let key_end = skip_cbor(state_cbor, co)?;
        let tag_end = skip_cbor(state_cbor, key_inner)?;
        let (hash_start, _, hash_len) = read_cbor_initial(state_cbor, tag_end)?;
        let mut h = [0u8; 28];
        if hash_len >= 28 {
            h.copy_from_slice(&state_cbor[hash_start..hash_start + 28]);
        }
        co = key_end;

        // Value: UMElem = array(4) [rdpair, ptrs, pool, drep]
        // Check if UMElem[0] (RDPair) is non-null (SJust = array(1+))
        let (elem_body, elem_maj, _) = read_cbor_initial(state_cbor, co)?;
        if elem_maj == 4 {
            // UMElem is array(4). Read first field = StrictMaybe RDPair
            let (_, rdp_maj, rdp_val) = read_cbor_initial(state_cbor, elem_body)?;
            // SNothing = array(0), SJust = array(1, [...])
            let is_registered = rdp_maj == 4 && rdp_val > 0;
            if is_registered && hash_len >= 28 {
                creds.insert(ade_types::Hash28(h));
            }
        } else {
            // Non-array UMElem — treat as registered (conservative)
            if hash_len >= 28 {
                creds.insert(ade_types::Hash28(h));
            }
        }
        co = skip_cbor(state_cbor, co)?;
    }
    Ok(creds)
}

// ── Public wrappers for CBOR navigation (used by integration tests) ──

pub fn navigate_to_nes_pub(data: &[u8]) -> Result<usize, HarnessError> { navigate_to_nes(data) }
pub fn read_array_header_pub(data: &[u8], off: usize) -> Result<(usize, u32), HarnessError> { read_array_header(data, off) }
pub fn read_uint_pub(data: &[u8], off: usize) -> Result<(usize, u64), HarnessError> { read_uint(data, off) }
pub fn skip_cbor_pub(data: &[u8], off: usize) -> Result<usize, HarnessError> { skip_cbor(data, off) }
pub fn read_cbor_initial_pub(data: &[u8], off: usize) -> Result<(usize, u8, u64), HarnessError> { read_cbor_initial(data, off) }

/// Parse the aggregate deposits value from UTxOState[1].
///
/// Path: NES → ES → LS[1] = UTxOState → UTxOState[1] = deposited (uint)
/// On-disk: LS[0] = CertState, LS[1] = UTxOState
pub fn parse_utxo_deposits(state_cbor: &[u8]) -> Result<u64, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;
    let off = skip_cbor(state_cbor, es)?; // skip AccountState
    let (ls_body, _) = read_array_header(state_cbor, off)?;
    let off = skip_cbor(state_cbor, ls_body)?; // skip LS[0] = CertState
    let (utxo_body, _) = read_array_header(state_cbor, off)?;
    let off = skip_cbor(state_cbor, utxo_body)?; // skip UTxOState[0] = UTxO
    let (_, deposited) = read_uint(state_cbor, off)?;
    Ok(deposited)
}

/// Reward-relevant protocol parameters parsed from CBOR.
#[derive(Debug)]
pub struct RewardParams {
    pub n_opt: u64,
    pub a0_num: u64,
    pub a0_den: u64,
    pub rho_num: u64,
    pub rho_den: u64,
    pub tau_num: u64,
    pub tau_den: u64,
}

/// Parse reward-relevant protocol parameters (nOpt, a0, rho, tau) from CBOR.
pub fn parse_reward_params(
    state_cbor: &[u8],
) -> Result<RewardParams, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;
    let off = skip_cbor(state_cbor, es)?; // skip AccountState
    let (ls_body, _) = read_array_header(state_cbor, off)?;
    let off = skip_cbor(state_cbor, ls_body)?; // skip CertState (LS[0])
    let (utxo_body, _) = read_array_header(state_cbor, off)?;
    let mut off = utxo_body;
    for _ in 0..3 { off = skip_cbor(state_cbor, off)?; }
    let (gs_body, gs_len) = read_array_header(state_cbor, off)?;
    let pp_index = if gs_len >= 7 { 3u32 } else { 2 };
    let mut off = gs_body;
    for _ in 0..pp_index { off = skip_cbor(state_cbor, off)?; }
    let (pp_body, _pp_len) = read_array_header(state_cbor, off)?;

    // Skip PP[0..7] to reach PP[8] = nOpt
    let mut off = pp_body;
    for _ in 0..8 { off = skip_cbor(state_cbor, off)?; }
    let (_, n_opt) = read_uint(state_cbor, off)?;
    off = skip_cbor(state_cbor, off)?;

    // PP[9] = a0 = tag(30, [num, den])
    let (a0_num, a0_den) = parse_rational_field(state_cbor, off)?;
    off = skip_cbor(state_cbor, off)?;

    // PP[10] = rho
    let (rho_num, rho_den) = parse_rational_field(state_cbor, off)?;
    off = skip_cbor(state_cbor, off)?;

    // PP[11] = tau
    let (tau_num, tau_den) = parse_rational_field(state_cbor, off)?;

    Ok(RewardParams { n_opt, a0_num, a0_den, rho_num, rho_den, tau_num, tau_den })
}

fn parse_rational_field(state_cbor: &[u8], off: usize) -> Result<(u64, u64), HarnessError> {
    let (inner, maj, _) = read_cbor_initial(state_cbor, off)?;
    if maj == 6 {
        let (arr_body, _) = read_array_header(state_cbor, inner)?;
        let (next, num) = read_uint(state_cbor, arr_body)?;
        let (_, den) = read_uint(state_cbor, next)?;
        Ok((num, den))
    } else if maj == 0 {
        let (_, val) = read_uint(state_cbor, off)?;
        Ok((val, 1))
    } else {
        Err(HarnessError::ParseError(format!("expected rational at {off}")))
    }
}

/// MIR (Move Instantaneous Rewards) parsed from DState[3].
#[derive(Debug, Default)]
pub struct InstantaneousRewardsSummary {
    /// Total MIR from reserves to individual accounts (lovelace).
    pub reserves_to_accounts: u64,
    /// Number of accounts receiving MIR from reserves.
    pub reserves_to_accounts_count: usize,
    /// Total MIR from treasury to individual accounts (lovelace).
    pub treasury_to_accounts: u64,
    /// Number of accounts receiving MIR from treasury.
    pub treasury_to_accounts_count: usize,
    /// Bulk reserves→treasury transfer (DeltaCoin, can be negative).
    pub delta_reserves: i64,
    /// Bulk treasury→reserves transfer (DeltaCoin, can be negative).
    pub delta_treasury: i64,
}

/// Parse InstantaneousRewards from DState[3].
///
/// DState = array(4) [UMap, futureGenDelegs, genDelegs, iRewards]
/// iRewards = array(4) [iRReserves_map, iRTreasury_map, deltaReserves, deltaTreasury]
///
/// For Conway era (no MIR), DState may have fewer fields or iRewards may be empty.
pub fn parse_instantaneous_rewards(
    state_cbor: &[u8],
) -> Result<InstantaneousRewardsSummary, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;

    // ES[0] = AccountState (skip)
    let off = skip_cbor(state_cbor, es)?;
    // ES[1] = LedgerState = array(2) [CertState, UTxOState]
    let (ls_body, _) = read_array_header(state_cbor, off)?;
    // LS[0] = CertState
    let (cs_body, cs_len) = read_array_header(state_cbor, ls_body)?;

    // Navigate to DState
    let dstate_off = if cs_len == 3 {
        // Conway: [VState, PState, DState]
        let off = skip_cbor(state_cbor, cs_body)?;
        skip_cbor(state_cbor, off)?
    } else if cs_len == 2 {
        // Pre-Conway: [DState, PState]
        cs_body
    } else {
        // Unknown layout — try navigating through LS[1] instead
        return Ok(InstantaneousRewardsSummary::default());
    };

    // DState = array(4) [UMap, futureGenDelegs, genDelegs, iRewards]
    let (ds_body, ds_maj, ds_len) = read_cbor_initial(state_cbor, dstate_off)?;
    if ds_maj != 4 || ds_len < 4 {
        return Ok(InstantaneousRewardsSummary::default());
    }

    // Skip DState[0..2] to reach DState[3] = iRewards
    let mut off = ds_body;
    for _ in 0..3 { off = skip_cbor(state_cbor, off)?; }

    // iRewards = array(4) [iRReserves, iRTreasury, deltaReserves, deltaTreasury]
    let (ir_body, ir_maj, ir_len) = read_cbor_initial(state_cbor, off)?;
    if ir_maj != 4 || ir_len < 4 {
        return Ok(InstantaneousRewardsSummary::default());
    }

    // [0] iRReserves: map(credential → coin)
    let mut reserves_to_accounts = 0u64;
    let mut reserves_count = 0usize;
    let (mut co, _, _) = read_cbor_initial(state_cbor, ir_body)?;
    while co < state_cbor.len() && state_cbor[co] != 0xff {
        co = skip_cbor(state_cbor, co)?; // key
        let (_, val) = read_uint(state_cbor, co)?;
        reserves_to_accounts += val;
        reserves_count += 1;
        co = skip_cbor(state_cbor, co)?; // value
    }
    let ir1_off = skip_cbor(state_cbor, ir_body)?;

    // [1] iRTreasury: map(credential → coin)
    let mut treasury_to_accounts = 0u64;
    let mut treasury_count = 0usize;
    let (mut co, _, _) = read_cbor_initial(state_cbor, ir1_off)?;
    while co < state_cbor.len() && state_cbor[co] != 0xff {
        co = skip_cbor(state_cbor, co)?; // key
        let (_, val) = read_uint(state_cbor, co)?;
        treasury_to_accounts += val;
        treasury_count += 1;
        co = skip_cbor(state_cbor, co)?;
    }
    let ir2_off = skip_cbor(state_cbor, ir1_off)?;

    // [2] deltaReserves: int (major 0 = positive, major 1 = negative)
    let (_, dr_maj, dr_val) = read_cbor_initial(state_cbor, ir2_off)?;
    let delta_reserves = if dr_maj == 0 {
        dr_val as i64
    } else if dr_maj == 1 {
        -(dr_val as i64 + 1)
    } else { 0 };
    let ir3_off = skip_cbor(state_cbor, ir2_off)?;

    // [3] deltaTreasury: int
    let (_, dt_maj, dt_val) = read_cbor_initial(state_cbor, ir3_off)?;
    let delta_treasury = if dt_maj == 0 {
        dt_val as i64
    } else if dt_maj == 1 {
        -(dt_val as i64 + 1)
    } else { 0 };

    Ok(InstantaneousRewardsSummary {
        reserves_to_accounts,
        reserves_to_accounts_count: reserves_count,
        treasury_to_accounts,
        treasury_to_accounts_count: treasury_count,
        delta_reserves,
        delta_treasury,
    })
}

/// Parse snapshot entry counts for a specific snapshot position.
pub fn parse_snapshot_counts(
    state_cbor: &[u8],
    snapshot_index: u32,
) -> Result<(usize, usize, usize), HarnessError> {
    let snap_inner = navigate_to_snapshot(state_cbor, snapshot_index)?;
    let stake_count = count_indef_map(state_cbor, snap_inner)?;
    let deleg_off = skip_cbor(state_cbor, snap_inner)?;
    let deleg_count = count_indef_map(state_cbor, deleg_off)?;
    let pool_off = skip_cbor(state_cbor, deleg_off)?;
    let pool_count = count_indef_map(state_cbor, pool_off)?;
    Ok((pool_count, stake_count, deleg_count))
}

/// Parse a credential → coin map (used for stake distributions and rewards).
fn parse_credential_coin_map(
    state_cbor: &[u8],
    map_offset: usize,
) -> Result<Vec<(Hash32, u64)>, HarnessError> {
    let (mut co, _, _) = read_cbor_initial(state_cbor, map_offset)?;
    let mut entries = Vec::new();
    while co < state_cbor.len() && state_cbor[co] != 0xff {
        let (key_inner, _, _) = read_cbor_initial(state_cbor, co)?;
        let key_end = skip_cbor(state_cbor, co)?;
        let (_, _tag) = read_uint(state_cbor, key_inner)?;
        let tag_end = skip_cbor(state_cbor, key_inner)?;
        let (hash_start, _, hash_len) = read_cbor_initial(state_cbor, tag_end)?;
        let mut cred_hash = [0u8; 32];
        if hash_len >= 28 {
            cred_hash[..28].copy_from_slice(&state_cbor[hash_start..hash_start + 28]);
        }
        co = key_end;
        let (_, value) = read_uint(state_cbor, co)?;
        co = skip_cbor(state_cbor, co)?;
        entries.push((Hash32(cred_hash), value));
    }
    Ok(entries)
}

/// Parse a credential → pool_hash map (used for delegation maps).
fn parse_credential_hash_map(
    state_cbor: &[u8],
    map_offset: usize,
) -> Result<Vec<(Hash32, Hash32)>, HarnessError> {
    let (mut co, _, _) = read_cbor_initial(state_cbor, map_offset)?;
    let mut entries = Vec::new();
    while co < state_cbor.len() && state_cbor[co] != 0xff {
        let (key_inner, _, _) = read_cbor_initial(state_cbor, co)?;
        let key_end = skip_cbor(state_cbor, co)?;
        let (_, _tag) = read_uint(state_cbor, key_inner)?;
        let tag_end = skip_cbor(state_cbor, key_inner)?;
        let (hash_start, _, hash_len) = read_cbor_initial(state_cbor, tag_end)?;
        let mut cred_hash = [0u8; 32];
        if hash_len >= 28 {
            cred_hash[..28].copy_from_slice(&state_cbor[hash_start..hash_start + 28]);
        }
        co = key_end;
        let (pool_start, _, pool_len) = read_cbor_initial(state_cbor, co)?;
        let mut pool_hash = [0u8; 32];
        if pool_len >= 28 {
            pool_hash[..28].copy_from_slice(&state_cbor[pool_start..pool_start + 28]);
        }
        co = skip_cbor(state_cbor, co)?;
        entries.push((Hash32(cred_hash), Hash32(pool_hash)));
    }
    Ok(entries)
}

fn skip_n_fields(state_cbor: &[u8], start: usize, n: u32) -> Result<usize, HarnessError> {
    let mut off = start;
    for _ in 0..n {
        off = skip_cbor(state_cbor, off)?;
    }
    Ok(off)
}

fn count_indef_map(state_cbor: &[u8], offset: usize) -> Result<usize, HarnessError> {
    let (mut co, _, val) = read_cbor_initial(state_cbor, offset)?;
    if val != u64::MAX && val > 0 {
        // Definite map — but our read_cbor_initial returns -1 as u64::MAX? No.
        // Actually, for indef, we returned -1 which is i64 -1 but stored as... hmm.
        // Let me handle this properly:
        return Ok(val as usize);
    }
    // Indefinite map: count entries until break
    let mut count = 0;
    while co < state_cbor.len() && state_cbor[co] != 0xff {
        co = skip_cbor(state_cbor, co)?; // key
        co = skip_cbor(state_cbor, co)?; // value
        count += 1;
    }
    Ok(count)
}

// --- Minimal CBOR reader (no external deps, no unwrap) ---

fn read_array_header(data: &[u8], offset: usize) -> Result<(usize, u32), HarnessError> {
    let (off, major, val) = read_cbor_initial(data, offset)?;
    if major != 4 {
        return Err(HarnessError::ParseError(format!(
            "expected array (major 4) at offset {offset}, got major {major}"
        )));
    }
    Ok((off, val as u32))
}

fn read_uint(data: &[u8], offset: usize) -> Result<(usize, u64), HarnessError> {
    let (off, major, val) = read_cbor_initial(data, offset)?;
    if major != 0 {
        return Err(HarnessError::ParseError(format!(
            "expected uint (major 0) at offset {offset}, got major {major}"
        )));
    }
    Ok((off, val))
}

fn read_cbor_initial(data: &[u8], offset: usize) -> Result<(usize, u8, u64), HarnessError> {
    if offset >= data.len() {
        return Err(HarnessError::ParseError(format!(
            "unexpected EOF at offset {offset}"
        )));
    }
    let b = data[offset];
    let major = (b >> 5) & 0x7;
    let ai = b & 0x1f;
    let mut off = offset + 1;

    let val = if ai < 24 {
        ai as u64
    } else if ai == 24 {
        if off >= data.len() {
            return Err(HarnessError::ParseError("EOF in uint8".to_string()));
        }
        let v = data[off] as u64;
        off += 1;
        v
    } else if ai == 25 {
        if off + 2 > data.len() {
            return Err(HarnessError::ParseError("EOF in uint16".to_string()));
        }
        let v = u16::from_be_bytes([data[off], data[off + 1]]) as u64;
        off += 2;
        v
    } else if ai == 26 {
        if off + 4 > data.len() {
            return Err(HarnessError::ParseError("EOF in uint32".to_string()));
        }
        let v = u32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
            as u64;
        off += 4;
        v
    } else if ai == 27 {
        if off + 8 > data.len() {
            return Err(HarnessError::ParseError("EOF in uint64".to_string()));
        }
        let v = u64::from_be_bytes([
            data[off],
            data[off + 1],
            data[off + 2],
            data[off + 3],
            data[off + 4],
            data[off + 5],
            data[off + 6],
            data[off + 7],
        ]);
        off += 8;
        v
    } else if ai == 31 {
        // indefinite length — sentinel value, callers must handle
        return Ok((off, major, u64::MAX));
    } else {
        return Err(HarnessError::ParseError(format!(
            "unsupported additional info {ai} at offset {offset}"
        )));
    };

    Ok((off, major, val))
}

fn skip_cbor(data: &[u8], offset: usize) -> Result<usize, HarnessError> {
    if offset >= data.len() {
        return Err(HarnessError::ParseError(format!(
            "unexpected EOF at offset {offset}"
        )));
    }
    let b = data[offset];
    let major = (b >> 5) & 0x7;
    let ai = b & 0x1f;
    let mut off = offset + 1;

    let val: u64 = if ai < 24 {
        ai as u64
    } else if ai == 24 {
        let v = data[off] as u64;
        off += 1;
        v
    } else if ai == 25 {
        let v = u16::from_be_bytes([data[off], data[off + 1]]) as u64;
        off += 2;
        v
    } else if ai == 26 {
        let v = u32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
            as u64;
        off += 4;
        v
    } else if ai == 27 {
        let v = u64::from_be_bytes([
            data[off],
            data[off + 1],
            data[off + 2],
            data[off + 3],
            data[off + 4],
            data[off + 5],
            data[off + 6],
            data[off + 7],
        ]);
        off += 8;
        v
    } else if ai == 31 {
        // indefinite
        match major {
            4 => {
                while off < data.len() && data[off] != 0xff {
                    off = skip_cbor(data, off)?;
                }
                return Ok(off + 1);
            }
            5 => {
                while off < data.len() && data[off] != 0xff {
                    off = skip_cbor(data, off)?;
                    off = skip_cbor(data, off)?;
                }
                return Ok(off + 1);
            }
            2 | 3 => {
                while off < data.len() && data[off] != 0xff {
                    off = skip_cbor(data, off)?;
                }
                return Ok(off + 1);
            }
            _ => {
                return Err(HarnessError::ParseError(format!(
                    "unexpected indefinite major {major} at offset {offset}"
                )));
            }
        }
    } else {
        return Err(HarnessError::ParseError(format!(
            "unsupported ai {ai} at offset {offset}"
        )));
    };

    match major {
        0 | 1 | 7 => Ok(off),
        2 | 3 => Ok(off + val as usize),
        6 => skip_cbor(data, off),
        4 => {
            for _ in 0..val {
                off = skip_cbor(data, off)?;
            }
            Ok(off)
        }
        5 => {
            for _ in 0..val {
                off = skip_cbor(data, off)?;
                off = skip_cbor(data, off)?;
            }
            Ok(off)
        }
        _ => Err(HarnessError::ParseError(format!(
            "unknown major {major} at offset {offset}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn snapshots_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("corpus")
            .join("snapshots")
    }

    #[test]
    fn load_byron_pre_hfc_snapshot() {
        let tarball = snapshots_dir().join("snapshot_4492800.tar.gz");
        if !tarball.exists() {
            eprintln!("Skipping: {}", tarball.display());
            return;
        }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();
        assert!(state_bytes.len() > 100, "state file too small");

        let header = parse_snapshot_header(&state_bytes).unwrap();
        eprintln!("Byron pre-HFC: telescope={}, era_idx={}, epoch={}, size={}",
            header.telescope_length, header.current_era_index,
            header.epoch, header.state_size);

        // At slot 4,492,800 the HFC has occurred: epoch 208, telescope 2
        assert_eq!(header.epoch, 208);
    }

    #[test]
    fn load_shelley_epoch_boundary_snapshot() {
        let tarball = snapshots_dir().join("snapshot_4924880.tar.gz");
        if !tarball.exists() {
            eprintln!("Skipping: {}", tarball.display());
            return;
        }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();
        let header = parse_snapshot_header(&state_bytes).unwrap();
        eprintln!("Shelley epoch boundary: telescope={}, era_idx={}, epoch={}, size={}",
            header.telescope_length, header.current_era_index,
            header.epoch, header.state_size);

        // Shelley at slot 4,924,880 = epoch 209, telescope length 2
        assert_eq!(header.epoch, 209);
        assert_eq!(header.telescope_length, 2);
    }

    #[test]
    fn snapshot_state_hashes_are_stable() {
        let tarball = snapshots_dir().join("snapshot_4492800.tar.gz");
        if !tarball.exists() {
            eprintln!("Skipping: {}", tarball.display());
            return;
        }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();
        let hash1 = compute_state_hash(&state_bytes);
        let hash2 = compute_state_hash(&state_bytes);
        assert_eq!(hash1, hash2, "state hash must be deterministic");

        // Hash is 64 hex chars
        let hex = format!("{hash1}");
        assert_eq!(hex.len(), 64);
        eprintln!("Byron HFC state hash: {hex}");
    }

    #[test]
    fn parse_oracle_hash_file() {
        let hash_file = snapshots_dir().join("hashes_4924880.txt");
        if !hash_file.exists() {
            eprintln!("Skipping: {}", hash_file.display());
            return;
        }

        let content = std::fs::read_to_string(&hash_file).unwrap();
        let entries = parse_oracle_hashes(&content).unwrap();

        assert!(!entries.is_empty());
        assert_eq!(entries[0].slot, 4924900);
        assert!(entries[0].state_size > 0);
        eprintln!(
            "Shelley epoch boundary hashes: {} entries, slots {}..{}",
            entries.len(),
            entries[0].slot,
            entries[entries.len() - 1].slot
        );
    }

    #[test]
    fn epoch_boundary_state_size_matches_oracle() {
        // The snapshot state size should match the oracle's state_size
        // for the first hash entry (which is the state AFTER the first
        // block post-snapshot).
        let cases = [
            ("snapshot_4924880.tar.gz", "hashes_4924880.txt"),
            ("snapshot_17020848.tar.gz", "hashes_17020848.txt"),
            ("snapshot_40348902.tar.gz", "hashes_40348902.txt"),
            ("snapshot_134092810.tar.gz", "hashes_134092810.txt"),
        ];

        for (snap, hashes) in &cases {
            let tarball = snapshots_dir().join(snap);
            let hash_file = snapshots_dir().join(hashes);
            if !tarball.exists() || !hash_file.exists() {
                continue;
            }

            let state_bytes = extract_state_from_tarball(&tarball).unwrap();
            let content = std::fs::read_to_string(&hash_file).unwrap();
            let entries = parse_oracle_hashes(&content).unwrap();

            // State size should be close to (but not exactly equal to)
            // the first oracle entry's state_size. The snapshot is PRE-block,
            // the oracle hash is POST-block.
            let snap_size = state_bytes.len() as u64;
            let oracle_size = entries[0].state_size;
            let diff = snap_size.abs_diff(oracle_size);

            eprintln!(
                "{}: snapshot={}, oracle_first={}, diff={}",
                snap, snap_size, oracle_size, diff
            );

            // Size should be within 1% — same era, close slots
            let pct = (diff as f64 / snap_size as f64) * 100.0;
            assert!(
                pct < 1.0,
                "{snap}: state size divergence too large: {pct:.2}%"
            );
        }
    }

    #[test]
    fn loaded_snapshot_provides_reset_to() {
        let tarball = snapshots_dir().join("snapshot_4492800.tar.gz");
        if !tarball.exists() {
            return;
        }

        let snap = LoadedSnapshot::from_tarball(&tarball).unwrap();

        // State hash is deterministic
        assert_eq!(snap.state_hash, compute_state_hash(&snap.raw_cbor));

        // Can produce a LedgerState
        let state = snap.to_ledger_state();
        assert_eq!(state.epoch_state.epoch, ade_types::EpochNo(208));
        assert_eq!(state.era, ade_types::CardanoEra::Shelley);

        eprintln!(
            "LoadedSnapshot: epoch={}, era={:?}, hash={}, size={}",
            snap.header.epoch, state.era, snap.state_hash, snap.raw_cbor.len()
        );
    }

    #[test]
    fn hfc_pair_hashes_differ() {
        // Pre-HFC and post-epoch-boundary snapshots for the same transition
        // must have different state hashes (blocks were applied between them).
        let pairs = [
            ("snapshot_4492800.tar.gz", "snapshot_4924880.tar.gz"),
            ("snapshot_16588800.tar.gz", "snapshot_17020848.tar.gz"),
            ("snapshot_72316896.tar.gz", "snapshot_72748820.tar.gz"),
        ];

        for (pre, post) in &pairs {
            let pre_path = snapshots_dir().join(pre);
            let post_path = snapshots_dir().join(post);
            if !pre_path.exists() || !post_path.exists() {
                continue;
            }

            let pre_snap = LoadedSnapshot::from_tarball(&pre_path).unwrap();
            let post_snap = LoadedSnapshot::from_tarball(&post_path).unwrap();

            assert_ne!(
                pre_snap.state_hash, post_snap.state_hash,
                "pre-HFC and epoch boundary must have different hashes"
            );

            // Telescope should match (same era after HFC)
            assert_eq!(pre_snap.header.telescope_length, post_snap.header.telescope_length);

            eprintln!(
                "{} -> {}: telescopes match ({}), hashes differ",
                pre, post, pre_snap.header.telescope_length
            );
        }
    }

    #[test]
    fn to_ledger_state_preserves_delegation_count() {
        let tarball = snapshots_dir().join("snapshot_17020848.tar.gz");
        if !tarball.exists() {
            return;
        }

        let snap = LoadedSnapshot::from_tarball(&tarball).unwrap();

        // Parse count directly from CBOR
        let (_, _, raw_deleg_count) = parse_go_snapshot_counts(&snap.raw_cbor).unwrap();

        // Load into LedgerState
        let state = snap.to_ledger_state();
        let stored_deleg_count = state.cert_state.delegation.delegations.len();
        let stored_reg_count = state.cert_state.delegation.registrations.len();

        eprintln!("Delegation count chain:");
        eprintln!("  raw CBOR:      {raw_deleg_count}");
        eprintln!("  stored delegs: {stored_deleg_count}");
        eprintln!("  stored regs:   {stored_reg_count}");

        assert_eq!(
            stored_deleg_count, raw_deleg_count,
            "delegation count must survive parse → store conversion"
        );
        // Registrations now include ALL registered credentials from DState rewards map,
        // not just delegating ones. So registrations >= delegations.
        assert!(
            stored_reg_count >= raw_deleg_count,
            "registration count ({stored_reg_count}) must be >= delegation count ({raw_deleg_count})"
        );
    }

    #[test]
    fn to_ledger_state_preserves_pool_count() {
        let tarball = snapshots_dir().join("snapshot_17020848.tar.gz");
        if !tarball.exists() {
            return;
        }

        let snap = LoadedSnapshot::from_tarball(&tarball).unwrap();
        let (raw_pool_count, _, _) = parse_go_snapshot_counts(&snap.raw_cbor).unwrap();
        let state = snap.to_ledger_state();
        let stored_pool_count = state.cert_state.pool.pools.len();

        eprintln!("Pool count chain:");
        eprintln!("  raw CBOR:     {raw_pool_count}");
        eprintln!("  stored pools: {stored_pool_count}");

        assert_eq!(
            stored_pool_count, raw_pool_count,
            "pool count must survive parse → store conversion"
        );

        // Verify pool params have correct values
        if let Some((_, params)) = state.cert_state.pool.pools.iter().next() {
            eprintln!("  first pool: pledge={}, cost={}, margin={}/{}",
                params.pledge.0 / 1_000_000,
                params.cost.0 / 1_000_000,
                params.margin.0, params.margin.1,
            );
            assert!(params.cost.0 > 0, "pool cost must be non-zero");
            assert!(params.margin.1 > 0, "margin denominator must be non-zero");
        }
    }

    #[test]
    fn go_snapshot_total_stake_matches_oracle() {
        let tarball = snapshots_dir().join("snapshot_17020848.tar.gz");
        if !tarball.exists() { return; }

        let snap = LoadedSnapshot::from_tarball(&tarball).unwrap();
        let state = snap.to_ledger_state();

        let total_stake: u64 = state.epoch_state.snapshots.go.0.pool_stakes
            .values().map(|c| c.0).sum();

        // Oracle total stake (from Python): ~20.2T lovelace
        eprintln!("Go snapshot total stake: {} lovelace = {} ADA",
            total_stake, total_stake / 1_000_000);

        let pool_count = state.epoch_state.snapshots.go.0.pool_stakes.len();
        let deleg_count = state.epoch_state.snapshots.go.0.delegations.len();

        eprintln!("  pool_stakes entries: {}", pool_count);
        eprintln!("  delegation entries: {}", deleg_count);

        // Should be in the right ballpark (~20B ADA)
        assert!(total_stake > 1_000_000_000_000_000,
            "total stake should be > 1B ADA in lovelace");
    }

    #[test]
    fn parse_go_snapshot_from_allegra() {
        let tarball = snapshots_dir().join("snapshot_17020848.tar.gz");
        if !tarball.exists() {
            return;
        }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();
        let (pools, stakes, delegs) = parse_go_snapshot_counts(&state_bytes).unwrap();

        eprintln!(
            "Allegra ep237 go snapshot: {} pools, {} stakes, {} delegations",
            pools, stakes, delegs
        );

        // From Python analysis: SS[2] (go) has ~1,445 pools, ~96,260 stakes, ~98,331 delegations
        assert!(pools > 1000, "should have > 1000 pools");
        assert!(stakes > 90000, "should have > 90000 stake entries");
        assert!(delegs > 90000, "should have > 90000 delegation entries");
    }

    #[test]
    fn parse_go_delegations_from_allegra() {
        let tarball = snapshots_dir().join("snapshot_17020848.tar.gz");
        if !tarball.exists() {
            return;
        }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();
        let delegations = parse_go_delegations(&state_bytes).unwrap();

        eprintln!("Allegra ep237: {} delegations loaded", delegations.len());
        assert!(delegations.len() > 90000);

        // Verify delegation entries have non-zero pool hashes
        for (cred, pool) in delegations.iter().take(3) {
            eprintln!(
                "  cred={}... → pool={}...",
                &format!("{cred}")[..16],
                &format!("{pool}")[..16],
            );
            assert_ne!(*pool, Hash32([0u8; 32]), "pool hash must be non-zero");
        }
    }

    #[test]
    fn load_all_proof_grade_snapshots() {
        // Epoch values are from the CBOR (ground truth), not assumed.
        // Pre-HFC snapshots may show the post-transition epoch because
        // the state at the HFC boundary slot is already the new era.
        let proof_snapshots = [
            ("snapshot_4492800.tar.gz", 208u64, "byron->shelley HFC"),
            ("snapshot_4924880.tar.gz", 209, "shelley epoch boundary"),
            ("snapshot_16588800.tar.gz", 236, "shelley->allegra HFC"),
            ("snapshot_17020848.tar.gz", 237, "allegra epoch boundary"),
            ("snapshot_23068800.tar.gz", 251, "allegra->mary HFC"),
            ("snapshot_23500962.tar.gz", 252, "mary epoch boundary"),
            ("snapshot_39916975.tar.gz", 290, "mary->alonzo HFC"),
            ("snapshot_40348902.tar.gz", 291, "alonzo epoch boundary"),
            ("snapshot_72316896.tar.gz", 365, "alonzo->babbage HFC"),
            ("snapshot_72748820.tar.gz", 366, "babbage epoch boundary"),
            ("snapshot_133660855.tar.gz", 507, "babbage->conway HFC"),
            ("snapshot_134092810.tar.gz", 508, "conway epoch boundary"),
        ];

        eprintln!("\n=== Proof-Grade Snapshot Verification ===");
        eprintln!("{:<35} {:>5} {:>5} {:>10}", "Snapshot", "Epoch", "Tele", "Size");
        eprintln!("{}", "-".repeat(60));

        for (filename, expected_epoch, label) in &proof_snapshots {
            let tarball = snapshots_dir().join(filename);
            if !tarball.exists() {
                eprintln!("{:<35} MISSING", label);
                continue;
            }

            let state_bytes = extract_state_from_tarball(&tarball).unwrap();
            let header = parse_snapshot_header(&state_bytes).unwrap();

            eprintln!(
                "{:<35} {:>5} {:>5} {:>10}",
                label, header.epoch, header.telescope_length, header.state_size
            );

            assert_eq!(
                header.epoch, *expected_epoch,
                "{label}: expected epoch {expected_epoch}, got {}",
                header.epoch
            );
        }
        eprintln!("========================================\n");
    }

    #[test]
    fn probe_dstate_allegra_237() {
        let tarball = snapshots_dir().join("snapshot_17020848.tar.gz");
        if !tarball.exists() {
            return;
        }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();
        // Verbose navigation to see offsets
        let _ = navigate_to_dstate_verbose(&state_bytes, true);
        let probe = probe_dstate_structure(&state_bytes).unwrap();

        eprintln!("\n=== DState Structure Probe (Allegra epoch 237) ===");
        eprintln!("  container: {} ({})", if probe.container_type == 5 { "map" } else { "array" }, probe.container_type);
        eprintln!("  header count: {} (0 = indefinite)", probe.field_count);
        eprintln!("  entries found: {}", probe.entry_count);
        for f in &probe.fields {
            if let Some((km, kv, ks)) = f.key_info {
                let kt = match km { 0 => "uint", 2 => "bytes", 3 => "text", _ => "?" };
                eprintln!(
                    "  DS[{}]: key={}(val={},{}B) → {}(val={}, {}B)",
                    f.index, kt, kv, ks, f.type_name(), f.header_value, f.size,
                );
            } else {
                eprintln!(
                    "  DS[{}]: {}(val={}, {}B)",
                    f.index, f.type_name(), f.header_value, f.size,
                );
            }
        }
        eprintln!("===================================================\n");

        assert!(probe.entry_count >= 3, "CertState should have >= 3 fields, got {}", probe.entry_count);

        // Probe CS[4] (the 5.3MB array(2)) — likely contains DState data
        if let Some(cs4) = probe.fields.get(4) {
            if cs4.major_type == 4 {
                let (inner, _, _) = read_cbor_initial(&state_bytes, cs4.offset).unwrap();
                let mut sub_off = inner;
                for j in 0..cs4.header_value.min(5) {
                    let (_, sm, sv) = read_cbor_initial(&state_bytes, sub_off).unwrap();
                    let sz = skip_cbor(&state_bytes, sub_off).unwrap() - sub_off;
                    let st = match sm { 0 => "uint", 4 => "array", 5 => "map", _ => "?" };
                    eprintln!("  CS[4][{j}]: {st}(val={sv}, {sz}B)");
                    sub_off = skip_cbor(&state_bytes, sub_off).unwrap();
                }
            }
        }
        eprintln!();

        // Also check: what's inside the first map of CS[4][0] if it's a map?
        if let Some(cs4) = probe.fields.get(4) {
            let (inner, _, _) = read_cbor_initial(&state_bytes, cs4.offset).unwrap();
            let (sub0_body, sub0_maj, _sub0_val) = read_cbor_initial(&state_bytes, inner).unwrap();
            if sub0_maj == 5 {
                // It's a map — peek at first 3 entries
                let mut co = sub0_body;
                for k in 0..3u32 {
                    if co >= state_bytes.len() || state_bytes[co] == 0xff { break; }
                    let (kb, km, kv) = read_cbor_initial(&state_bytes, co).unwrap();
                    let ksz = skip_cbor(&state_bytes, co).unwrap() - co;
                    co = skip_cbor(&state_bytes, co).unwrap();
                    let (_, vm, vv) = read_cbor_initial(&state_bytes, co).unwrap();
                    let vsz = skip_cbor(&state_bytes, co).unwrap() - co;
                    co = skip_cbor(&state_bytes, co).unwrap();
                    let kt = match km { 0 => "uint", 2 => "bytes", 4 => "array", _ => "?" };
                    let vt = match vm { 0 => "uint", 2 => "bytes", 4 => "array", _ => "?" };
                    // If key is bytes, show first few bytes
                    let key_preview = if km == 2 && kv >= 4 {
                        format!("{:02x}{:02x}{:02x}{:02x}...", state_bytes[kb], state_bytes[kb+1], state_bytes[kb+2], state_bytes[kb+3])
                    } else {
                        format!("val={kv}")
                    };
                    eprintln!("    CS[4][0] entry {k}: key={kt}({key_preview}, {ksz}B) → {vt}(val={vv}, {vsz}B)");
                    let _ = (kb, kv);
                }
                let count = count_indef_map(&state_bytes, inner).unwrap_or(0);
                eprintln!("    CS[4][0] total entries: {count}");
            }
        }
    }

    #[test]
    fn parse_reward_accounts_allegra_237() {
        let tarball = snapshots_dir().join("snapshot_17020848.tar.gz");
        if !tarball.exists() { return; }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();
        let accounts = parse_reward_accounts(&state_bytes).unwrap();

        let total_registered = accounts.len();
        let with_rewards: usize = accounts.iter().filter(|(_, r)| *r > 0).count();
        let total_reward: u64 = accounts.iter().map(|(_, r)| *r).sum();

        eprintln!("\n=== Reward Accounts (Allegra epoch 237) ===");
        eprintln!("  total registered:  {total_registered}");
        eprintln!("  with rewards > 0:  {with_rewards}");
        eprintln!("  total reward bal:  {total_reward} lovelace ({} ADA)", total_reward / 1_000_000);
        eprintln!("============================================\n");

        // Should have more registrations than the go-snapshot delegation count (98,331)
        assert!(total_registered > 100_000,
            "should have > 100K registered credentials, got {total_registered}");
        // Should match the probe count (135,863)
        assert!(total_registered > 130_000,
            "should have > 130K registered credentials (from probe), got {total_registered}");
    }

    #[test]
    fn probe_dstate_conway_508() {
        let tarball = snapshots_dir().join("snapshot_134092810.tar.gz");
        if !tarball.exists() {
            return;
        }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();
        let probe = probe_dstate_structure(&state_bytes).unwrap();

        eprintln!("\n=== DState Structure Probe (Conway epoch 508) ===");
        eprintln!("  container: {} ({})", if probe.container_type == 5 { "map" } else { "array" }, probe.container_type);
        eprintln!("  header count: {} (0 = indefinite)", probe.field_count);
        eprintln!("  entries found: {}", probe.entry_count);
        for f in &probe.fields {
            if let Some((km, kv, ks)) = f.key_info {
                let kt = match km { 0 => "uint", 2 => "bytes", 3 => "text", _ => "?" };
                eprintln!(
                    "  DS[{}]: key={}(val={},{}B) → {}(val={}, {}B)",
                    f.index, kt, kv, ks, f.type_name(), f.header_value, f.size,
                );
            } else {
                eprintln!(
                    "  DS[{}]: {}(val={}, {}B)",
                    f.index, f.type_name(), f.header_value, f.size,
                );
            }
        }
        eprintln!("===================================================\n");

        assert!(probe.entry_count >= 3, "DState should have >= 3 entries, got {}", probe.entry_count);
    }

    #[test]
    fn probe_conway_certstate_navigation() {
        let tarball = snapshots_dir().join("snapshot_134092810.tar.gz");
        if !tarball.exists() {
            return;
        }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();

        // Step-by-step navigation with diagnostics
        let off = navigate_to_nes(&state_bytes).unwrap();
        let es = skip_nes_to_epoch_state(&state_bytes, off).unwrap();

        // ES[0] = AccountState
        let (_, acct_maj, acct_val) = read_cbor_initial(&state_bytes, es).unwrap();
        let acct_size = skip_cbor(&state_bytes, es).unwrap() - es;
        eprintln!("\n=== Conway CertState Navigation ===");
        eprintln!("  ES[0] AccountState: major={acct_maj} val={acct_val} size={acct_size}B");

        let off = skip_cbor(&state_bytes, es).unwrap();

        // ES[1] = LedgerState
        let (_, ls_maj, ls_val) = read_cbor_initial(&state_bytes, off).unwrap();
        eprintln!("  ES[1] LedgerState: major={ls_maj} val={ls_val}");
        let (ls_body, ls_len) = read_array_header(&state_bytes, off).unwrap();
        eprintln!("    array length: {ls_len}");

        // LS[0] = UTxOState
        let (_, utxo_maj, utxo_val) = read_cbor_initial(&state_bytes, ls_body).unwrap();
        let utxo_size = skip_cbor(&state_bytes, ls_body).unwrap() - ls_body;
        eprintln!("  LS[0] UTxOState: major={utxo_maj} val={utxo_val} size={}MB", utxo_size / 1_000_000);

        // If UTxOState is an array, probe its elements
        if utxo_maj == 4 {
            let (mut elem_off, utxo_len) = read_array_header(&state_bytes, ls_body).unwrap();
            eprintln!("    UTxOState array length: {utxo_len}");
            for i in 0..utxo_len.min(10) {
                let (_, em, ev) = read_cbor_initial(&state_bytes, elem_off).unwrap();
                let esize = skip_cbor(&state_bytes, elem_off).unwrap() - elem_off;
                let etype = match em { 0 => "uint", 2 => "bytes", 3 => "text", 4 => "array", 5 => "map", 6 => "tag", _ => "?" };
                eprintln!("    UTxO[{i}]: {etype}(val={ev}) size={}",
                    if esize > 1_000_000 { format!("{}MB", esize / 1_000_000) } else { format!("{esize}B") });
                elem_off = skip_cbor(&state_bytes, elem_off).unwrap();
            }
        }

        let off = skip_cbor(&state_bytes, ls_body).unwrap();

        // LS[1] = CertState (or whatever is here)
        let (_, cs_maj, cs_val) = read_cbor_initial(&state_bytes, off).unwrap();
        let cs_size = skip_cbor(&state_bytes, off).unwrap() - off;
        eprintln!("  LS[1] CertState: major={cs_maj} val={cs_val} size={}",
            if cs_size > 1_000_000 { format!("{}MB", cs_size / 1_000_000) } else { format!("{cs_size}B") });

        // If CertState is an array, probe its elements
        if cs_maj == 4 {
            let (mut elem_off, cs_len) = read_array_header(&state_bytes, off).unwrap();
            eprintln!("    CertState array length: {cs_len}");
            for i in 0..cs_len.min(10) {
                let (_, em, ev) = read_cbor_initial(&state_bytes, elem_off).unwrap();
                let esize = skip_cbor(&state_bytes, elem_off).unwrap() - elem_off;
                let etype = match em { 0 => "uint", 2 => "bytes", 3 => "text", 4 => "array", 5 => "map", 6 => "tag", _ => "?" };
                eprintln!("    CS[{i}]: {etype}(val={ev}) size={}",
                    if esize > 1_000_000 { format!("{}MB", esize / 1_000_000) } else { format!("{esize}B") });
                elem_off = skip_cbor(&state_bytes, elem_off).unwrap();
            }
        }

        // If LS has more than 2 elements, show them
        if ls_len > 2 {
            let mut extra_off = skip_cbor(&state_bytes, off).unwrap();
            for i in 2..ls_len.min(10) {
                let (_, em, ev) = read_cbor_initial(&state_bytes, extra_off).unwrap();
                let esize = skip_cbor(&state_bytes, extra_off).unwrap() - extra_off;
                let etype = match em { 0 => "uint", 2 => "bytes", 3 => "text", 4 => "array", 5 => "map", 6 => "tag", _ => "?" };
                eprintln!("  LS[{i}]: {etype}(val={ev}) size={}",
                    if esize > 1_000_000 { format!("{}MB", esize / 1_000_000) } else { format!("{esize}B") });
                extra_off = skip_cbor(&state_bytes, extra_off).unwrap();
            }
        }

        // Deep probe into CS[3] (likely VState/governance)
        if cs_maj == 4 {
            let (mut elem_off, cs_len) = read_array_header(&state_bytes, off).unwrap();
            // Navigate to CS[3]
            for _ in 0..3.min(cs_len) {
                elem_off = skip_cbor(&state_bytes, elem_off).unwrap();
            }
            if cs_len > 3 {
                let (_, g_maj, _g_val) = read_cbor_initial(&state_bytes, elem_off).unwrap();
                let _g_size = skip_cbor(&state_bytes, elem_off).unwrap() - elem_off;
                eprintln!("\n  --- CS[3] deep probe (array(7), 212KB) ---");
                if g_maj == 4 {
                    let (mut gi, g_len) = read_array_header(&state_bytes, elem_off).unwrap();
                    for i in 0..g_len.min(10) {
                        let (_, gm, gv) = read_cbor_initial(&state_bytes, gi).unwrap();
                        let gs = skip_cbor(&state_bytes, gi).unwrap() - gi;
                        let gt = match gm { 0 => "uint", 2 => "bytes", 3 => "text", 4 => "array", 5 => "map", 6 => "tag", _ => "?" };
                        eprintln!("    CS3[{i}]: {gt}(val={gv}) size={}",
                            if gs > 1_000 { format!("{}KB", gs / 1_000) } else { format!("{gs}B") });
                        gi = skip_cbor(&state_bytes, gi).unwrap();
                    }
                }
            }
        }

        // Probe VState from LS[0] (CertState) = array(3) [VState, PState, DState]
        // LS[0] = CertState (disk order: CertState first, UTxOState second)
        {
            // Re-navigate cleanly to LedgerState
            let off2 = navigate_to_nes(&state_bytes).unwrap();
            let es2 = skip_nes_to_epoch_state(&state_bytes, off2).unwrap();
            let off2 = skip_cbor(&state_bytes, es2).unwrap(); // skip AccountState
            let (ls_body, _ls_len) = read_array_header(&state_bytes, off2).unwrap();

            eprintln!("\n  --- LS[0] = CertState (Conway) ---");
            let (_, ls0_maj, ls0_val) = read_cbor_initial(&state_bytes, ls_body).unwrap();
            let ls0_size = skip_cbor(&state_bytes, ls_body).unwrap() - ls_body;
            let ls0_type = match ls0_maj { 4 => "array", 5 => "map", _ => "?" };
            eprintln!("    LS[0]: {ls0_type}({ls0_val}) size={}MB", ls0_size / 1_000_000);

            if ls0_maj == 4 {
                let (mut elem, ls0_len) = read_array_header(&state_bytes, ls_body).unwrap();
                let names = ["VState", "PState", "DState"];
                for i in 0..ls0_len.min(3) {
                    let (_, em, ev) = read_cbor_initial(&state_bytes, elem).unwrap();
                    let es = skip_cbor(&state_bytes, elem).unwrap() - elem;
                    let et = match em { 4 => "array", 5 => "map", _ => "?" };
                    let name = names.get(i as usize).unwrap_or(&"?");
                    eprintln!("    LS0[{i}] {name}: {et}({ev}) size={}",
                        if es > 1_000_000 { format!("{}MB", es / 1_000_000) } else if es > 1_000 { format!("{}KB", es / 1_000) } else { format!("{es}B") });

                    // Deep probe VState
                    if i == 0 && em == 4 {
                        let (mut vi, vlen) = read_array_header(&state_bytes, elem).unwrap();
                        for j in 0..vlen.min(5) {
                            let (_, vm, vv) = read_cbor_initial(&state_bytes, vi).unwrap();
                            let vs = skip_cbor(&state_bytes, vi).unwrap() - vi;
                            let vt = match vm { 0 => "uint", 4 => "array", 5 => "map", 6 => "tag", _ => "?" };
                            eprintln!("      VS[{j}]: {vt}(val={vv}) size={}",
                                if vs > 1_000 { format!("{}KB", vs / 1_000) } else { format!("{vs}B") });
                            vi = skip_cbor(&state_bytes, vi).unwrap();
                        }
                    }

                    elem = skip_cbor(&state_bytes, elem).unwrap();
                }
            }

            // Now probe ConwayGovState proposals: LS[1][3][0]
            let utxo_off = skip_cbor(&state_bytes, ls_body).unwrap(); // LS[1] = UTxOState
            let (utxo_body, _) = read_array_header(&state_bytes, utxo_off).unwrap();
            // Skip UTxO[0], UTxO[1], UTxO[2] to reach UTxO[3] = GovState
            let gs_off = skip_cbor(&state_bytes,
                skip_cbor(&state_bytes,
                    skip_cbor(&state_bytes, utxo_body).unwrap()
                ).unwrap()
            ).unwrap();
            let (gs_body, gs_len) = read_array_header(&state_bytes, gs_off).unwrap();
            eprintln!("\n  --- ConwayGovState = LS[1][3] = array({gs_len}) ---");

            // GovState[0] = Proposals
            let (_, p_maj, p_val) = read_cbor_initial(&state_bytes, gs_body).unwrap();
            let p_size = skip_cbor(&state_bytes, gs_body).unwrap() - gs_body;
            eprintln!("    GS[0] Proposals: {}({p_val}) size={p_size}B", match p_maj { 4 => "array", 5 => "map", _ => "?" });

            // Probe inside Proposals
            if p_maj == 4 {
                let (mut pi, plen) = read_array_header(&state_bytes, gs_body).unwrap();
                for j in 0..plen.min(5) {
                    let (_, pm, pv) = read_cbor_initial(&state_bytes, pi).unwrap();
                    let ps = skip_cbor(&state_bytes, pi).unwrap() - pi;
                    let pt = match pm { 0 => "uint", 2 => "bytes", 4 => "array", 5 => "map", 6 => "tag", _ => "?" };
                    eprintln!("      P[{j}]: {pt}(val={pv}) size={ps}B");

                    // If this is a map (proposals map), probe first few entries
                    if pm == 5 && ps > 10 {
                        let (mut mi, _, _) = read_cbor_initial(&state_bytes, pi).unwrap();
                        let mut count = 0;
                        let limit = 3;
                        while mi < state_bytes.len() && state_bytes[mi] != 0xff && count < limit {
                            let key_start = mi;
                            mi = skip_cbor(&state_bytes, mi).unwrap(); // key
                            let key_size = mi - key_start;
                            let val_start = mi;
                            let (_, vm, vv) = read_cbor_initial(&state_bytes, mi).unwrap();
                            mi = skip_cbor(&state_bytes, mi).unwrap(); // value
                            let val_size = mi - val_start;
                            let vt = match vm { 4 => "array", 5 => "map", 6 => "tag", _ => "?" };
                            eprintln!("        entry[{count}]: key={key_size}B → {vt}(val={vv}) {val_size}B");
                            count += 1;
                        }
                        if count == 0 {
                            eprintln!("        (empty map)");
                        }
                    }

                    pi = skip_cbor(&state_bytes, pi).unwrap();
                }
            }

            // GovState[3] = CurPParams — probe first few params
            let mut gs_elem = gs_body;
            for _ in 0..3 { gs_elem = skip_cbor(&state_bytes, gs_elem).unwrap(); }
            let (pp_body, pp_len) = read_array_header(&state_bytes, gs_elem).unwrap();
            eprintln!("\n    GS[3] CurPParams: array({pp_len})");
            let mut pp_off = pp_body;
            for i in 0..pp_len.min(10) {
                let (_, pm, pv) = read_cbor_initial(&state_bytes, pp_off).unwrap();
                let ps = skip_cbor(&state_bytes, pp_off).unwrap() - pp_off;
                let pt = match pm { 0 => "uint", 6 => "tag", _ => "?" };
                eprintln!("      PP[{i}]: {pt}(val={pv}) size={ps}B");
                pp_off = skip_cbor(&state_bytes, pp_off).unwrap();
            }
        }

        // Deep probe the single governance proposal in P[1]
        {
            let off2 = navigate_to_nes(&state_bytes).unwrap();
            let es2 = skip_nes_to_epoch_state(&state_bytes, off2).unwrap();
            let off2 = skip_cbor(&state_bytes, es2).unwrap(); // skip AccountState
            let (ls_body, _) = read_array_header(&state_bytes, off2).unwrap();
            let utxo_off = skip_cbor(&state_bytes, ls_body).unwrap(); // skip CertState → UTxOState
            let (utxo_body, _) = read_array_header(&state_bytes, utxo_off).unwrap();
            // Skip UTxO[0], [1], [2] to GovState
            let gs_off = skip_cbor(&state_bytes,
                skip_cbor(&state_bytes,
                    skip_cbor(&state_bytes, utxo_body).unwrap()
                ).unwrap()
            ).unwrap();
            let (gs_body, _) = read_array_header(&state_bytes, gs_off).unwrap();
            // GS[0] = Proposals = array(2)
            let (prop_body, _) = read_array_header(&state_bytes, gs_body).unwrap();
            // P[0] = metadata, P[1] = proposals sequence
            let p1_off = skip_cbor(&state_bytes, prop_body).unwrap();
            let (p1_body, p1_len) = read_array_header(&state_bytes, p1_off).unwrap();
            eprintln!("\n  --- Governance Proposal Deep Probe ---");
            eprintln!("    P[1] = array({p1_len}) (proposals sequence)");

            // Each proposal entry
            let mut prop_off = p1_body;
            for pi in 0..p1_len.min(5) {
                let (_, pm, pv) = read_cbor_initial(&state_bytes, prop_off).unwrap();
                let ps = skip_cbor(&state_bytes, prop_off).unwrap() - prop_off;
                let pt = match pm { 4 => "array", 5 => "map", 6 => "tag", _ => "?" };
                eprintln!("    proposal[{pi}]: {pt}(val={pv}) size={ps}B");

                // If it's an array, show sub-elements (GovActionState fields)
                if pm == 4 {
                    let (mut gi, glen) = read_array_header(&state_bytes, prop_off).unwrap();
                    let field_names = ["gasId", "committeeVotes", "drepVotes", "spoVotes",
                                       "proposalProcedure", "proposedIn", "expiresAfter"];
                    for j in 0..glen.min(10) {
                        let (_, gm, gv) = read_cbor_initial(&state_bytes, gi).unwrap();
                        let gs = skip_cbor(&state_bytes, gi).unwrap() - gi;
                        let gt = match gm { 0 => "uint", 2 => "bytes", 3 => "text", 4 => "array", 5 => "map", 6 => "tag", _ => "?" };
                        let fname = field_names.get(j as usize).unwrap_or(&"?");
                        eprintln!("      [{j}] {fname}: {gt}(val={gv}) size={gs}B");

                        // Deep probe ProposalProcedure (field 4)
                        if j == 4 && gm == 4 {
                            let (mut pi2, plen) = read_array_header(&state_bytes, gi).unwrap();
                            let pp_names = ["deposit", "returnAddr", "govAction", "anchor"];
                            for k in 0..plen.min(5) {
                                let (_, km, kv) = read_cbor_initial(&state_bytes, pi2).unwrap();
                                let ks = skip_cbor(&state_bytes, pi2).unwrap() - pi2;
                                let kt = match km { 0 => "uint", 2 => "bytes", 4 => "array", 5 => "map", 6 => "tag", _ => "?" };
                                let ppn = pp_names.get(k as usize).unwrap_or(&"?");
                                eprintln!("        PP[{k}] {ppn}: {kt}(val={kv}) size={ks}B");

                                // Deep probe govAction
                                if k == 2 && km == 4 {
                                    let (mut ai, alen) = read_array_header(&state_bytes, pi2).unwrap();
                                    eprintln!("          govAction = array({alen}):");
                                    for l in 0..alen.min(5) {
                                        let (_, am, av) = read_cbor_initial(&state_bytes, ai).unwrap();
                                        let az = skip_cbor(&state_bytes, ai).unwrap() - ai;
                                        let at2 = match am { 0 => "uint", 2 => "bytes", 4 => "array", 5 => "map", 6 => "tag", _ => "?" };
                                        eprintln!("          GA[{l}]: {at2}(val={av}) size={az}B");

                                        // If first element is uint, it's the action tag
                                        if l == 0 && am == 0 {
                                            let action_name = match av {
                                                0 => "ParameterChange",
                                                1 => "HardForkInitiation",
                                                2 => "TreasuryWithdrawals",
                                                3 => "NoConfidence",
                                                4 => "UpdateCommittee",
                                                5 => "NewConstitution",
                                                6 => "InfoAction",
                                                _ => "Unknown",
                                            };
                                            eprintln!("          → ACTION TYPE: {action_name}");
                                        }

                                        // If TreasuryWithdrawals, GA[1] = map(RewardAccount → Coin)
                                        if l == 1 && am == 5 {
                                            eprintln!("          → Treasury withdrawal map with {av} entries");
                                            let (mut mi, _, _) = read_cbor_initial(&state_bytes, ai).unwrap();
                                            let mut total_withdrawal = 0u64;
                                            let mut w_count = 0u64;
                                            while mi < state_bytes.len() && state_bytes[mi] != 0xff {
                                                // key = RewardAccount (bytes)
                                                mi = skip_cbor(&state_bytes, mi).unwrap();
                                                // value = Coin (uint)
                                                let (next, coin_val) = read_uint(&state_bytes, mi).unwrap();
                                                mi = next;
                                                total_withdrawal += coin_val;
                                                w_count += 1;
                                                if w_count <= 5 {
                                                    eprintln!("            withdrawal[{}]: {} lovelace ({} ADA)",
                                                        w_count - 1, coin_val, coin_val / 1_000_000);
                                                }
                                            }
                                            eprintln!("          → TOTAL WITHDRAWAL: {} lovelace ({} ADA), {} recipients",
                                                total_withdrawal, total_withdrawal / 1_000_000, w_count);
                                        }
                                        // If map for definite-length
                                        if l == 1 && am == 5 && av != u64::MAX {
                                            let (mut mi, _, _) = read_cbor_initial(&state_bytes, ai).unwrap();
                                            let mut total_withdrawal = 0u64;
                                            for entry in 0..av {
                                                mi = skip_cbor(&state_bytes, mi).unwrap(); // key
                                                let (next, coin_val) = read_uint(&state_bytes, mi).unwrap();
                                                mi = next;
                                                total_withdrawal += coin_val;
                                                if entry < 5 {
                                                    eprintln!("            withdrawal[{entry}]: {} lovelace ({} ADA)",
                                                        coin_val, coin_val / 1_000_000);
                                                }
                                            }
                                            eprintln!("          → TOTAL WITHDRAWAL (definite): {} lovelace ({} ADA)",
                                                total_withdrawal, total_withdrawal / 1_000_000);
                                        }

                                        ai = skip_cbor(&state_bytes, ai).unwrap();
                                    }
                                }

                                pi2 = skip_cbor(&state_bytes, pi2).unwrap();
                            }
                        }

                        gi = skip_cbor(&state_bytes, gi).unwrap();
                    }
                }

                prop_off = skip_cbor(&state_bytes, prop_off).unwrap();
            }
        }

        // Now probe what Allegra CertState looks like for comparison
        let allegra_tarball = snapshots_dir().join("snapshot_17020848.tar.gz");
        if allegra_tarball.exists() {
            let allegra_bytes = extract_state_from_tarball(&allegra_tarball).unwrap();
            let aoff = navigate_to_nes(&allegra_bytes).unwrap();
            let aes = skip_nes_to_epoch_state(&allegra_bytes, aoff).unwrap();
            let aoff = skip_cbor(&allegra_bytes, aes).unwrap(); // skip AccountState
            let (als_body, als_len) = read_array_header(&allegra_bytes, aoff).unwrap();
            let aoff = skip_cbor(&allegra_bytes, als_body).unwrap(); // skip UTxOState
            let (_, acs_maj, acs_val) = read_cbor_initial(&allegra_bytes, aoff).unwrap();
            let acs_size = skip_cbor(&allegra_bytes, aoff).unwrap() - aoff;
            eprintln!("\n  --- Allegra CertState comparison ---");
            eprintln!("  LS length: {als_len}");
            eprintln!("  CertState: major={acs_maj} val={acs_val} size={}MB", acs_size / 1_000_000);
            if acs_maj == 4 {
                let (mut aelem, acs_len) = read_array_header(&allegra_bytes, aoff).unwrap();
                eprintln!("    array length: {acs_len}");
                for i in 0..acs_len.min(10) {
                    let (_, am, av) = read_cbor_initial(&allegra_bytes, aelem).unwrap();
                    let asz = skip_cbor(&allegra_bytes, aelem).unwrap() - aelem;
                    let at = match am { 0 => "uint", 2 => "bytes", 3 => "text", 4 => "array", 5 => "map", 6 => "tag", _ => "?" };
                    eprintln!("    ACS[{i}]: {at}(val={av}) size={}",
                        if asz > 1_000_000 { format!("{}MB", asz / 1_000_000) } else if asz > 1_000 { format!("{}KB", asz / 1_000) } else { format!("{asz}B") });
                    aelem = skip_cbor(&allegra_bytes, aelem).unwrap();
                }
            }
        }

        eprintln!("===================================\n");
    }

    #[test]
    fn conway_voting_delegation_stats() {
        let tarball = snapshots_dir().join("snapshot_133660855.tar.gz");
        if !tarball.exists() {
            eprintln!("Skipping: Conway pre-boundary snapshot not available");
            return;
        }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();
        // First, probe the UMElem encoding by looking at a few entries
        {
            let umap_off = navigate_to_umap_elems(&state_bytes).unwrap();
            let (mut co, _, _) = read_cbor_initial(&state_bytes, umap_off).unwrap();
            eprintln!("\n=== UMap Entry Probe ===");
            for i in 0..5 {
                if co >= state_bytes.len() || state_bytes[co] == 0xff { break; }
                // Key
                let (_, km, kv) = read_cbor_initial(&state_bytes, co).unwrap();
                let key_size = skip_cbor(&state_bytes, co).unwrap() - co;
                let kt = match km { 0 => "uint", 2 => "bytes", 4 => "array", 5 => "map", 6 => "tag", _ => "?" };
                eprintln!("  entry[{i}] key: {kt}(val={kv}) size={key_size}B");
                co = skip_cbor(&state_bytes, co).unwrap();

                // Value (UMElem)
                let (elem_body, em, ev) = read_cbor_initial(&state_bytes, co).unwrap();
                let val_size = skip_cbor(&state_bytes, co).unwrap() - co;
                let et = match em { 0 => "uint", 2 => "bytes", 4 => "array", 5 => "map", 6 => "tag", _ => "?" };
                eprintln!("  entry[{i}] val: {et}(val={ev}) size={val_size}B");

                // If it's an array, show sub-elements
                if em == 4 {
                    let mut si = elem_body;
                    for j in 0..(ev as u32).min(6) {
                        let (_, sm, sv) = read_cbor_initial(&state_bytes, si).unwrap();
                        let ss = skip_cbor(&state_bytes, si).unwrap() - si;
                        let st = match sm { 0 => "uint", 2 => "bytes", 4 => "array", 5 => "map", 6 => "tag", 7 => "special", _ => "?" };
                        eprintln!("    [{j}]: {st}(val={sv}) size={ss}B");
                        si = skip_cbor(&state_bytes, si).unwrap();
                    }
                }
                // If it's a map, show first few key-value pairs
                if em == 5 {
                    let (mut mi, _, _) = read_cbor_initial(&state_bytes, co).unwrap();
                    for j in 0..3 {
                        if mi >= state_bytes.len() || state_bytes[mi] == 0xff { break; }
                        let (_, mk, mkv) = read_cbor_initial(&state_bytes, mi).unwrap();
                        let mks = skip_cbor(&state_bytes, mi).unwrap() - mi;
                        mi = skip_cbor(&state_bytes, mi).unwrap();
                        let (_, mv, mvv) = read_cbor_initial(&state_bytes, mi).unwrap();
                        let mvs = skip_cbor(&state_bytes, mi).unwrap() - mi;
                        mi = skip_cbor(&state_bytes, mi).unwrap();
                        let mkt = match mk { 0 => "uint", 2 => "bytes", _ => "?" };
                        let mvt = match mv { 0 => "uint", 2 => "bytes", 4 => "array", _ => "?" };
                        eprintln!("    [{j}]: key={mkt}({mkv},{mks}B) → val={mvt}({mvv},{mvs}B)");
                    }
                }

                co = skip_cbor(&state_bytes, co).unwrap();
            }
            eprintln!("========================\n");
        }

        let stats = count_voting_delegations(&state_bytes).unwrap();

        eprintln!("\n=== Conway Voting Delegation Stats (epoch 507 pre-boundary) ===");
        eprintln!("  total credentials:    {}", stats.total_credentials);
        eprintln!("  with voting deleg:    {} ({:.1}%)",
            stats.with_voting,
            stats.with_voting as f64 / stats.total_credentials.max(1) as f64 * 100.0);
        eprintln!("  without voting deleg: {} ({:.1}%)",
            stats.without_voting,
            stats.without_voting as f64 / stats.total_credentials.max(1) as f64 * 100.0);
        eprintln!("  parse errors:         {}", stats.errors);
        eprintln!("===============================================================\n");

        assert!(stats.total_credentials > 100_000,
            "should have significant credentials, got {}", stats.total_credentials);
        assert_eq!(stats.errors, 0, "should have no parse errors");
    }

    #[test]
    fn conway_nes_reward_update_probe() {
        let tarball = snapshots_dir().join("snapshot_133660855.tar.gz");
        if !tarball.exists() {
            eprintln!("Skipping: Conway pre-boundary snapshot not available");
            return;
        }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();
        let nes_body = navigate_to_nes(&state_bytes).unwrap();

        // NES = array(7+) [epoch, bprev, bcur, ES, nesRu, stashed, pd]
        // Skip NES[0..3] to reach NES[4] = nesRu
        let mut off = nes_body;
        for i in 0..4 {
            let (_, m, v) = read_cbor_initial(&state_bytes, off).unwrap();
            let sz = skip_cbor(&state_bytes, off).unwrap() - off;
            let t = match m { 0 => "uint", 4 => "array", 5 => "map", 6 => "tag", 7 => "special", _ => "?" };
            eprintln!("  NES[{i}]: {t}(val={v}) size={}",
                if sz > 1_000_000 { format!("{}MB", sz / 1_000_000) } else if sz > 1_000 { format!("{}KB", sz / 1_000) } else { format!("{sz}B") });
            off = skip_cbor(&state_bytes, off).unwrap();
        }

        // NES[4] = nesRu = StrictMaybe (PulsingRewUpdate)
        let (ru_body, ru_maj, ru_val) = read_cbor_initial(&state_bytes, off).unwrap();
        let ru_size = skip_cbor(&state_bytes, off).unwrap() - off;
        let ru_type = match ru_maj { 0 => "uint", 4 => "array", 5 => "map", 6 => "tag", 7 => "special", _ => "?" };
        eprintln!("\n=== NES[4] nesRu (Reward Update) ===");
        eprintln!("  type: {ru_type}(val={ru_val}) size={}",
            if ru_size > 1_000_000 { format!("{}MB", ru_size / 1_000_000) } else if ru_size > 1_000 { format!("{}KB", ru_size / 1_000) } else { format!("{ru_size}B") });

        // If it's an array (StrictMaybe wrapped), probe contents
        if ru_maj == 4 && ru_val > 0 {
            let mut ri = ru_body;
            for i in 0..(ru_val as u32).min(10) {
                let (_, rm, rv) = read_cbor_initial(&state_bytes, ri).unwrap();
                let rs = skip_cbor(&state_bytes, ri).unwrap() - ri;
                let rt = match rm { 0 => "uint", 2 => "bytes", 4 => "array", 5 => "map", 6 => "tag", 7 => "special", _ => "?" };
                eprintln!("  RU[{i}]: {rt}(val={rv}) size={}",
                    if rs > 1_000_000 { format!("{}MB", rs / 1_000_000) } else if rs > 1_000 { format!("{}KB", rs / 1_000) } else { format!("{rs}B") });

                // If RU is a RewardUpdate, it has: deltaT, deltaR, rs (reward map), deltaF, nonMyopic
                // The reward map (rs) would be huge — that tells us total rewards
                if rm == 5 && rs > 1_000 {
                    // This is likely the rewards map: credential → Set(Reward)
                    let count = count_indef_map(&state_bytes, ri).unwrap_or(0);
                    eprintln!("    → map with {count} entries");

                    // Sum up rewards
                    let (mut mi, _, _) = read_cbor_initial(&state_bytes, ri).unwrap();
                    let mut total_reward = 0u128;
                    let mut sampled = 0usize;
                    while mi < state_bytes.len() && state_bytes[mi] != 0xff && sampled < 20 {
                        mi = skip_cbor(&state_bytes, mi).unwrap(); // key
                        // Value: Set(Reward) or just Reward
                        let (_, vm, vv) = read_cbor_initial(&state_bytes, mi).unwrap();
                        if vm == 0 {
                            total_reward += vv as u128;
                        }
                        mi = skip_cbor(&state_bytes, mi).unwrap();
                        sampled += 1;
                    }
                    if sampled > 0 {
                        eprintln!("    → first {sampled} rewards total: {} lovelace", total_reward);
                    }
                }

                // If it's a tagged/uint value, it might be deltaT or deltaR
                if rm == 0 {
                    eprintln!("    → uint value: {} ({} ADA)", rv, rv / 1_000_000);
                }
                // Negative int (major 1) for delta values
                if rm == 1 {
                    eprintln!("    → negint value: -{} (-{} ADA)", rv + 1, (rv + 1) / 1_000_000);
                }

                ri = skip_cbor(&state_bytes, ri).unwrap();
            }
        }

        // Also show NES[5] and NES[6]
        off = skip_cbor(&state_bytes, off).unwrap(); // skip NES[4]
        for i in 5..7 {
            if off >= state_bytes.len() { break; }
            let (_, m, v) = read_cbor_initial(&state_bytes, off).unwrap();
            let sz = skip_cbor(&state_bytes, off).unwrap() - off;
            let t = match m { 0 => "uint", 4 => "array", 5 => "map", 6 => "tag", 7 => "special", _ => "?" };
            eprintln!("  NES[{i}]: {t}(val={v}) size={}",
                if sz > 1_000_000 { format!("{}MB", sz / 1_000_000) } else if sz > 1_000 { format!("{}KB", sz / 1_000) } else { format!("{sz}B") });
            off = skip_cbor(&state_bytes, off).unwrap();
        }

        eprintln!("====================================\n");

        // Probe DRepPulsingState from ConwayGovState[6]
        // Path: ES[1] → LS[1] → UTxOState[3] → GovState[6]
        {
            let off = navigate_to_nes(&state_bytes).unwrap();
            let es = skip_nes_to_epoch_state(&state_bytes, off).unwrap();
            let off = skip_cbor(&state_bytes, es).unwrap(); // skip AccountState
            let (ls_body, _) = read_array_header(&state_bytes, off).unwrap();
            let utxo_off = skip_cbor(&state_bytes, ls_body).unwrap(); // skip CertState
            let (utxo_body, _) = read_array_header(&state_bytes, utxo_off).unwrap();
            // Skip UTxO[0..2] to GovState[3]
            let mut goff = utxo_body;
            for _ in 0..3 { goff = skip_cbor(&state_bytes, goff).unwrap(); }
            let (gs_body, _gs_len) = read_array_header(&state_bytes, goff).unwrap();

            // Skip GS[0..5] to reach GS[6] = DRepPulsingState
            let mut drp_off = gs_body;
            for _ in 0..6 { drp_off = skip_cbor(&state_bytes, drp_off).unwrap(); }

            let (drp_body, drp_maj, drp_val) = read_cbor_initial(&state_bytes, drp_off).unwrap();
            let drp_size = skip_cbor(&state_bytes, drp_off).unwrap() - drp_off;
            eprintln!("=== DRepPulsingState (GovState[6]) ===");
            eprintln!("  type: {}(val={drp_val}) size={}KB",
                match drp_maj { 4 => "array", 5 => "map", _ => "?" }, drp_size / 1_000);

            if drp_maj == 4 {
                let mut di = drp_body;
                for i in 0..(drp_val as u32).min(10) {
                    let (_, dm, dv) = read_cbor_initial(&state_bytes, di).unwrap();
                    let ds = skip_cbor(&state_bytes, di).unwrap() - di;
                    let dt = match dm { 0 => "uint", 1 => "negint", 2 => "bytes", 4 => "array", 5 => "map", 6 => "tag", 7 => "special", _ => "?" };
                    eprintln!("  DRP[{i}]: {dt}(val={dv}) size={}",
                        if ds > 1_000_000 { format!("{}MB", ds / 1_000_000) } else if ds > 1_000 { format!("{}KB", ds / 1_000) } else { format!("{ds}B") });

                    // Show uint values (likely deltaT, deltaR)
                    if dm == 0 {
                        eprintln!("    → {} lovelace ({} ADA)", dv, dv / 1_000_000);
                    }
                    if dm == 1 {
                        eprintln!("    → -{} lovelace (-{} ADA)", dv + 1, (dv + 1) / 1_000_000);
                    }

                    // If it's an array, show sub-elements
                    if dm == 4 && ds > 100 {
                        let (mut si, _, sv) = read_cbor_initial(&state_bytes, di).unwrap();
                        for j in 0..(sv as u32).min(8) {
                            let (_, sm, sv2) = read_cbor_initial(&state_bytes, si).unwrap();
                            let ss = skip_cbor(&state_bytes, si).unwrap() - si;
                            let st = match sm { 0 => "uint", 1 => "negint", 4 => "array", 5 => "map", 6 => "tag", _ => "?" };
                            eprintln!("    [{j}]: {st}(val={sv2}) size={}",
                                if ss > 1_000_000 { format!("{}MB", ss / 1_000_000) } else if ss > 1_000 { format!("{}KB", ss / 1_000) } else { format!("{ss}B") });
                            if sm == 0 { eprintln!("      → {} lovelace ({} ADA)", sv2, sv2 / 1_000_000); }
                            if sm == 1 { eprintln!("      → -{} lovelace (-{} ADA)", sv2 + 1, (sv2 + 1) / 1_000_000); }
                            si = skip_cbor(&state_bytes, si).unwrap();
                        }
                    }

                    di = skip_cbor(&state_bytes, di).unwrap();
                }
            }
            eprintln!("======================================\n");

            // Probe ALL 31 protocol parameters from CurPParams
            let mut pp_off2 = gs_body;
            for _ in 0..3 { pp_off2 = skip_cbor(&state_bytes, pp_off2).unwrap(); }
            let (pp_body2, pp_len2) = read_array_header(&state_bytes, pp_off2).unwrap();
            eprintln!("=== Conway CurPParams (all {pp_len2} fields) ===");
            let mut ppi = pp_body2;
            let pp_names = [
                "minFeeA", "minFeeB", "maxBlockBodySize", "maxTxSize", "maxBlockHeaderSize",
                "keyDeposit", "poolDeposit", "eMax", "nOpt", "a0",
                "rho", "tau", "d_deprecated", "extraEntropy_deprecated", "protocolVersion",
                "minPoolCost", "adaPerUTxOByte", "costModels", "executionPrices", "maxTxExUnits",
                "maxBlockExUnits", "maxValSize", "collateralPercent", "maxCollateralInputs",
                "poolVotingThresholds", "dRepVotingThresholds", "committeeMinSize",
                "committeeMaxTermLength", "govActionLifetime", "govActionDeposit", "dRepDeposit",
            ];
            for i in 0..pp_len2.min(31) {
                let (inner, pm, pv) = read_cbor_initial(&state_bytes, ppi).unwrap();
                let ps = skip_cbor(&state_bytes, ppi).unwrap() - ppi;
                let name = pp_names.get(i as usize).unwrap_or(&"?");

                if pm == 0 {
                    eprintln!("  PP[{i:2}] {name}: uint({pv})");
                } else if pm == 6 {
                    // Tag — likely rational tag(30, [num, den])
                    let (arr_body, _, _) = read_cbor_initial(&state_bytes, inner).unwrap();
                    if let Ok((_, num)) = read_uint(&state_bytes, arr_body) {
                        let den_off = skip_cbor(&state_bytes, arr_body).unwrap();
                        if let Ok((_, den)) = read_uint(&state_bytes, den_off) {
                            eprintln!("  PP[{i:2}] {name}: rational({num}/{den})");
                        } else {
                            eprintln!("  PP[{i:2}] {name}: tag({pv}) size={ps}B");
                        }
                    } else {
                        eprintln!("  PP[{i:2}] {name}: tag({pv}) size={ps}B");
                    }
                } else {
                    let pt = match pm { 2 => "bytes", 4 => "array", 5 => "map", _ => "?" };
                    eprintln!("  PP[{i:2}] {name}: {pt}(val={pv}) size={ps}B");
                }

                ppi = skip_cbor(&state_bytes, ppi).unwrap();
            }
            eprintln!("========================================\n");
        }
    }

    #[test]
    fn conway_block_production_epoch_comparison() {
        let pre_path = snapshots_dir().join("snapshot_133660855.tar.gz");
        let post_path = snapshots_dir().join("snapshot_134092810.tar.gz");
        if !pre_path.exists() || !post_path.exists() {
            eprintln!("Skipping: Conway snapshots not available");
            return;
        }

        let pre_bytes = extract_state_from_tarball(&pre_path).unwrap();
        let post_bytes = extract_state_from_tarball(&post_path).unwrap();

        let bp_pre = parse_block_production(&pre_bytes).unwrap();
        let bp_post = parse_block_production(&post_bytes).unwrap();

        let total_pre: u64 = bp_pre.values().sum();
        let total_post: u64 = bp_post.values().sum();
        let pools_pre = bp_pre.len();
        let pools_post = bp_post.len();

        eprintln!("\n=== Block Production Comparison ===");
        eprintln!("  PRE  bprev (epoch 506): {} blocks, {} pools", total_pre, pools_pre);
        eprintln!("  POST bprev (epoch 507): {} blocks, {} pools", total_post, pools_post);
        eprintln!();
        eprintln!("  eta_506 = {total_pre}/21600 = {:.6}", total_pre as f64 / 21600.0);
        eprintln!("  eta_507 = {total_post}/21600 = {:.6}", total_post as f64 / 21600.0);
        eprintln!();

        // Compute delta_r1 for both epochs using same reserves
        let header = parse_snapshot_header(&pre_bytes).unwrap();
        let reserves = header.reserves;
        eprintln!("  reserves: {} ({} ADA)", reserves, reserves / 1_000_000);

        // rho = 3/1000
        let dr1_506 = (reserves as u128 * 3 * total_pre as u128
            / (1000u128 * 21600)) as u64;
        let dr1_507 = (reserves as u128 * 3 * total_post as u128
            / (1000u128 * 21600)) as u64;

        eprintln!("  delta_r1 (epoch 506 blocks): {} ({} ADA)", dr1_506, dr1_506 / 1_000_000);
        eprintln!("  delta_r1 (epoch 507 blocks): {} ({} ADA)", dr1_507, dr1_507 / 1_000_000);
        eprintln!("  delta_r1 difference:         {} ({} ADA)",
            dr1_506.abs_diff(dr1_507), dr1_506.abs_diff(dr1_507) / 1_000_000);
        eprintln!();

        // Check pool overlap
        let pre_keys: std::collections::BTreeSet<_> = bp_pre.keys().collect();
        let post_keys: std::collections::BTreeSet<_> = bp_post.keys().collect();
        let both = pre_keys.intersection(&post_keys).count();
        let only_pre = pre_keys.difference(&post_keys).count();
        let only_post = post_keys.difference(&pre_keys).count();
        eprintln!("  pools in both epochs:    {both}");
        eprintln!("  pools only in epoch 506: {only_pre}");
        eprintln!("  pools only in epoch 507: {only_post}");
        eprintln!("====================================\n");
    }

    #[test]
    fn all_three_snapshots_load_allegra_237() {
        let tarball = snapshots_dir().join("snapshot_17020848.tar.gz");
        if !tarball.exists() { return; }

        let snap = LoadedSnapshot::from_tarball(&tarball).unwrap();
        let state = snap.to_ledger_state();

        let mark_d = state.epoch_state.snapshots.mark.0.delegations.len();
        let mark_p = state.epoch_state.snapshots.mark.0.pool_stakes.len();
        let set_d = state.epoch_state.snapshots.set.0.delegations.len();
        let set_p = state.epoch_state.snapshots.set.0.pool_stakes.len();
        let go_d = state.epoch_state.snapshots.go.0.delegations.len();
        let go_p = state.epoch_state.snapshots.go.0.pool_stakes.len();

        eprintln!("\n=== Snapshot Family (Allegra epoch 237) ===");
        eprintln!("  mark: {} delegations, {} pools", mark_d, mark_p);
        eprintln!("  set:  {} delegations, {} pools", set_d, set_p);
        eprintln!("  go:   {} delegations, {} pools", go_d, go_p);
        eprintln!("============================================\n");

        // All three should be populated (not just go)
        assert!(mark_d > 0, "mark snapshot should have delegations, got {mark_d}");
        assert!(set_d > 0, "set snapshot should have delegations, got {set_d}");
        assert!(go_d > 0, "go snapshot should have delegations, got {go_d}");

        // Sizes should differ between snapshots (different epochs)
        // mark > set > go is typical (newer snapshots have more delegations)
        eprintln!("  mark > set > go delegations: {} > {} > {}", mark_d, set_d, go_d);
    }

    #[test]
    fn snapshot_counts_match_oracle_summaries() {
        // Verify parsed counts against sub_state_summaries.toml for Allegra HFC
        let tarball = snapshots_dir().join("snapshot_16588800.tar.gz");
        if !tarball.exists() { return; }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();

        // Oracle: mark_snapshot_size = 10692071, set = 10316429, go = 9914946
        // These are byte sizes, not entry counts. But we can verify counts are reasonable.
        for (idx, name) in [(0, "mark"), (1, "set"), (2, "go")] {
            let (pools, stakes, delegs) = parse_snapshot_counts(&state_bytes, idx).unwrap();
            eprintln!("  {name}: {pools} pools, {stakes} stakes, {delegs} delegations");
            assert!(pools > 0, "{name} should have pools");
            assert!(stakes > 0, "{name} should have stakes");
            assert!(delegs > 0, "{name} should have delegations");
        }
    }

    /// Compare raw stake_dist sum vs reconstructed pool_stakes sum.
    ///
    /// The oracle's `totalActiveStake` is `sum(stake_dist values)` in the go snapshot.
    /// Our `total_stake` is `sum(pool_stakes)` where pool_stakes are aggregated from
    /// the intersection of stake_dist and delegations. If these differ, the gap
    /// explains the reward formula mismatch.
    #[test]
    fn stake_dist_sum_vs_pool_stakes_sum() {
        let snapshots = [
            ("Allegra PRE 236", "snapshot_16588800.tar.gz"),
            ("Allegra POST 237", "snapshot_17020848.tar.gz"),
            ("Mary    PRE 251", "snapshot_23068800.tar.gz"),
            ("Mary    POST 252", "snapshot_23500962.tar.gz"),
            ("Conway  PRE 507", "snapshot_133660855.tar.gz"),
            ("Conway  POST 508", "snapshot_134092810.tar.gz"),
        ];

        for (label, filename) in &snapshots {
            let tarball = snapshots_dir().join(filename);
            if !tarball.exists() {
                eprintln!("Skipping {label}: {filename} not available");
                continue;
            }

            let snap = LoadedSnapshot::from_tarball(&tarball).unwrap();
            let state = snap.to_ledger_state();

            // Raw stake_dist sum from go snapshot (oracle's totalActiveStake)
            let raw_stakes = parse_snapshot_stake_distribution(&snap.raw_cbor, 2).unwrap();
            let raw_sum: u128 = raw_stakes.iter().map(|(_, v)| *v as u128).sum();
            let raw_count = raw_stakes.len();

            // Our reconstructed pool_stakes sum
            let pool_sum: u128 = state.epoch_state.snapshots.go.0.pool_stakes
                .values().map(|c| c.0 as u128).sum();
            let pool_count = state.epoch_state.snapshots.go.0.pool_stakes.len();

            // Delegation count
            let deleg_count = state.epoch_state.snapshots.go.0.delegations.len();

            // Raw delegations from go snapshot
            let raw_delegs = parse_snapshot_delegations(&snap.raw_cbor, 2).unwrap();
            let raw_deleg_count = raw_delegs.len();

            // Credentials in stake_dist but NOT in delegations
            let deleg_set: std::collections::BTreeSet<[u8; 28]> = raw_delegs.iter()
                .map(|(h, _)| {
                    let mut k = [0u8; 28];
                    k.copy_from_slice(&h.0[..28]);
                    k
                }).collect();
            let stake_only_count = raw_stakes.iter().filter(|(h, _)| {
                let mut k = [0u8; 28];
                k.copy_from_slice(&h.0[..28]);
                !deleg_set.contains(&k)
            }).count();
            let stake_only_sum: u128 = raw_stakes.iter().filter(|(h, _)| {
                let mut k = [0u8; 28];
                k.copy_from_slice(&h.0[..28]);
                !deleg_set.contains(&k)
            }).map(|(_, v)| *v as u128).sum();

            // Also check SnapShots array length — Conway may have extra fields
            let snap_off = {
                let off = navigate_to_nes(&snap.raw_cbor).unwrap();
                let es = skip_nes_to_epoch_state(&snap.raw_cbor, off).unwrap();
                skip_n_fields(&snap.raw_cbor, es, 2).unwrap()
            };
            let (ss_inner, ss_len) = read_array_header(&snap.raw_cbor, snap_off).unwrap();
            // Probe each element type
            let mut ss_elem = ss_inner;
            let mut ss_fields = Vec::new();
            for i in 0..ss_len.min(10) {
                let (_, maj, val) = read_cbor_initial(&snap.raw_cbor, ss_elem).unwrap();
                let sz = skip_cbor(&snap.raw_cbor, ss_elem).unwrap() - ss_elem;
                let t = match maj { 0 => "uint", 4 => "array", 5 => "map", 6 => "tag", _ => "?" };
                ss_fields.push(format!("SS[{i}]={t}({val}),{}",
                    if sz > 1_000_000 { format!("{}MB", sz / 1_000_000) }
                    else if sz > 1_000 { format!("{}KB", sz / 1_000) }
                    else { format!("{sz}B") }));
                ss_elem = skip_cbor(&snap.raw_cbor, ss_elem).unwrap();
            }

            let delta = raw_sum.abs_diff(pool_sum);
            let pct = if raw_sum > 0 { delta as f64 / raw_sum as f64 * 100.0 } else { 0.0 };

            eprintln!("\n=== {label} — stake_dist vs pool_stakes ===");
            eprintln!("  SnapShots = array({ss_len}): {}", ss_fields.join(", "));
            eprintln!("  raw stake_dist sum:      {} ({} entries)", raw_sum, raw_count);
            eprintln!("  reconstructed pool sum:  {} ({} pools)", pool_sum, pool_count);
            eprintln!("  delta:                   {} ({:.6}%)", delta, pct);
            eprintln!("  raw delegations:         {raw_deleg_count}");
            eprintln!("  our delegations:         {deleg_count}");
            eprintln!("  stake-only (no deleg):   {stake_only_count} creds, {} lovelace", stake_only_sum);
            if delta > 0 {
                eprintln!("  delta in ADA:            {} ADA", delta / 1_000_000);
            }
            eprintln!("=============================================\n");
        }
    }

    /// Compare PRE vs POST nesBprev block counts for each era boundary.
    /// The reward pulser is initialized at boundary N→N+1 with nesBcur = epoch N blocks.
    /// After the boundary, nesBprev = epoch N blocks (what the pulser used).
    /// PRE snapshot nesBprev = epoch (N-1) blocks.
    /// POST snapshot nesBprev = epoch N blocks.
    #[test]
    fn block_production_pre_vs_post_all_eras() {
        let pairs = [
            ("Allegra 236→237", "snapshot_16588800.tar.gz", "snapshot_17020848.tar.gz"),
            ("Mary    251→252", "snapshot_23068800.tar.gz", "snapshot_23500962.tar.gz"),
            ("Alonzo  290→291", "snapshot_39916975.tar.gz", "snapshot_40348902.tar.gz"),
            ("Babbage 365→366", "snapshot_72316896.tar.gz", "snapshot_72748820.tar.gz"),
            ("Conway  507→508", "snapshot_133660855.tar.gz", "snapshot_134092810.tar.gz"),
        ];

        eprintln!("\n=== Block Production PRE vs POST nesBprev ===");
        for (label, pre_file, post_file) in &pairs {
            let pre_path = snapshots_dir().join(pre_file);
            let post_path = snapshots_dir().join(post_file);
            if !pre_path.exists() || !post_path.exists() {
                eprintln!("  {label}: SKIPPED");
                continue;
            }

            let pre_bytes = extract_state_from_tarball(&pre_path).unwrap();
            let post_bytes = extract_state_from_tarball(&post_path).unwrap();

            let pre_bp = parse_block_production(&pre_bytes).unwrap_or_default();
            let post_bp = parse_block_production(&post_bytes).unwrap_or_default();

            let pre_total: u64 = pre_bp.values().sum();
            let post_total: u64 = post_bp.values().sum();
            let pre_pools = pre_bp.len();
            let post_pools = post_bp.len();

            // Also get d from CBOR for eta computation
            let (d_num, d_den) = parse_decentralization_param(&pre_bytes).unwrap_or((0, 1));
            let d_val = d_num as f64 / d_den.max(1) as f64;
            let expected = ((1.0 - d_val) * 21600.0).floor() as u64;

            let eta_pre = if pre_total >= expected { 1.0 } else { pre_total as f64 / expected as f64 };
            let eta_post = if post_total >= expected { 1.0 } else { post_total as f64 / expected as f64 };

            eprintln!("  {label}:");
            eprintln!("    d = {d_num}/{d_den} = {d_val:.4}, expected = {expected}");
            eprintln!("    PRE  nesBprev: {pre_total:>6} blocks ({pre_pools} pools) → eta = {eta_pre:.6}");
            eprintln!("    POST nesBprev: {post_total:>6} blocks ({post_pools} pools) → eta = {eta_post:.6}");
            eprintln!("    delta:         {:>6} blocks", post_total as i64 - pre_total as i64);
        }
        eprintln!("=============================================\n");
    }

    /// Extract total reward account balances from DState.
    ///
    /// Pre-Conway: DState[0] = rewards map (credential → coin)
    /// Conway: DState[0] = UMap, entries = array(4) [reward, deposit, poolDeleg, voteDeleg]
    ///
    /// Navigation: ES[1] → LS[0] → CertState → DState → first field
    #[test]
    fn total_reward_balances_all_pre_snapshots() {
        let snapshots = [
            ("Allegra PRE  236", "snapshot_16588800.tar.gz"),
            ("Allegra POST 237", "snapshot_17020848.tar.gz"),
            ("Mary    PRE  251", "snapshot_23068800.tar.gz"),
            ("Mary    POST 252", "snapshot_23500962.tar.gz"),
            ("Conway  PRE  507", "snapshot_133660855.tar.gz"),
            ("Conway  POST 508", "snapshot_134092810.tar.gz"),
        ];

        eprintln!("\n=== Total Reward Balances from DState (PRE vs POST) ===");
        for (label, filename) in &snapshots {
            let path = snapshots_dir().join(filename);
            if !path.exists() { continue; }
            let bytes = extract_state_from_tarball(&path).unwrap();

            // Navigate: NES → ES → ES[1] = LS → LS[0] = CertState
            let off = navigate_to_nes(&bytes).unwrap();
            let es = skip_nes_to_epoch_state(&bytes, off).unwrap();
            let off = skip_cbor(&bytes, es).unwrap(); // skip ES[0] AccountState
            let (ls_body, _) = read_array_header(&bytes, off).unwrap(); // ES[1] = LS
            let (cs_body, cs_len) = read_array_header(&bytes, ls_body).unwrap(); // LS[0] = CertState

            // Probe CertState elements to find DState
            if cs_len == 2 {
                for ci in 0..2 {
                    let ci_off = if ci == 0 { cs_body } else { skip_cbor(&bytes, cs_body).unwrap() };
                    let (_, cm, cv) = read_cbor_initial(&bytes, ci_off).unwrap();
                    let csz = skip_cbor(&bytes, ci_off).unwrap() - ci_off;
                    let ct = match cm { 4=>"array", 5=>"map", _=>"?" };
                    eprintln!("    CS[{ci}]: {ct}(val={cv}) size={}",
                        if csz > 1_000_000 { format!("{}MB", csz/1_000_000) }
                        else if csz > 1_000 { format!("{}KB", csz/1_000) }
                        else { format!("{csz}B") });
                }
            }

            // Navigate to DState within CertState
            let ds_off = if cs_len == 3 {
                // Conway: [VState, PState, DState]
                let o = skip_cbor(&bytes, cs_body).unwrap();
                skip_cbor(&bytes, o).unwrap()
            } else if cs_len == 2 {
                // Try SECOND element (CertState might be [PState, DState])
                skip_cbor(&bytes, cs_body).unwrap()
            } else {
                eprintln!("  {label}: unexpected CertState length {cs_len}");
                continue;
            };

            // DState = array(N)
            let (ds_body, ds_maj, ds_len) = read_cbor_initial(&bytes, ds_off).unwrap();
            if ds_maj != 4 {
                eprintln!("  {label}: DState not array (major={ds_maj})");
                continue;
            }

            // Probe all DState fields to identify the rewards map
            let mut probe_off = ds_body;
            for i in 0..ds_len.min(6) {
                let (_, fm, fv) = read_cbor_initial(&bytes, probe_off).unwrap();
                let fsize = skip_cbor(&bytes, probe_off).unwrap() - probe_off;
                let ft = match fm { 0=>"uint", 4=>"array", 5=>"map", 6=>"tag", _=>"?" };
                let count = if fm == 5 { count_indef_map(&bytes, probe_off).unwrap_or(0) } else { 0 };
                eprintln!("    DS[{i}]: {ft}(val={fv}) size={} count={count}",
                    if fsize > 1_000_000 { format!("{}MB", fsize/1_000_000) }
                    else if fsize > 1_000 { format!("{}KB", fsize/1_000) }
                    else { format!("{fsize}B") });
                probe_off = skip_cbor(&bytes, probe_off).unwrap();
            }

            // Probe first entry of DS[0] (the unified/rewards structure)
            let (um_body, um_maj, um_val) = read_cbor_initial(&bytes, ds_body).unwrap();
            if um_maj == 4 && um_val == 2 {
                // array(2) = [credMap, ptrMap]
                let (cred_body, cred_maj, cred_val) = read_cbor_initial(&bytes, um_body).unwrap();
                let cred_size = skip_cbor(&bytes, um_body).unwrap() - um_body;
                eprintln!("    DS[0] inner: array(2), credMap: {}(val={}) size={}",
                    match cred_maj { 5=>"map", _=>"?" }, cred_val,
                    if cred_size > 1_000_000 { format!("{}MB", cred_size/1_000_000) }
                    else if cred_size > 1_000 { format!("{}KB", cred_size/1_000) }
                    else { format!("{cred_size}B") });

                // Probe first credential entry
                let (mut co, _, _) = read_cbor_initial(&bytes, um_body).unwrap();
                if co < bytes.len() && bytes[co] != 0xff {
                    let key_start = co;
                    co = skip_cbor(&bytes, co).unwrap();
                    let key_size = co - key_start;
                    let (val_body, val_maj, val_val) = read_cbor_initial(&bytes, co).unwrap();
                    let val_size = skip_cbor(&bytes, co).unwrap() - co;
                    let vt = match val_maj { 0=>"uint", 4=>"array", 5=>"map", 6=>"tag", _=>"?" };
                    eprintln!("    first cred entry: key={key_size}B → {vt}(val={val_val}) {val_size}B");
                    if val_maj == 4 && val_val <= 6 {
                        let mut si = val_body;
                        for i in 0..(val_val as u32).min(6) {
                            let (_, sm, sv) = read_cbor_initial(&bytes, si).unwrap();
                            let ss = skip_cbor(&bytes, si).unwrap() - si;
                            let st = match sm { 0=>"uint", 2=>"bytes", 4=>"array", 5=>"map", 7=>"special", _=>"?" };
                            eprintln!("      [{i}]: {st}(val={sv}) {ss}B");
                            si = skip_cbor(&bytes, si).unwrap();
                        }
                    }
                }
            }

            let (_first_body, first_maj, _first_val) = read_cbor_initial(&bytes, ds_body).unwrap();

            let mut total_rewards: u128 = 0;
            let mut count = 0u64;

            if first_maj == 5 {
                // Map — iterate entries
                let (mut co, _, _) = read_cbor_initial(&bytes, ds_body).unwrap();
                while co < bytes.len() && bytes[co] != 0xff {
                    co = skip_cbor(&bytes, co).unwrap(); // key
                    let (val_body, val_maj, val_val) = read_cbor_initial(&bytes, co).unwrap();
                    if val_maj == 0 {
                        total_rewards += val_val as u128;
                    } else if val_maj == 4 && val_val >= 2 {
                        // UMap entry: auto-detect format
                        let (_, f0_maj, f0_val) = read_cbor_initial(&bytes, val_body).unwrap();
                        if f0_maj == 0 {
                            // Conway: [0]=reward(uint)
                            total_rewards += f0_val as u128;
                        } else {
                            // Pre-Conway: [0]=rdpair(array), [1]=reward(uint)
                            let off1 = skip_cbor(&bytes, val_body).unwrap();
                            if let Ok((_, reward)) = read_uint(&bytes, off1) {
                                total_rewards += reward as u128;
                            }
                        }
                    }
                    count += 1;
                    co = skip_cbor(&bytes, co).unwrap();
                }
            } else if first_maj == 4 {
                // UMap = array(2) [umElems, umPtrs]
                let (umap_body, _) = read_array_header(&bytes, ds_body).unwrap();
                let (mut co, _, val) = read_cbor_initial(&bytes, umap_body).unwrap();
                let is_indef = val == u64::MAX;
                let limit = if is_indef { u64::MAX } else { val };
                for _ in 0..limit {
                    if co >= bytes.len() || (is_indef && bytes[co] == 0xff) { break; }
                    co = skip_cbor(&bytes, co).unwrap(); // key
                    let (elem_body, elem_maj, _) = read_cbor_initial(&bytes, co).unwrap();
                    let elem_end = skip_cbor(&bytes, co).unwrap();
                    if elem_maj == 4 {
                        // array(4) entry — check if [0] is uint (Conway) or array (pre-Conway)
                        let (_, f0_maj, f0_val) = read_cbor_initial(&bytes, elem_body).unwrap();
                        if f0_maj == 0 {
                            // Conway: [0]=reward(uint), [1]=deposit(uint)
                            total_rewards += f0_val as u128;
                        } else {
                            // Pre-Conway: [0]=rdpair(array), [1]=reward(uint)
                            let off1 = skip_cbor(&bytes, elem_body).unwrap();
                            if let Ok((_, reward)) = read_uint(&bytes, off1) {
                                total_rewards += reward as u128;
                            }
                        }
                    }
                    count += 1;
                    co = elem_end;
                }
            }

            let header = parse_snapshot_header(&bytes).unwrap();
            eprintln!("  {label}: DState=array({ds_len}), {count} creds, rewards={} ADA, treasury={} ADA",
                total_rewards / 1_000_000, header.treasury / 1_000_000);
        }
        eprintln!("=========================================\n");
    }

    /// Extract nesRu from mid-epoch state dumps.
    /// Mid-epoch states should have nesRu = SJust(...) with the in-progress
    /// reward computation, unlike boundary snapshots where nesRu is cleared.
    #[test]
    fn nesru_from_mid_epoch_dumps() {
        let dumps = [
            ("Allegra epoch 242 (60%)", "corpus/ext_ledger_state_dumps/allegra/slot_19440024.bin"),
            ("Mary    epoch 267 (60%)", "corpus/ext_ledger_state_dumps/mary/slot_30240073.bin"),
        ];

        eprintln!("\n=== nesRu from mid-epoch state dumps ===");
        for (label, relpath) in &dumps {
            let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..").join("..").join(relpath);
            if !path.exists() {
                eprintln!("  {label}: SKIPPED ({relpath} not found)");
                continue;
            }

            let data = std::fs::read(&path).unwrap();
            eprintln!("  {label}: loaded {} bytes", data.len());

            // .bin files are ExtLedgerState = array(2) [HeaderState, LedgerState]
            let (ext_body, ext_len) = read_array_header(&data, 0).unwrap();
            eprintln!("    outer: array({ext_len})");

            // Probe first element
            let (_, h_maj, h_val) = read_cbor_initial(&data, ext_body).unwrap();
            let h_size = skip_cbor(&data, ext_body).unwrap() - ext_body;
            eprintln!("    [0] HeaderState: {}(val={h_val}) size={}B",
                match h_maj { 4=>"array", _=>"?" }, h_size);

            let ls_start = skip_cbor(&data, ext_body).unwrap();
            let (_, ls_maj, ls_val) = read_cbor_initial(&data, ls_start).unwrap();
            let ls_size = data.len() - ls_start;
            eprintln!("    [1] LedgerState: {}(val={ls_val}) at offset {ls_start}, ~{ls_size}B",
                match ls_maj { 4=>"array", _=>"?" });

            // The LedgerState might be the telescope directly
            // or might need different navigation
            // Probe its first few elements
            if ls_maj == 4 {
                let (mut si, _) = read_array_header(&data, ls_start).unwrap();
                for i in 0..ls_val.min(5) as u32 {
                    let (_, sm, sv) = read_cbor_initial(&data, si).unwrap();
                    let ss = skip_cbor(&data, si).unwrap() - si;
                    let st = match sm { 0=>"uint", 4=>"array", 5=>"map", 6=>"tag", _=>"?" };
                    eprintln!("      LS[{i}]: {st}(val={sv}) size={}",
                        if ss > 1_000_000 { format!("{}MB", ss/1_000_000) }
                        else if ss > 1_000 { format!("{}KB", ss/1_000) }
                        else { format!("{ss}B") });
                    si = skip_cbor(&data, si).unwrap();
                }
            }

            // Try navigate_to_nes on the raw data (tarball-style)
            let nes_body = match navigate_to_nes(&data) {
                Ok(off) => { eprintln!("    navigate_to_nes (tarball): OK at {off}"); off }
                Err(_) => {
                    // Try from ls_start (telescope start)
                    match navigate_to_nes_from(&data, ls_start) {
                        Ok(off) => { eprintln!("    navigate_to_nes_from: OK at {off}"); off }
                        Err(e) => { eprintln!("    both navigations failed: {e}"); continue; }
                    }
                }
            };

            // NES[0] = epoch
            let (off, epoch) = read_uint(&data, nes_body).unwrap();
            eprintln!("    epoch: {epoch}");

            // Skip NES[1..3] to reach NES[4] = nesRu
            let off = skip_cbor(&data, off).unwrap(); // NES[1] bprev
            let off = skip_cbor(&data, off).unwrap(); // NES[2] bcur
            let off = skip_cbor(&data, off).unwrap(); // NES[3] EpochState

            // NES[4] = nesRu = StrictMaybe (PulsingRewUpdate)
            let (ru_body, ru_maj, ru_val) = read_cbor_initial(&data, off).unwrap();
            let ru_size = skip_cbor(&data, off).unwrap() - off;
            let ru_type = match ru_maj { 0=>"uint", 4=>"array", 5=>"map", 6=>"tag", 7=>"special", _=>"?" };

            eprintln!("    nesRu: {ru_type}(val={ru_val}) size={}",
                if ru_size > 1_000_000 { format!("{}MB", ru_size / 1_000_000) }
                else if ru_size > 1_000 { format!("{}KB", ru_size / 1_000) }
                else { format!("{ru_size}B") });

            // If nesRu is non-empty (array with elements), probe its contents
            if ru_maj == 4 && ru_val > 0 {
                eprintln!("    nesRu is SJust! Probing reward update...");
                let mut ri = ru_body;
                for i in 0..(ru_val as u32).min(10) {
                    let (_, rm, rv) = read_cbor_initial(&data, ri).unwrap();
                    let rs = skip_cbor(&data, ri).unwrap() - ri;
                    let rt = match rm { 0=>"uint", 1=>"negint", 4=>"array", 5=>"map", 6=>"tag", _=>"?" };
                    let rs_str = if rs > 1_000_000 { format!("{}MB", rs/1_000_000) }
                        else if rs > 1_000 { format!("{}KB", rs/1_000) }
                        else { format!("{rs}B") };
                    eprintln!("      RU[{i}]: {rt}(val={rv}) size={rs_str}");

                    // Show values for uint/negint fields
                    if rm == 0 {
                        eprintln!("        → {} lovelace ({} ADA)", rv, rv / 1_000_000);
                    }
                    if rm == 1 {
                        eprintln!("        → -{} lovelace (-{} ADA)", rv + 1, (rv + 1) / 1_000_000);
                    }
                    // Show map entry count
                    if rm == 5 {
                        let count = count_indef_map(&data, ri).unwrap_or(0);
                        eprintln!("        → {count} entries");
                    }
                    ri = skip_cbor(&data, ri).unwrap();
                }
            } else if ru_maj == 4 && ru_val == 0 {
                eprintln!("    nesRu is SNothing (empty array)");
            }
        }
        eprintln!("========================================\n");
    }

    #[test]
    fn mir_from_all_pre_snapshots() {
        let snapshots = [
            ("Allegra 236 PRE", "snapshot_16588800.tar.gz"),
            ("Mary    251 PRE", "snapshot_23068800.tar.gz"),
            ("Alonzo  290 PRE", "snapshot_39916975.tar.gz"),
            ("Babbage 365 PRE", "snapshot_72316896.tar.gz"),
            ("Conway  507 PRE", "snapshot_133660855.tar.gz"),
        ];

        eprintln!("\n=== InstantaneousRewards from PRE snapshots ===");
        for (label, filename) in &snapshots {
            let path = snapshots_dir().join(filename);
            if !path.exists() { continue; }
            let bytes = extract_state_from_tarball(&path).unwrap();
            match parse_instantaneous_rewards(&bytes) {
                Ok(mir) => {
                    let total_res = mir.reserves_to_accounts as i64 + mir.delta_reserves;
                    let total_trs = mir.treasury_to_accounts as i64 + mir.delta_treasury;
                    eprintln!("  {label}:");
                    eprintln!("    reserves→accounts: {} ({} entries, {} ADA)",
                        mir.reserves_to_accounts, mir.reserves_to_accounts_count,
                        mir.reserves_to_accounts / 1_000_000);
                    eprintln!("    treasury→accounts: {} ({} entries, {} ADA)",
                        mir.treasury_to_accounts, mir.treasury_to_accounts_count,
                        mir.treasury_to_accounts / 1_000_000);
                    eprintln!("    deltaReserves:     {} ({} ADA)", mir.delta_reserves,
                        mir.delta_reserves / 1_000_000);
                    eprintln!("    deltaTreasury:     {} ({} ADA)", mir.delta_treasury,
                        mir.delta_treasury / 1_000_000);
                    eprintln!("    net from reserves: {} ADA", total_res / 1_000_000);
                    eprintln!("    net from treasury: {} ADA", total_trs / 1_000_000);
                }
                Err(e) => eprintln!("  {label}: ERROR: {e}"),
            }
        }
        eprintln!("================================================\n");
    }

    #[test]
    fn treasury_values_all_pre_snapshots() {
        let snapshots = [
            ("Allegra 236 PRE ", "snapshot_16588800.tar.gz"),
            ("Allegra 237 POST", "snapshot_17020848.tar.gz"),
            ("Mary    251 PRE ", "snapshot_23068800.tar.gz"),
            ("Mary    252 POST", "snapshot_23500962.tar.gz"),
            ("Alonzo  290 PRE ", "snapshot_39916975.tar.gz"),
            ("Alonzo  291 POST", "snapshot_40348902.tar.gz"),
            ("Alonzo  310 PRE ", "snapshot_48557136.tar.gz"),
            ("Alonzo  311 POST", "snapshot_48989209.tar.gz"),
            ("Babbage 365 PRE ", "snapshot_72316896.tar.gz"),
            ("Babbage 366 POST", "snapshot_72748820.tar.gz"),
            ("Conway  507 PRE ", "snapshot_133660855.tar.gz"),
            ("Conway  508 POST", "snapshot_134092810.tar.gz"),
        ];

        eprintln!("\n=== Treasury & Reserves from PRE snapshots ===");
        for (label, filename) in &snapshots {
            let path = snapshots_dir().join(filename);
            if !path.exists() { continue; }
            let snap = LoadedSnapshot::from_tarball(&path).unwrap();
            let treasury = snap.header.treasury;
            let reserves = snap.header.reserves;
            eprintln!("  {label}: treasury={:>15} ({:>8} ADA)  reserves={:>18} ({:>12} ADA)",
                treasury, treasury / 1_000_000, reserves, reserves / 1_000_000);
        }
        eprintln!("=============================================\n");
    }

    /// Extract SS[3] (epoch fees) from PRE and POST snapshots for each boundary.
    #[test]
    fn epoch_fees_pre_vs_post() {
        let pairs = [
            ("Allegra 236→237", "snapshot_16588800.tar.gz", "snapshot_17020848.tar.gz"),
            ("Mary    251→252", "snapshot_23068800.tar.gz", "snapshot_23500962.tar.gz"),
            ("Alonzo  290→291", "snapshot_39916975.tar.gz", "snapshot_40348902.tar.gz"),
            ("Babbage 365→366", "snapshot_72316896.tar.gz", "snapshot_72748820.tar.gz"),
            ("Conway  507→508", "snapshot_133660855.tar.gz", "snapshot_134092810.tar.gz"),
        ];

        eprintln!("\n=== Epoch Fees (SS[3]) PRE vs POST ===");
        for (label, pre_file, post_file) in &pairs {
            let pre_path = snapshots_dir().join(pre_file);
            let post_path = snapshots_dir().join(post_file);

            let pre_fees = if pre_path.exists() {
                let bytes = extract_state_from_tarball(&pre_path).unwrap();
                parse_epoch_fees(&bytes).unwrap_or(0)
            } else { 0 };

            let post_fees = if post_path.exists() {
                let bytes = extract_state_from_tarball(&post_path).unwrap();
                parse_epoch_fees(&bytes).unwrap_or(0)
            } else { 0 };

            eprintln!("  {label}:");
            eprintln!("    PRE  SS[3]: {:>15} ({:>8} ADA)", pre_fees, pre_fees / 1_000_000);
            eprintln!("    POST SS[3]: {:>15} ({:>8} ADA)", post_fees, post_fees / 1_000_000);
            eprintln!("    delta:      {:>15} ({:>8} ADA)", post_fees as i64 - pre_fees as i64,
                (post_fees as i64 - pre_fees as i64) / 1_000_000);
        }
        eprintln!("======================================\n");
    }

    #[test]
    fn parse_d_from_all_snapshots() {
        let snapshots = [
            ("Allegra 237 (post)", "snapshot_17020848.tar.gz"),
            ("Allegra 251 (pre-Mary)", "snapshot_23068800.tar.gz"),
            ("Mary    252 (post)", "snapshot_23500962.tar.gz"),
            ("Mary    290 (pre-Alonzo)", "snapshot_39916975.tar.gz"),
            ("Alonzo  291 (post)", "snapshot_40348902.tar.gz"),
            ("Alonzo  365 (pre-Babbage)", "snapshot_72316896.tar.gz"),
            ("Babbage 366 (post)", "snapshot_72748820.tar.gz"),
            ("Babbage 507 (pre-Conway)", "snapshot_133660855.tar.gz"),
            ("Conway  508 (post)", "snapshot_134092810.tar.gz"),
        ];

        eprintln!("\n=== Decentralization parameter from CBOR ===");
        for (label, filename) in &snapshots {
            let tarball = snapshots_dir().join(filename);
            if !tarball.exists() {
                eprintln!("  {label}: SKIPPED");
                continue;
            }
            let state_bytes = extract_state_from_tarball(&tarball).unwrap();
            match parse_decentralization_param(&state_bytes) {
                Ok((num, den)) => {
                    let d = num as f64 / den as f64;
                    eprintln!("  {label}: d = {num}/{den} = {d:.6}");
                }
                Err(e) => {
                    eprintln!("  {label}: ERROR: {e}");
                }
            }
        }
        eprintln!("=============================================\n");
    }

    /// Probe the exact CBOR structure of each go snapshot field.
    ///
    /// Check whether `totalActiveStake` exists as a separate precomputed
    /// field in the snapshot, or is only derivable from the stake map.
    #[test]
    fn go_snapshot_field_structure() {
        let snapshots = [
            ("Allegra 237", "snapshot_17020848.tar.gz"),
            ("Conway  507", "snapshot_133660855.tar.gz"),
        ];

        for (label, filename) in &snapshots {
            let tarball = snapshots_dir().join(filename);
            if !tarball.exists() { continue; }
            let state_bytes = extract_state_from_tarball(&tarball).unwrap();

            // Navigate to go snapshot = SnapShots[2]
            let off = navigate_to_nes(&state_bytes).unwrap();
            let es = skip_nes_to_epoch_state(&state_bytes, off).unwrap();
            let ss_off = skip_n_fields(&state_bytes, es, 2).unwrap();
            let (ss_inner, ss_len) = read_array_header(&state_bytes, ss_off).unwrap();
            let go_off = skip_n_fields(&state_bytes, ss_inner, 2).unwrap();
            let (go_inner, go_len) = read_array_header(&state_bytes, go_off).unwrap();

            eprintln!("\n=== {label} go snapshot = array({go_len}) ===");

            let mut field_off = go_inner;
            for i in 0..go_len.min(10) {
                let (body, maj, val) = read_cbor_initial(&state_bytes, field_off).unwrap();
                let size = skip_cbor(&state_bytes, field_off).unwrap() - field_off;
                let type_name = match maj {
                    0 => "uint", 1 => "negint", 2 => "bytes", 3 => "text",
                    4 => "array", 5 => "map", 6 => "tag", 7 => "special", _ => "?"
                };
                let size_str = if size > 1_000_000 { format!("{}MB", size / 1_000_000) }
                    else if size > 1_000 { format!("{}KB", size / 1_000) }
                    else { format!("{size}B") };
                eprintln!("  [{i}]: {type_name}(val={val}) size={size_str}");

                // If it's a uint, show the value (might be totalActiveStake)
                if maj == 0 {
                    eprintln!("       → value: {val} ({} ADA)", val / 1_000_000);
                }

                // If it's a map, show entry count and first entry structure
                if maj == 5 || (maj == 5 && val == u64::MAX) {
                    let count = count_indef_map(&state_bytes, field_off).unwrap_or(0);
                    eprintln!("       → {count} entries");
                    // Probe first entry
                    let (mut mi, _, _) = read_cbor_initial(&state_bytes, field_off).unwrap();
                    if mi < state_bytes.len() && state_bytes[mi] != 0xff {
                        let (_, km, kv) = read_cbor_initial(&state_bytes, mi).unwrap();
                        let ks = skip_cbor(&state_bytes, mi).unwrap() - mi;
                        mi = skip_cbor(&state_bytes, mi).unwrap();
                        let (_, vm, vv) = read_cbor_initial(&state_bytes, mi).unwrap();
                        let vs = skip_cbor(&state_bytes, mi).unwrap() - mi;
                        let kt = match km { 0=>"uint", 2=>"bytes", 4=>"array", 6=>"tag", _=>"?" };
                        let vt = match vm { 0=>"uint", 2=>"bytes", 4=>"array", 6=>"tag", _=>"?" };
                        eprintln!("       first entry: key={kt}({kv},{ks}B) → val={vt}({vv},{vs}B)");
                    }
                }

                // If it's an array, probe sub-elements (might be [map, uint])
                if maj == 4 && val <= 5 {
                    let mut si = body;
                    for j in 0..(val as u32).min(5) {
                        let (_, sm, sv) = read_cbor_initial(&state_bytes, si).unwrap();
                        let ss = skip_cbor(&state_bytes, si).unwrap() - si;
                        let st = match sm { 0=>"uint", 5=>"map", 4=>"array", _=>"?" };
                        let ss_str = if ss > 1_000_000 { format!("{}MB", ss / 1_000_000) }
                            else if ss > 1_000 { format!("{}KB", ss / 1_000) }
                            else { format!("{ss}B") };
                        eprintln!("       [{j}]: {st}(val={sv}) size={ss_str}");
                        if sm == 0 {
                            eprintln!("            → uint value: {sv} ({} ADA)", sv / 1_000_000);
                        }
                        si = skip_cbor(&state_bytes, si).unwrap();
                    }
                }

                field_off = skip_cbor(&state_bytes, field_off).unwrap();
            }
            eprintln!("=============================================\n");
        }
    }
}
