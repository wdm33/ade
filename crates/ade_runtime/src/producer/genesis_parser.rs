// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED Shelley genesis closed-contract parser (PHASE4-N-R-C C2).
//!
//! Parses a real Cardano `shelley-genesis.json` (the file
//! cardano-node reads at startup) and produces a canonical
//! `GenesisAnchor`. The fields N-Q's `GenesisAnchor` requires
//! all live in Shelley genesis (per N-R-A A1 OQ7 capture);
//! Conway genesis carries governance parameters not consumed
//! by `GenesisAnchor`.
//!
//! **Closed parser contract** (per DQ-C2):
//!
//! - Required fields fail-closed on missing / malformed /
//!   wrong-type input.
//! - No implicit defaults ("if missing, assume preprod"
//!   rejected).
//! - No stringly fallback (`"1"` rejected for u32 fields).
//! - Extra keys accepted-and-ignored for forward compatibility,
//!   iff inert. The `GenesisAnchor` produced by an extra-key
//!   fixture MUST byte-equal the `GenesisAnchor` produced by
//!   the canonical fixture.

use ade_runtime_genesis_export::GenesisAnchor;

mod ade_runtime_genesis_export {
    pub use crate::producer::coordinator::GenesisAnchor;
}

/// Closed parser error surface. No `String` payloads in the
/// load-bearing parts; the `field` discriminants are
/// `&'static str` from a closed list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenesisParseError {
    JsonShape,
    MissingRequiredField { name: &'static str },
    MalformedFieldType { name: &'static str },
    MalformedFieldValue { name: &'static str },
    SystemStartParseFailure,
}

/// Parse a Shelley genesis JSON into a canonical
/// `GenesisAnchor`.
///
/// `kes_anchor_slot` is operator-supplied (the slot at which
/// the operator generated their KES seed) and is NOT in the
/// genesis file. The caller provides it.
pub fn parse_shelley_genesis(
    json_bytes: &[u8],
    kes_anchor_slot: u64,
) -> Result<GenesisAnchor, GenesisParseError> {
    let json: serde_json::Value =
        serde_json::from_slice(json_bytes).map_err(|_| GenesisParseError::JsonShape)?;
    let obj = json.as_object().ok_or(GenesisParseError::JsonShape)?;

    let network_magic = require_u32(obj, "networkMagic")?;
    let system_start =
        require_str(obj, "systemStart")?;
    let slot_zero_time_unix_ms = parse_iso8601_to_unix_ms(&system_start)
        .ok_or(GenesisParseError::SystemStartParseFailure)?;
    // slotLength is "seconds" (may be u64 or possibly fractional in
    // some fixtures; reject fractional per no-floats discipline).
    let slot_length_s = require_u64(obj, "slotLength")?;
    let slot_length_ms = slot_length_s
        .checked_mul(1000)
        .ok_or(GenesisParseError::MalformedFieldValue {
            name: "slotLength",
        })?;
    let slots_per_kes_period = require_u64(obj, "slotsPerKESPeriod")?;
    let kes_max_period = require_u32(obj, "maxKESEvolutions")?;

    Ok(GenesisAnchor {
        network_magic,
        slot_zero_time_unix_ms,
        slot_length_ms,
        slots_per_kes_period,
        kes_anchor_slot,
        kes_max_period,
    })
}

// =========================================================================
// Closed required-field accessors
// =========================================================================

fn require_u32(
    obj: &serde_json::Map<String, serde_json::Value>,
    name: &'static str,
) -> Result<u32, GenesisParseError> {
    let v = obj.get(name).ok_or(GenesisParseError::MissingRequiredField { name })?;
    // Strict numeric check — reject string fallbacks (DQ-C2).
    if !v.is_number() {
        return Err(GenesisParseError::MalformedFieldType { name });
    }
    let n = v.as_u64().ok_or(GenesisParseError::MalformedFieldValue { name })?;
    if n > u32::MAX as u64 {
        return Err(GenesisParseError::MalformedFieldValue { name });
    }
    Ok(n as u32)
}

fn require_u64(
    obj: &serde_json::Map<String, serde_json::Value>,
    name: &'static str,
) -> Result<u64, GenesisParseError> {
    let v = obj.get(name).ok_or(GenesisParseError::MissingRequiredField { name })?;
    if !v.is_number() {
        return Err(GenesisParseError::MalformedFieldType { name });
    }
    // Reject negative numbers explicitly — as_u64 returns None for
    // negative serde_json::Number values, surface as MalformedFieldValue.
    v.as_u64().ok_or(GenesisParseError::MalformedFieldValue { name })
}

fn require_str(
    obj: &serde_json::Map<String, serde_json::Value>,
    name: &'static str,
) -> Result<String, GenesisParseError> {
    let v = obj.get(name).ok_or(GenesisParseError::MissingRequiredField { name })?;
    v.as_str()
        .map(|s| s.to_string())
        .ok_or(GenesisParseError::MalformedFieldType { name })
}

/// Parse `YYYY-MM-DDTHH:MM:SSZ` (the cardano-cli `systemStart`
/// format) into Unix epoch milliseconds. Deterministic
/// computation — no chrono/time dependency.
///
/// Algorithm (closed cases):
/// 1. Tokenize on `T` and `Z`; require exact suffix `Z`.
/// 2. Parse date as `YYYY-MM-DD`.
/// 3. Parse time as `HH:MM:SS`.
/// 4. Compute days from epoch via the proleptic Gregorian
///    calendar.
/// 5. Combine days + seconds → milliseconds.
fn parse_iso8601_to_unix_ms(s: &str) -> Option<u64> {
    if !s.ends_with('Z') {
        return None;
    }
    let body = &s[..s.len() - 1];
    let (date_str, time_str) = body.split_once('T')?;
    let date_parts: Vec<&str> = date_str.split('-').collect();
    if date_parts.len() != 3 {
        return None;
    }
    let year: i64 = date_parts[0].parse().ok()?;
    let month: u32 = date_parts[1].parse().ok()?;
    let day: u32 = date_parts[2].parse().ok()?;
    let time_parts: Vec<&str> = time_str.split(':').collect();
    if time_parts.len() != 3 {
        return None;
    }
    let hour: u64 = time_parts[0].parse().ok()?;
    let minute: u64 = time_parts[1].parse().ok()?;
    let second: u64 = time_parts[2].parse().ok()?;
    if month < 1 || month > 12 || day < 1 || day > 31 || hour >= 24 || minute >= 60 || second >= 60 {
        return None;
    }
    let days = days_since_unix_epoch(year, month, day)?;
    let seconds = (days as u64).checked_mul(86_400)?
        .checked_add(hour * 3600)?
        .checked_add(minute * 60)?
        .checked_add(second)?;
    seconds.checked_mul(1000)
}

fn days_since_unix_epoch(year: i64, month: u32, day: u32) -> Option<i64> {
    // Proleptic Gregorian; epoch = 1970-01-01.
    // Use Howard Hinnant's days_from_civil algorithm.
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as i64; // [0, 399]
    let m = month as i64;
    let d = day as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(days)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    const FIXTURE_DIR: &str = "tests/fixtures/conway_genesis";

    fn fixture_bytes(name: &str) -> Vec<u8> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(FIXTURE_DIR)
            .join(name);
        std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {:?}", path.display(), e))
    }

    #[test]
    fn accepted_shelley_genesis_parses_to_expected_anchor() {
        let bytes = fixture_bytes("accepted-shelley-genesis.json");
        let anchor = parse_shelley_genesis(&bytes, /*kes_anchor_slot=*/ 0)
            .expect("accepted fixture parses");

        // Per OQ7 fixture metadata:
        // - networkMagic = 1
        // - systemStart = "2022-06-01T00:00:00Z" → 1654041600000 ms
        // - slotLength = 1 second → 1000 ms
        // - slotsPerKESPeriod = 129600
        // - maxKESEvolutions = 62
        assert_eq!(anchor.network_magic, 1);
        assert_eq!(anchor.slot_zero_time_unix_ms, 1_654_041_600_000);
        assert_eq!(anchor.slot_length_ms, 1_000);
        assert_eq!(anchor.slots_per_kes_period, 129_600);
        assert_eq!(anchor.kes_max_period, 62);
        assert_eq!(anchor.kes_anchor_slot, 0);
    }

    #[test]
    fn missing_required_field_emits_structured_error() {
        let bytes = fixture_bytes("missing-required.shelley-genesis.json");
        let err = parse_shelley_genesis(&bytes, 0).unwrap_err();
        assert_eq!(
            err,
            GenesisParseError::MissingRequiredField {
                name: "networkMagic"
            }
        );
    }

    #[test]
    fn stringly_int_emits_malformed_field_type() {
        let bytes = fixture_bytes("stringly-int.shelley-genesis.json");
        let err = parse_shelley_genesis(&bytes, 0).unwrap_err();
        assert_eq!(
            err,
            GenesisParseError::MalformedFieldType {
                name: "networkMagic"
            }
        );
    }

    #[test]
    fn extra_inert_keys_produce_byte_identical_anchor() {
        let canonical = parse_shelley_genesis(
            &fixture_bytes("accepted-shelley-genesis.json"),
            0,
        )
        .unwrap();
        let with_extras = parse_shelley_genesis(
            &fixture_bytes("extra-inert-key.shelley-genesis.json"),
            0,
        )
        .unwrap();
        assert_eq!(canonical, with_extras, "extras must be inert");
    }

    #[test]
    fn malformed_numeric_negative_slot_length_rejected() {
        let bytes = fixture_bytes("malformed-numeric.shelley-genesis.json");
        let err = parse_shelley_genesis(&bytes, 0).unwrap_err();
        assert_eq!(
            err,
            GenesisParseError::MalformedFieldValue {
                name: "slotLength"
            }
        );
    }

    #[test]
    fn iso8601_parse_anchors_to_known_unix_ms_values() {
        // Round-trip a few known points.
        assert_eq!(parse_iso8601_to_unix_ms("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(
            parse_iso8601_to_unix_ms("2022-06-01T00:00:00Z"),
            Some(1_654_041_600_000)
        );
        // Reject malformed.
        assert_eq!(parse_iso8601_to_unix_ms("2022-06-01T00:00:00"), None);
        assert_eq!(parse_iso8601_to_unix_ms("not-a-date"), None);
    }

    #[test]
    fn parser_is_byte_identical_across_two_runs() {
        let bytes = fixture_bytes("accepted-shelley-genesis.json");
        let a = parse_shelley_genesis(&bytes, 0).unwrap();
        let b = parse_shelley_genesis(&bytes, 0).unwrap();
        assert_eq!(a, b);
    }
}
