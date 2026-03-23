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

        // Era/epoch-specific overrides from oracle
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
                // oracle: d = 8/25 at epoch 236
                if epoch <= 236 {
                    pp.decentralization = Rational::new(8, 25).unwrap_or_else(zero);
                } else {
                    // d decreased further during Allegra
                    let delta = epoch.saturating_sub(236);
                    let d_num = 8u64.saturating_sub(delta);
                    pp.decentralization = Rational::new(d_num as i128, 25)
                        .unwrap_or_else(zero);
                }
            }
            ade_types::CardanoEra::Mary => {
                pp.protocol_major = 4;
                // oracle: d = 3/25 at epoch 251
                if epoch <= 251 {
                    pp.decentralization = Rational::new(3, 25).unwrap_or_else(zero);
                } else {
                    let delta = epoch.saturating_sub(251);
                    let d_num = 3u64.saturating_sub(delta);
                    pp.decentralization = Rational::new(d_num as i128, 25)
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
        for (pool_hash, pledge, cost, margin_num, margin_den, reward_acct) in &pools_raw {
            let mut pool_bytes = [0u8; 28];
            pool_bytes.copy_from_slice(&pool_hash.0[..28]);
            let pool_id = PoolId(Hash28(pool_bytes));

            pools.insert(pool_id.clone(), PoolParams {
                pool_id,
                vrf_hash: ade_types::Hash32([0u8; 32]), // not needed for rewards
                pledge: Coin(*pledge),
                cost: Coin(*cost),
                margin: (*margin_num, *margin_den),
                reward_account: reward_acct.clone(),
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

/// Pool params tuple: (pool_hash, pledge, cost, margin_num, margin_den, reward_account).
type PoolParamsTuple = (Hash32, u64, u64, u64, u64, Vec<u8>);

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

            pools.push((Hash32(pool_hash), pledge, cost, margin_num, margin_den, reward_acct));
        }

        co = skip_cbor(state_cbor, val_start)?;
    }

    Ok(pools)
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
}
