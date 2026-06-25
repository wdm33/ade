// Imperative-Shell genesis parser (S-B1 RED).
//
// Reads the four genesis JSON blobs (byron, shelley, alonzo, conway),
// computes a BootstrapAnchorHash that pins the BLUE EraSchedule to a
// specific genesis configuration, and materializes the typed schedule
// once at startup. The schedule is then consumed BLUE by-value; this
// module is never reached by BLUE.

use ade_codec::cbor::{
    canonical_width, write_array_header, write_bytes_canonical, ContainerEncoding,
};
use ade_core::consensus::{
    BootstrapAnchorHash, EraSchedule, EraSummary, HFCError,
};
use ade_crypto::blake2b::blake2b_256;
use ade_types::{CardanoEra, EpochNo, SlotNo};

/// Network magic tagging which boundary-slot table the parser must
/// consult. Mainnet/preprod/preview are the three operator-facing
/// public networks; private testnets may add variants later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetworkMagic(pub u32);

impl NetworkMagic {
    pub const MAINNET: NetworkMagic = NetworkMagic(764824073);
    pub const PREPROD: NetworkMagic = NetworkMagic(1);
    pub const PREVIEW: NetworkMagic = NetworkMagic(2);
}

/// Genesis blob identifier — used to attribute parse errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenesisBlob {
    Byron,
    Shelley,
    Alonzo,
    Conway,
}

/// Owned slices of the four genesis blobs.
pub struct GenesisBundle<'a> {
    pub byron_json: &'a [u8],
    pub shelley_json: &'a [u8],
    pub alonzo_json: &'a [u8],
    pub conway_json: &'a [u8],
}

