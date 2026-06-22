//! BOOTSTRAP-CERTSTATE-PRODUCER — the hermetic assembly core (part 1).
//!
//! GREEN deterministic transformation: cardano-cli `pool-state` + `ledger-state` dstate.accounts
//! JSON (RED capture material, never a persistent Ade input) -> the canonical `CertState` the
//! existing `encode_cert_state` codec + the importer consume. This does NOT duplicate the test-only
//! `ade_testkit` snapshot loader (which parses the CBOR LedgerDB go-snapshot, with a zeroed VRF and
//! empty rewards): this is the LIVE cardano-cli surface, complete (real VRF + rewards +
//! registrations), promoting the off-repo 3M-preview-oracle extraction into a first-class tool.
//!
//! Pure / deterministic: no clock, rand, float-in-output, or I/O. `BTreeMap` only. Two fidelity
//! reconstructions are documented inline (margin decimal -> rational; reward_account header byte) —
//! both verified against cardano-node in the part-4 admission round-trip.

use std::collections::BTreeMap;

use ade_ledger::delegation::{CertState, DelegationState, PoolParams, PoolState};
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::{Coin, PoolId};
use ade_types::{EpochNo, Hash28, Hash32};
use serde_json::Value;

/// A structured, fail-closed extraction error — a malformed/missing field is NEVER silently defaulted
/// (a defaulted cert-state would defeat the producer's judge-reproducibility).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CertExtractError {
    NotAnObject(&'static str),
    MissingField(&'static str),
    BadHex(&'static str),
    BadNumber(&'static str),
    BadCredentialKey(String),
    BadPoolId(String),
}

fn hex_to_vec(s: &str, what: &'static str) -> Result<Vec<u8>, CertExtractError> {
    if s.len() % 2 != 0 {
        return Err(CertExtractError::BadHex(what));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| CertExtractError::BadHex(what)))
        .collect()
}

fn hex_to_hash28(s: &str, what: &'static str) -> Result<Hash28, CertExtractError> {
    let v = hex_to_vec(s, what)?;
    if v.len() != 28 {
        return Err(CertExtractError::BadHex(what));
    }
    let mut a = [0u8; 28];
    a.copy_from_slice(&v);
    Ok(Hash28(a))
}

fn hex_to_hash32(s: &str, what: &'static str) -> Result<Hash32, CertExtractError> {
    let v = hex_to_vec(s, what)?;
    if v.len() != 32 {
        return Err(CertExtractError::BadHex(what));
    }
    let mut a = [0u8; 32];
    a.copy_from_slice(&v);
    Ok(Hash32(a))
}

fn field<'a>(v: &'a Value, key: &'static str) -> Result<&'a Value, CertExtractError> {
    v.get(key).ok_or(CertExtractError::MissingField(key))
}

fn as_str<'a>(v: &'a Value, what: &'static str) -> Result<&'a str, CertExtractError> {
    v.as_str().ok_or(CertExtractError::BadNumber(what))
}

fn as_u64(v: &Value, what: &'static str) -> Result<u64, CertExtractError> {
    v.as_u64().ok_or(CertExtractError::BadNumber(what))
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a.max(1)
    } else {
        gcd(b, a % b)
    }
}

/// Reconstruct the exact `(numerator, denominator)` margin from cardano-cli's lossy DECIMAL render
/// (`spsMargin: 0.010`). serde_json normalizes to the shortest decimal string (`"0.01"`); we recover
/// the rational from that string (NOT from an f64, which would lose precision), then reduce. An
/// integer (`0`) is `(0, 1)`. (The CBOR snapshot carries the rational directly; this is the JSON-path
/// reconstruction, exercised by the part-4 round-trip against cardano-node.)
fn parse_margin(v: &Value) -> Result<(u64, u64), CertExtractError> {
    let s = match v {
        Value::Number(n) => n.to_string(),
        _ => return Err(CertExtractError::BadNumber("spsMargin")),
    };
    decimal_or_scientific_to_rational(&s).ok_or(CertExtractError::BadNumber("spsMargin"))
}

/// Reconstruct `(num, den)` from a cardano-cli margin render — DECIMAL (`"0.01"`, `"0"`, `"1"`) OR
/// SCIENTIFIC (`"1e-7"`, which serde_json emits for very small margins like the live `1.0E-7`). The
/// margins live in `[0,1]`, so a negative exponent grows the denominator. Fail-closed (`None`) on
/// overflow or malformed input.
fn decimal_or_scientific_to_rational(s: &str) -> Option<(u64, u64)> {
    // split off an optional base-10 exponent (e/E).
    let (mantissa, exp): (&str, i32) = match s.split_once(['e', 'E']) {
        Some((m, e)) => (m, e.parse().ok()?),
        None => (s, 0),
    };
    // the mantissa decimal -> (num, den).
    let (mut num, mut den): (u64, u64) = match mantissa.split_once('.') {
        None => (mantissa.parse().ok()?, 1),
        Some((int_part, frac)) => {
            let digits = format!("{int_part}{frac}");
            (digits.parse().ok()?, 10u64.checked_pow(frac.len() as u32)?)
        }
    };
    if exp >= 0 {
        num = num.checked_mul(10u64.checked_pow(exp as u32)?)?;
    } else {
        den = den.checked_mul(10u64.checked_pow(exp.unsigned_abs())?)?;
    }
    if num == 0 {
        return Some((0, 1));
    }
    let g = gcd(num, den);
    Some((num / g, den / g))
}

/// Build the reward-account bytes from cardano-cli `spsAccountId` (`{keyHash|scriptHash: <hex>}`).
/// A stake-address reward account is `[header] ++ hash28`, header `0xe0 | network` (key-hash) or
/// `0xf0 | network` (script-hash); `network` = 0 for the testnets (preview/preprod), 1 for mainnet.
fn parse_reward_account(account_id: &Value, network: u8) -> Result<Vec<u8>, CertExtractError> {
    let obj = account_id
        .as_object()
        .ok_or(CertExtractError::NotAnObject("spsAccountId"))?;
    let (header, hex) = if let Some(k) = obj.get("keyHash") {
        (0xe0u8 | (network & 0x0f), as_str(k, "keyHash")?)
    } else if let Some(sh) = obj.get("scriptHash") {
        (0xf0u8 | (network & 0x0f), as_str(sh, "scriptHash")?)
    } else {
        return Err(CertExtractError::MissingField("spsAccountId.keyHash|scriptHash"));
    };
    let hash = hex_to_hash28(hex, "reward_account")?;
    let mut out = Vec::with_capacity(29);
    out.push(header);
    out.extend_from_slice(&hash.0);
    Ok(out)
}

fn parse_pool_params(
    pool_id: PoolId,
    pp: &Value,
    network: u8,
) -> Result<PoolParams, CertExtractError> {
    let vrf_hash = hex_to_hash32(as_str(field(pp, "spsVrf")?, "spsVrf")?, "spsVrf")?;
    let pledge = Coin(as_u64(field(pp, "spsPledge")?, "spsPledge")?);
    let cost = Coin(as_u64(field(pp, "spsCost")?, "spsCost")?);
    let margin = parse_margin(field(pp, "spsMargin")?)?;
    let reward_account = parse_reward_account(field(pp, "spsAccountId")?, network)?;
    let owners = field(pp, "spsOwners")?
        .as_array()
        .ok_or(CertExtractError::MissingField("spsOwners"))?
        .iter()
        .map(|o| hex_to_hash28(as_str(o, "owner")?, "owner"))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PoolParams {
        pool_id,
        vrf_hash,
        pledge,
        cost,
        margin,
        reward_account,
        owners,
    })
}

