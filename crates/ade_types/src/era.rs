// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Cardano era discriminant for the HardForkCombinator block envelope.
///
/// Each variant corresponds to a specific era tag in the outer CBOR array.
/// This enum is closed — unknown era tags produce a `CodecError`, never
/// a fallback variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum CardanoEra {
    ByronEbb = 0,
    ByronRegular = 1,
    Shelley = 2,
    Allegra = 3,
    Mary = 4,
    Alonzo = 5,
    Babbage = 6,
    Conway = 7,
}

impl CardanoEra {
    /// All era variants in tag order.
    pub const ALL: [CardanoEra; 8] = [
        CardanoEra::ByronEbb,
        CardanoEra::ByronRegular,
        CardanoEra::Shelley,
        CardanoEra::Allegra,
        CardanoEra::Mary,
        CardanoEra::Alonzo,
        CardanoEra::Babbage,
        CardanoEra::Conway,
    ];

    /// Returns the HFC era tag as a `u8`.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Returns a lowercase string name for this era.
    pub fn as_str(self) -> &'static str {
        match self {
            CardanoEra::ByronEbb => "byron_ebb",
            CardanoEra::ByronRegular => "byron_regular",
            CardanoEra::Shelley => "shelley",
            CardanoEra::Allegra => "allegra",
            CardanoEra::Mary => "mary",
            CardanoEra::Alonzo => "alonzo",
            CardanoEra::Babbage => "babbage",
            CardanoEra::Conway => "conway",
        }
    }

    /// Returns true if this is a Byron-era variant (EBB or regular).
    pub fn is_byron(self) -> bool {
        matches!(self, CardanoEra::ByronEbb | CardanoEra::ByronRegular)
    }
}

/// Error returned when an unknown era tag is encountered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownEraTag(pub u8);

impl core::fmt::Display for UnknownEraTag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "unknown era tag: {}", self.0)
    }
}

impl TryFrom<u8> for CardanoEra {
    type Error = UnknownEraTag;

    fn try_from(tag: u8) -> Result<Self, Self::Error> {
        match tag {
            0 => Ok(CardanoEra::ByronEbb),
            1 => Ok(CardanoEra::ByronRegular),
            2 => Ok(CardanoEra::Shelley),
            3 => Ok(CardanoEra::Allegra),
            4 => Ok(CardanoEra::Mary),
            5 => Ok(CardanoEra::Alonzo),
            6 => Ok(CardanoEra::Babbage),
            7 => Ok(CardanoEra::Conway),
            _ => Err(UnknownEraTag(tag)),
        }
    }
}

impl core::fmt::Display for CardanoEra {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_contains_eight_variants() {
        assert_eq!(CardanoEra::ALL.len(), 8);
    }

    #[test]
    fn ordering_matches_tag_value() {
        for window in CardanoEra::ALL.windows(2) {
            assert!(window[0] < window[1]);
            assert!(window[0].as_u8() < window[1].as_u8());
        }
    }

    #[test]
    fn try_from_round_trips_all_valid_tags() {
        for era in CardanoEra::ALL {
            let tag = era.as_u8();
            let recovered = CardanoEra::try_from(tag);
            assert_eq!(recovered, Ok(era));
        }
    }

    #[test]
    fn try_from_rejects_unknown_tags() {
        for tag in 8..=255u8 {
            assert_eq!(CardanoEra::try_from(tag), Err(UnknownEraTag(tag)));
        }
    }

    #[test]
    fn display_matches_as_str() {
        for era in CardanoEra::ALL {
            assert_eq!(format!("{era}"), era.as_str());
        }
    }

    #[test]
    fn is_byron_correct() {
        assert!(CardanoEra::ByronEbb.is_byron());
        assert!(CardanoEra::ByronRegular.is_byron());
        assert!(!CardanoEra::Shelley.is_byron());
        assert!(!CardanoEra::Conway.is_byron());
    }

    #[test]
    fn repr_u8_matches_as_u8() {
        assert_eq!(CardanoEra::ByronEbb.as_u8(), 0);
        assert_eq!(CardanoEra::Conway.as_u8(), 7);
    }
}