/// Structured (no-`String`) parse error taxonomy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenesisParseError {
    MalformedJson { which: GenesisBlob },
    MissingField { which: GenesisBlob, field: &'static str },
    InvalidValue { which: GenesisBlob, field: &'static str },
    UnknownNetwork { magic: u32 },
    Hfc(HFCError),
}

/// Domain-separation tag for the bootstrap anchor.
const ANCHOR_DOMAIN_TAG: &[u8] = b"ade_bootstrap_v1";

/// Compute the BootstrapAnchorHash for a genesis bundle.
///
/// Layout (input to Blake2b-256):
///   ANCHOR_DOMAIN_TAG || canonical_cbor([byron_bytes, shelley_bytes, alonzo_bytes, conway_bytes])
///
/// Each element is encoded as a canonical CBOR byte string with the
/// minimal length width; the wrapping array uses a definite-length
/// header with canonical width. Same inputs → same hash, deterministic.
pub fn compute_anchor_hash(bundle: &GenesisBundle<'_>) -> BootstrapAnchorHash {
    let mut preimage: Vec<u8> = Vec::new();
    preimage.extend_from_slice(ANCHOR_DOMAIN_TAG);
    let mut arr: Vec<u8> = Vec::new();
    write_array_header(&mut arr, ContainerEncoding::Definite(4, canonical_width(4)));
    write_bytes_canonical(&mut arr, bundle.byron_json);
    write_bytes_canonical(&mut arr, bundle.shelley_json);
    write_bytes_canonical(&mut arr, bundle.alonzo_json);
    write_bytes_canonical(&mut arr, bundle.conway_json);
    preimage.extend_from_slice(&arr);
    BootstrapAnchorHash(blake2b_256(&preimage))
}

/// Parse the four genesis blobs into a typed EraSchedule for `network`.
///
/// The function:
///   1. Extracts `byron.protocolConsts.k`, `byron.startTime`,
///      `byron.blockVersionData.slotDuration`.
///   2. Extracts `shelley.epochLength`, `shelley.slotLength`,
///      `shelley.activeSlotsCoeff` (as integer numer/denom pair),
///      `shelley.securityParam`.
///   3. Reads boundary slots for `network` from the bundle's caller
///      (boundary slots are network-specific operator data; this
///      parser accepts them via the JSON itself in the `_boundaries`
///      key on the shelley blob — keeping mainnet slots out of BLUE
///      source).
///   4. Computes `safe_zone_slots = ceil(3 * k * denom / numer)`.
///   5. Builds the EraSchedule.
///
/// `alonzo_json` and `conway_json` are accepted and contribute to the
/// anchor hash; their parameters (cost models, plutus version maps)
/// are not relevant for the era-schedule shape and are not read here.
pub fn parse_genesis(
    bundle: &GenesisBundle<'_>,
    network: NetworkMagic,
) -> Result<EraSchedule, GenesisParseError> {
    let anchor = compute_anchor_hash(bundle);

    // Validate the alonzo + conway blobs at least parse as JSON. We
    // do not consume their fields here, but a malformed blob in either
    // slot must reject the schedule (it would change the anchor and a
    // downstream consumer could not trust the resulting state).
    let _byron = parse_object(bundle.byron_json, GenesisBlob::Byron)?;
    let shelley = parse_object(bundle.shelley_json, GenesisBlob::Shelley)?;
    let _alonzo = parse_object(bundle.alonzo_json, GenesisBlob::Alonzo)?;
    let _conway = parse_object(bundle.conway_json, GenesisBlob::Conway)?;

    // Byron fields.
    let byron_root = parse_object(bundle.byron_json, GenesisBlob::Byron)?;
    let proto = get_obj(&byron_root, GenesisBlob::Byron, "protocolConsts")?;
    let k = get_u64(&proto, GenesisBlob::Byron, "k")?;
    let start_time =
        get_u64(&byron_root, GenesisBlob::Byron, "startTime")?;
    let block_version = get_obj(&byron_root, GenesisBlob::Byron, "blockVersionData")?;
    let byron_slot_duration_str = get_str(
        &block_version,
        GenesisBlob::Byron,
        "slotDuration",
    )?;
    let byron_slot_duration_ms = parse_u64_str(
        &byron_slot_duration_str,
        GenesisBlob::Byron,
        "slotDuration",
    )?;
    let byron_epoch_length_slots = u32::try_from(10 * k)
        .map_err(|_| GenesisParseError::InvalidValue {
            which: GenesisBlob::Byron,
            field: "protocolConsts.k",
        })?;

    // Shelley fields.
    let shelley_epoch_length =
        get_u64(&shelley, GenesisBlob::Shelley, "epochLength")?;
    let shelley_slot_length_seconds =
        get_u64(&shelley, GenesisBlob::Shelley, "slotLength")?;
    let active = get_obj(&shelley, GenesisBlob::Shelley, "activeSlotsCoeff")?;
    let active_numer = get_u64(&active, GenesisBlob::Shelley, "numerator")?;
    let active_denom = get_u64(&active, GenesisBlob::Shelley, "denominator")?;
    if active_numer == 0 {
        return Err(GenesisParseError::InvalidValue {
            which: GenesisBlob::Shelley,
            field: "activeSlotsCoeff.numerator",
        });
    }
    let shelley_security_param =
        get_u64(&shelley, GenesisBlob::Shelley, "securityParam")?;

    let safe_zone_slots = ceil_div(
        shelley_security_param
            .checked_mul(3)
            .ok_or(GenesisParseError::InvalidValue {
                which: GenesisBlob::Shelley,
                field: "securityParam",
            })?
            .checked_mul(active_denom)
            .ok_or(GenesisParseError::InvalidValue {
                which: GenesisBlob::Shelley,
                field: "activeSlotsCoeff.denominator",
            })?,
        active_numer,
    );
    let safe_zone_slots = u32::try_from(safe_zone_slots).map_err(|_| {
        GenesisParseError::InvalidValue {
            which: GenesisBlob::Shelley,
            field: "securityParam",
        }
    })?;

    // RSW = ceil(4 * k / f) -- the Praos candidate-nonce freeze latitude
    // (DC-EPOCH-16), mirroring safe_zone_slots = ceil(3 * k / f).
    let rsw_slots = ceil_div(
        shelley_security_param
            .checked_mul(4)
            .ok_or(GenesisParseError::InvalidValue {
                which: GenesisBlob::Shelley,
                field: "securityParam",
            })?
            .checked_mul(active_denom)
            .ok_or(GenesisParseError::InvalidValue {
                which: GenesisBlob::Shelley,
                field: "activeSlotsCoeff.denominator",
            })?,
        active_numer,
    );
    let rsw_slots = u32::try_from(rsw_slots).map_err(|_| GenesisParseError::InvalidValue {
        which: GenesisBlob::Shelley,
        field: "securityParam",
    })?;

    let shelley_slot_length_ms = shelley_slot_length_seconds
        .checked_mul(1000)
        .ok_or(GenesisParseError::InvalidValue {
            which: GenesisBlob::Shelley,
            field: "slotLength",
        })?;
    let shelley_slot_length_ms = u32::try_from(shelley_slot_length_ms).map_err(|_| {
        GenesisParseError::InvalidValue {
            which: GenesisBlob::Shelley,
            field: "slotLength",
        }
    })?;
    let shelley_epoch_length_slots =
        u32::try_from(shelley_epoch_length).map_err(|_| {
            GenesisParseError::InvalidValue {
                which: GenesisBlob::Shelley,
                field: "epochLength",
            }
        })?;
    let byron_slot_length_ms =
        u32::try_from(byron_slot_duration_ms).map_err(|_| {
            GenesisParseError::InvalidValue {
                which: GenesisBlob::Byron,
                field: "blockVersionData.slotDuration",
            }
        })?;

    // Network-tagged boundary table lives under shelley._ade_boundaries.
    // Keeping operator-known mainnet/preprod boundary slots in JSON (and
    // not baked into BLUE source) is the slice §14 requirement.
    let boundaries = get_obj_optional(&shelley, "_ade_boundaries");
    let table = match boundaries {
        Some(obj) => obj,
        None => {
            return Err(GenesisParseError::MissingField {
                which: GenesisBlob::Shelley,
                field: "_ade_boundaries",
            });
        }
    };
    let key = match network {
        NetworkMagic::MAINNET => "mainnet",
        NetworkMagic::PREPROD => "preprod",
        NetworkMagic::PREVIEW => "preview",
        other => return Err(GenesisParseError::UnknownNetwork { magic: other.0 }),
    };
    let bn = get_obj(&table, GenesisBlob::Shelley, key)?;

    let mut eras: Vec<EraSummary> = Vec::with_capacity(7);
    let byron_start_epoch =
        get_u64(&bn, GenesisBlob::Shelley, "byron_start_epoch")?;
    eras.push(EraSummary {
        era: CardanoEra::ByronRegular,
        start_slot: SlotNo(0),
        start_epoch: EpochNo(byron_start_epoch),
        slot_length_ms: byron_slot_length_ms,
        epoch_length_slots: byron_epoch_length_slots,
        safe_zone_slots,
        randomness_stabilisation_window_slots: Some(rsw_slots),
    });
    let later_eras = [
        ("shelley", CardanoEra::Shelley),
        ("allegra", CardanoEra::Allegra),
        ("mary", CardanoEra::Mary),
        ("alonzo", CardanoEra::Alonzo),
        ("babbage", CardanoEra::Babbage),
        ("conway", CardanoEra::Conway),
    ];
    for (name, era) in later_eras {
        let entry = get_obj(&bn, GenesisBlob::Shelley, name)?;
        let s = get_u64(&entry, GenesisBlob::Shelley, "start_slot")?;
        let e = get_u64(&entry, GenesisBlob::Shelley, "start_epoch")?;
        eras.push(EraSummary {
            era,
            start_slot: SlotNo(s),
            start_epoch: EpochNo(e),
            slot_length_ms: shelley_slot_length_ms,
            epoch_length_slots: shelley_epoch_length_slots,
            safe_zone_slots,
            randomness_stabilisation_window_slots: Some(rsw_slots),
        });
    }

    // Byron start time is unix seconds. Multiply to milliseconds.
    let system_start_unix_ms = start_time
        .checked_mul(1000)
        .ok_or(GenesisParseError::InvalidValue {
            which: GenesisBlob::Byron,
            field: "startTime",
        })?;

    EraSchedule::new(anchor, system_start_unix_ms, eras)
        .map_err(GenesisParseError::Hfc)
}

fn ceil_div(num: u64, denom: u64) -> u64 {
    if denom == 0 {
        return 0;
    }
    let q = num / denom;
    let r = num % denom;
    if r == 0 {
        q
    } else {
        q + 1
    }
}

// ---------------------------------------------------------------------------
// Minimal JSON walker — we use serde_json::Value, but expose typed
// accessors that return structured errors (no `String`, no `Box<dyn Error>`).
// ---------------------------------------------------------------------------

type JsonValue = serde_json::Value;
type JsonObject = serde_json::Map<String, JsonValue>;

fn parse_object(
    bytes: &[u8],
    which: GenesisBlob,
) -> Result<JsonObject, GenesisParseError> {
    let v: JsonValue = serde_json::from_slice(bytes)
        .map_err(|_| GenesisParseError::MalformedJson { which })?;
    match v {
        JsonValue::Object(map) => Ok(map),
        _ => Err(GenesisParseError::MalformedJson { which }),
    }
}

fn get_obj(
    obj: &JsonObject,
    which: GenesisBlob,
    key: &'static str,
) -> Result<JsonObject, GenesisParseError> {
    match obj.get(key) {
        Some(JsonValue::Object(inner)) => Ok(inner.clone()),
        Some(_) => Err(GenesisParseError::InvalidValue { which, field: key }),
        None => Err(GenesisParseError::MissingField { which, field: key }),
    }
}

fn get_obj_optional(obj: &JsonObject, key: &str) -> Option<JsonObject> {
    match obj.get(key) {
        Some(JsonValue::Object(inner)) => Some(inner.clone()),
        _ => None,
    }
}

fn get_u64(
    obj: &JsonObject,
    which: GenesisBlob,
    key: &'static str,
) -> Result<u64, GenesisParseError> {
    match obj.get(key) {
        Some(JsonValue::Number(n)) => n
            .as_u64()
            .ok_or(GenesisParseError::InvalidValue { which, field: key }),
        Some(_) => Err(GenesisParseError::InvalidValue { which, field: key }),
        None => Err(GenesisParseError::MissingField { which, field: key }),
    }
}

fn get_str(
    obj: &JsonObject,
    which: GenesisBlob,
    key: &'static str,
) -> Result<String, GenesisParseError> {
    match obj.get(key) {
        Some(JsonValue::String(s)) => Ok(s.clone()),
        Some(_) => Err(GenesisParseError::InvalidValue { which, field: key }),
        None => Err(GenesisParseError::MissingField { which, field: key }),
    }
}

fn parse_u64_str(
    s: &str,
    which: GenesisBlob,
    key: &'static str,
) -> Result<u64, GenesisParseError> {
    s.parse::<u64>()
        .map_err(|_| GenesisParseError::InvalidValue { which, field: key })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BYRON_FIXTURE: &str = r#"{
        "protocolConsts": { "k": 2160 },
        "startTime": 1506203091,
        "blockVersionData": { "slotDuration": "20000" }
    }"#;

    const SHELLEY_FIXTURE: &str = r#"{
        "epochLength": 432000,
        "slotLength": 1,
        "activeSlotsCoeff": { "numerator": 1, "denominator": 20 },
        "securityParam": 2160,
        "_ade_boundaries": {
            "mainnet": {
                "byron_start_epoch": 0,
                "shelley":  { "start_slot": 4492800,   "start_epoch": 208 },
                "allegra":  { "start_slot": 16588800,  "start_epoch": 236 },
                "mary":     { "start_slot": 23068800,  "start_epoch": 251 },
                "alonzo":   { "start_slot": 39916800,  "start_epoch": 290 },
                "babbage":  { "start_slot": 72316796,  "start_epoch": 365 },
                "conway":   { "start_slot": 133660800, "start_epoch": 507 }
            },
            "preprod": {
                "byron_start_epoch": 0,
                "shelley":  { "start_slot": 86400,    "start_epoch": 4 },
                "allegra":  { "start_slot": 518400,   "start_epoch": 5 },
                "mary":     { "start_slot": 950400,   "start_epoch": 6 },
                "alonzo":   { "start_slot": 1382400,  "start_epoch": 7 },
                "babbage":  { "start_slot": 1814400,  "start_epoch": 8 },
                "conway":   { "start_slot": 55814400, "start_epoch": 132 }
            }
        }
    }"#;

    const ALONZO_FIXTURE: &str = r#"{ "lovelacePerUTxOWord": 34482 }"#;
    const CONWAY_FIXTURE: &str = r#"{ "poolVotingThresholds": {} }"#;

    fn bundle<'a>() -> GenesisBundle<'a> {
        GenesisBundle {
            byron_json: BYRON_FIXTURE.as_bytes(),
            shelley_json: SHELLEY_FIXTURE.as_bytes(),
            alonzo_json: ALONZO_FIXTURE.as_bytes(),
            conway_json: CONWAY_FIXTURE.as_bytes(),
        }
    }

    #[test]
    fn anchor_hash_deterministic() {
        let h1 = compute_anchor_hash(&bundle());
        let h2 = compute_anchor_hash(&bundle());
        assert_eq!(h1, h2, "same inputs must produce same anchor");
    }

    #[test]
    fn anchor_hash_distinguishes_inputs() {
        let h1 = compute_anchor_hash(&bundle());
        let mutated_byron = BYRON_FIXTURE.replace("1506203091", "1506203092");
        let other = GenesisBundle {
            byron_json: mutated_byron.as_bytes(),
            shelley_json: SHELLEY_FIXTURE.as_bytes(),
            alonzo_json: ALONZO_FIXTURE.as_bytes(),
            conway_json: CONWAY_FIXTURE.as_bytes(),
        };
        let h2 = compute_anchor_hash(&other);
        assert_ne!(h1, h2, "different inputs must produce different anchors");
    }

    #[test]
    fn parse_mainnet_produces_seven_eras() {
        let schedule = parse_genesis(&bundle(), NetworkMagic::MAINNET)
            .expect("parser succeeds on well-formed synthetic blobs");
        assert_eq!(schedule.eras().len(), 7);
        assert_eq!(schedule.eras()[0].era, CardanoEra::ByronRegular);
        assert_eq!(schedule.eras()[6].era, CardanoEra::Conway);
        assert_eq!(schedule.system_start_unix_ms(), 1_506_203_091_000);
        assert_eq!(schedule.eras()[0].safe_zone_slots, 129_600);
    }

    #[test]
    fn parse_preprod_produces_different_boundaries() {
        let mainnet = parse_genesis(&bundle(), NetworkMagic::MAINNET)
            .expect("mainnet parses");
        let preprod = parse_genesis(&bundle(), NetworkMagic::PREPROD)
            .expect("preprod parses");
        // Anchor is the same — both schedules are derived from the same
        // bundle bytes. Boundary slots must differ.
        assert_eq!(mainnet.anchor(), preprod.anchor());
        assert_ne!(mainnet.eras()[1].start_slot, preprod.eras()[1].start_slot);
    }

    #[test]
    fn parse_rejects_malformed_json() {
        let b = GenesisBundle {
            byron_json: b"not json",
            shelley_json: SHELLEY_FIXTURE.as_bytes(),
            alonzo_json: ALONZO_FIXTURE.as_bytes(),
            conway_json: CONWAY_FIXTURE.as_bytes(),
        };
        let result = parse_genesis(&b, NetworkMagic::MAINNET);
        assert_eq!(
            result,
            Err(GenesisParseError::MalformedJson {
                which: GenesisBlob::Byron
            })
        );
    }

    #[test]
    fn parse_rejects_missing_field() {
        let byron_missing = r#"{
            "protocolConsts": {},
            "startTime": 1506203091,
            "blockVersionData": { "slotDuration": "20000" }
        }"#;
        let b = GenesisBundle {
            byron_json: byron_missing.as_bytes(),
            shelley_json: SHELLEY_FIXTURE.as_bytes(),
            alonzo_json: ALONZO_FIXTURE.as_bytes(),
            conway_json: CONWAY_FIXTURE.as_bytes(),
        };
        let result = parse_genesis(&b, NetworkMagic::MAINNET);
        assert_eq!(
            result,
            Err(GenesisParseError::MissingField {
                which: GenesisBlob::Byron,
                field: "k"
            })
        );
    }

    #[test]
    fn parse_rejects_unknown_network() {
        let result = parse_genesis(&bundle(), NetworkMagic(999_999));
        assert_eq!(
            result,
            Err(GenesisParseError::UnknownNetwork { magic: 999_999 })
        );
    }

    #[test]
    fn safe_zone_uses_ceil_div() {
        // 3 * 2160 * 20 / 1 = 129600 exactly — no rounding needed.
        let schedule = parse_genesis(&bundle(), NetworkMagic::MAINNET)
            .expect("parser succeeds");
        assert_eq!(schedule.eras()[1].safe_zone_slots, 129_600);
    }
}