/// Parse the cardano-cli `pool-state --all-stake-pools` JSON (`<PoolId hex> -> {poolParams,
/// futurePoolParams, retiring}`) into the active/future/retiring pool maps.
pub fn parse_pool_state(json: &Value, network: u8) -> Result<PoolState, CertExtractError> {
    let obj = json
        .as_object()
        .ok_or(CertExtractError::NotAnObject("pool-state"))?;
    let mut pools = BTreeMap::new();
    let mut future_pools = BTreeMap::new();
    let mut retiring = BTreeMap::new();
    for (pool_hex, entry) in obj {
        let pool_id = PoolId(hex_to_hash28(pool_hex, "poolId")?);
        let pp = parse_pool_params(pool_id.clone(), field(entry, "poolParams")?, network)?;
        pools.insert(pool_id.clone(), pp);
        if let Some(fpp) = entry.get("futurePoolParams") {
            if !fpp.is_null() {
                future_pools.insert(pool_id.clone(), parse_pool_params(pool_id.clone(), fpp, network)?);
            }
        }
        if let Some(ret) = entry.get("retiring") {
            if !ret.is_null() {
                retiring.insert(pool_id, EpochNo(as_u64(ret, "retiring")?));
            }
        }
    }
    Ok(PoolState {
        pools,
        future_pools,
        retiring,
    })
}

