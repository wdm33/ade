use ade_types::CardanoEra;

use super::Era;

/// Map from BLUE `CardanoEra` (8 variants, codec-level) to GREEN `Era`
/// (7 variants, harness-level).
///
/// `CardanoEra` distinguishes Byron EBB (tag 0) from Byron regular (tag 1),
/// while `Era` collapses both into `Era::Byron`.
pub fn cardano_era_to_harness_era(era: CardanoEra) -> Era {
    match era {
        CardanoEra::ByronEbb | CardanoEra::ByronRegular => Era::Byron,
        CardanoEra::Shelley => Era::Shelley,
        CardanoEra::Allegra => Era::Allegra,
        CardanoEra::Mary => Era::Mary,
        CardanoEra::Alonzo => Era::Alonzo,
        CardanoEra::Babbage => Era::Babbage,
        CardanoEra::Conway => Era::Conway,
    }
}

/// Map from GREEN `Era` to the corresponding HFC era tag range.
///
/// Returns the set of `CardanoEra` variants that belong to this harness era.
/// Byron has two (EBB and regular); all others have exactly one.
pub fn harness_era_to_cardano_eras(era: Era) -> &'static [CardanoEra] {
    match era {
        Era::Byron => &[CardanoEra::ByronEbb, CardanoEra::ByronRegular],
        Era::Shelley => &[CardanoEra::Shelley],
        Era::Allegra => &[CardanoEra::Allegra],
        Era::Mary => &[CardanoEra::Mary],
        Era::Alonzo => &[CardanoEra::Alonzo],
        Era::Babbage => &[CardanoEra::Babbage],
        Era::Conway => &[CardanoEra::Conway],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byron_ebb_maps_to_byron() {
        assert_eq!(cardano_era_to_harness_era(CardanoEra::ByronEbb), Era::Byron);
    }

    #[test]
    fn byron_regular_maps_to_byron() {
        assert_eq!(
            cardano_era_to_harness_era(CardanoEra::ByronRegular),
            Era::Byron
        );
    }

    #[test]
    fn all_post_byron_eras_map_one_to_one() {
        assert_eq!(
            cardano_era_to_harness_era(CardanoEra::Shelley),
            Era::Shelley
        );
        assert_eq!(
            cardano_era_to_harness_era(CardanoEra::Allegra),
            Era::Allegra
        );
        assert_eq!(cardano_era_to_harness_era(CardanoEra::Mary), Era::Mary);
        assert_eq!(cardano_era_to_harness_era(CardanoEra::Alonzo), Era::Alonzo);
        assert_eq!(
            cardano_era_to_harness_era(CardanoEra::Babbage),
            Era::Babbage
        );
        assert_eq!(cardano_era_to_harness_era(CardanoEra::Conway), Era::Conway);
    }

    #[test]
    fn byron_has_two_cardano_eras() {
        let eras = harness_era_to_cardano_eras(Era::Byron);
        assert_eq!(eras.len(), 2);
        assert_eq!(eras[0], CardanoEra::ByronEbb);
        assert_eq!(eras[1], CardanoEra::ByronRegular);
    }

    #[test]
    fn non_byron_eras_have_one_cardano_era() {
        for era in &[
            Era::Shelley,
            Era::Allegra,
            Era::Mary,
            Era::Alonzo,
            Era::Babbage,
            Era::Conway,
        ] {
            assert_eq!(harness_era_to_cardano_eras(*era).len(), 1);
        }
    }

    #[test]
    fn round_trip_all_cardano_eras() {
        for ce in CardanoEra::ALL {
            let he = cardano_era_to_harness_era(ce);
            let back = harness_era_to_cardano_eras(he);
            assert!(
                back.contains(&ce),
                "{ce:?} -> {he:?} -> {back:?} does not contain original"
            );
        }
    }
}