/// Parse a `keyHash-<hex>` / `scriptHash-<hex>` dstate.accounts key into a `StakeCredential`.
fn parse_credential(key: &str) -> Result<StakeCredential, CertExtractError> {
    if let Some(h) = key.strip_prefix("keyHash-") {
        Ok(StakeCredential::KeyHash(hex_to_hash28(h, "cred")?))
    } else if let Some(h) = key.strip_prefix("scriptHash-") {
        Ok(StakeCredential::ScriptHash(hex_to_hash28(h, "cred")?))
    } else {
        Err(CertExtractError::BadCredentialKey(key.to_string()))
    }
}

/// Parse the cardano-cli `ledger-state` `…delegationState.dstate.accounts` JSON
/// (`keyHash-/scriptHash-<hex> -> {deposit, reward, spool, ...}`) into delegation state:
/// registrations = `cred -> deposit`, delegations = `cred -> spool` (where non-null), rewards =
/// `cred -> reward`.
pub fn parse_dstate_accounts(accounts: &Value) -> Result<DelegationState, CertExtractError> {
    let obj = accounts
        .as_object()
        .ok_or(CertExtractError::NotAnObject("dstate.accounts"))?;
    let mut registrations = BTreeMap::new();
    let mut delegations = BTreeMap::new();
    let mut rewards = BTreeMap::new();
    for (cred_key, acct) in obj {
        let cred = parse_credential(cred_key)?;
        registrations.insert(cred.clone(), Coin(as_u64(field(acct, "deposit")?, "deposit")?));
        rewards.insert(cred.clone(), Coin(as_u64(field(acct, "reward")?, "reward")?));
        if let Some(spool) = acct.get("spool") {
            if !spool.is_null() {
                delegations.insert(cred, PoolId(hex_to_hash28(as_str(spool, "spool")?, "spool")?));
            }
        }
    }
    Ok(DelegationState {
        registrations,
        delegations,
        rewards,
    })
}

/// Assemble the full `CertState` from the two point-consistent cardano-cli captures.
pub fn assemble_cert_state(
    pool_state_json: &Value,
    dstate_accounts_json: &Value,
    network: u8,
) -> Result<CertState, CertExtractError> {
    Ok(CertState {
        delegation: parse_dstate_accounts(dstate_accounts_json)?,
        pool: parse_pool_state(pool_state_json, network)?,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use ade_ledger::snapshot::cert_state::{decode_cert_state, encode_cert_state};

    // A representative pool-state fixture: one active pool, one with a future re-registration, one
    // retiring — covering pools / future_pools / retiring / VRF / margin / reward-account / owners.
    fn pool_state_fixture() -> Value {
        serde_json::json!({
            "11111111111111111111111111111111111111111111111111111111": {
                "futurePoolParams": null,
                "poolParams": {
                    "spsVrf": "52a8535d6b2e69025d188d13c10c3940a1ead314ca67cd9b400b3e36472164e0",
                    "spsMargin": 0.075,
                    "spsPledge": 500000000,
                    "spsCost": 340000000,
                    "spsAccountId": {"keyHash": "0470daa17236a4291be26c24d9b4bb9ed023e282077572458cdfcf1a"},
                    "spsOwners": ["0470daa17236a4291be26c24d9b4bb9ed023e282077572458cdfcf1a"]
                },
                "retiring": null
            },
            "22222222222222222222222222222222222222222222222222222222": {
                "futurePoolParams": {
                    "spsVrf": "aa00535d6b2e69025d188d13c10c3940a1ead314ca67cd9b400b3e3647216400",
                    "spsMargin": 0,
                    "spsPledge": 1000,
                    "spsCost": 2000,
                    "spsAccountId": {"scriptHash": "0470daa17236a4291be26c24d9b4bb9ed023e282077572458cdfcf1a"},
                    "spsOwners": []
                },
                "poolParams": {
                    "spsVrf": "bb00535d6b2e69025d188d13c10c3940a1ead314ca67cd9b400b3e3647216400",
                    "spsMargin": 0.01,
                    "spsPledge": 7,
                    "spsCost": 8,
                    "spsAccountId": {"keyHash": "0470daa17236a4291be26c24d9b4bb9ed023e282077572458cdfcf1a"},
                    "spsOwners": ["1170daa17236a4291be26c24d9b4bb9ed023e282077572458cdfcf1a"]
                },
                "retiring": 1340
            }
        })
    }

    fn dstate_fixture() -> Value {
        serde_json::json!({
            "keyHash-0000c862b7cda2e46b3b0eb5f17d1de67d8dd26fe7cc9f91a044bea6": {
                "balance": 18412, "deposit": 2000000, "drep": "drep-alwaysNoConfidence",
                "reward": 18412, "spool": "f3e0e5c79aca2a3e46640ed57d2a649c7e9c32ce7574111053f7d4e7"
            },
            "keyHash-0000908237cf59a50f9dfd6b2af546a75485ab8dd88fd2f339db6e5f": {
                "balance": 0, "deposit": 2000000, "drep": "drep-alwaysAbstain", "reward": 0, "spool": null
            },
            "scriptHash-001d616a180b28a3934373020d4b8ed5615f0bf9bffba8d6b55eca25": {
                "balance": 5, "deposit": 2000000, "drep": "drep-alwaysAbstain", "reward": 5,
                "spool": "f3e0e5c79aca2a3e46640ed57d2a649c7e9c32ce7574111053f7d4e7"
            }
        })
    }

    #[test]
    fn assembles_all_six_cert_state_components_and_round_trips_canonically() {
        let cs = assemble_cert_state(&pool_state_fixture(), &dstate_fixture(), 0).expect("assemble");

        // pools / future_pools / retiring
        assert_eq!(cs.pool.pools.len(), 2, "two active pools");
        assert_eq!(cs.pool.future_pools.len(), 1, "one staged re-registration");
        assert_eq!(cs.pool.retiring.len(), 1, "one retiring pool");
        assert_eq!(cs.pool.retiring.values().next(), Some(&EpochNo(1340)));

        // delegations / rewards / registrations (3 accounts; 2 delegated)
        assert_eq!(cs.delegation.registrations.len(), 3);
        assert_eq!(cs.delegation.delegations.len(), 2, "two delegated creds (spool != null)");
        assert_eq!(cs.delegation.rewards.len(), 3);

        // VRF fidelity (real, not zeroed like the test-corpus snapshot loader)
        let p1 = cs.pool.pools.values().next().unwrap();
        assert_eq!(
            p1.vrf_hash,
            Hash32(hex_to_hash32(
                "52a8535d6b2e69025d188d13c10c3940a1ead314ca67cd9b400b3e36472164e0", "x"
            ).unwrap().0)
        );

        // margin reconstruction: 0.075 -> 75/1000 -> 3/40; 0.01 -> 1/100
        let margins: Vec<(u64, u64)> = cs.pool.pools.values().map(|p| p.margin).collect();
        assert!(margins.contains(&(3, 40)), "0.075 -> 3/40, got {margins:?}");
        assert!(margins.contains(&(1, 100)), "0.01 -> 1/100, got {margins:?}");

        // reward_account: key-hash header 0xe0 (network 0) ++ 28-byte hash = 29 bytes
        assert_eq!(p1.reward_account.len(), 29);
        assert_eq!(p1.reward_account[0], 0xe0);

        // CANONICAL ROUND-TRIP through the EXISTING codec (the importer reads exactly this).
        let bytes = encode_cert_state(&cs);
        let decoded = decode_cert_state(&bytes).expect("decode");
        assert_eq!(decoded, cs, "assembled CertState round-trips byte-canonically");
    }

    #[test]
    fn fail_closed_on_malformed_fields_never_defaults() {
        // bad VRF length
        let mut bad = pool_state_fixture();
        bad["11111111111111111111111111111111111111111111111111111111"]["poolParams"]["spsVrf"] =
            serde_json::json!("dead");
        assert!(matches!(parse_pool_state(&bad, 0), Err(CertExtractError::BadHex(_))));

        // unknown credential prefix
        let bad_cred = serde_json::json!({ "ptr-1-2-3": {"deposit": 1, "reward": 0, "spool": null} });
        assert!(matches!(
            parse_dstate_accounts(&bad_cred),
            Err(CertExtractError::BadCredentialKey(_))
        ));
    }

    #[test]
    fn margin_reconstruction_is_exact_for_decimal_and_scientific_renders() {
        let cases = [
            (serde_json::json!(0), (0, 1)),
            (serde_json::json!(0.03), (3, 100)),
            (serde_json::json!(0.075), (3, 40)),
            (serde_json::json!(0.1), (1, 10)),
            (serde_json::json!(1), (1, 1)),
            // the LIVE preview finding: a tiny margin cardano-cli renders in scientific notation
            // (`1.0E-7`); serde re-renders to `1e-7`, which the decimal-only parser used to reject.
            (serde_json::from_str::<Value>("1.0E-7").unwrap(), (1, 10_000_000)),
            (serde_json::from_str::<Value>("0.9999").unwrap(), (9999, 10000)),
        ];
        for (v, expected) in cases {
            assert_eq!(parse_margin(&v).unwrap(), expected, "margin {v}");
        }
    }
}
