//! Beyeen Veil // Beyeen Coast — {1}{U} — Instant // Land
//! Set: ZNR #46 — Zendikar Rising | Scryfall ID: 5f411f08-45dd-4d73-8894-daf51c175150 | Oracle ID: b03de49d-246f-44e2-9487-9e4e43ec7be4
//! Face: Beyeen Veil — {1}{U} — Instant
//! Face: Beyeen Coast —  — Land
// IMPLEMENTED — Beyeen Veil debuffs all opponent creatures -2/-0 until EOT;
// Beyeen Coast enters tapped and taps for {U}.

use baylee_cards_dsl::prelude::*;

static COAST_ABILITIES: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
];

card! {
    index: 269,
    oracle_id: "b03de49d-246f-44e2-9487-9e4e43ec7be4",
    scryfall_id: "5f411f08-45dd-4d73-8894-daf51c175150",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    faces: &[
    face! {
        name: "Beyeen Veil",
        mana_cost: baylee_core::mana!("{1}{U}"),
        types: TypeSet::INSTANT,
    },
    face! {
        name: "Beyeen Coast",
        types: TypeSet::LAND,
        castable_from_hand: false,
        enter_modifiers: &[EnterModifier::Tapped],
        abilities: COAST_ABILITIES,
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        // "Creatures your opponents control get -2/-0 until end of turn."
        spell!(&[Effect::PumpFilter {
            filter: &Filter::OPPONENT_CREATURE,
            power: Amount::NegXFixed(2),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }]),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_matches_registry() {
        assert_eq!(CARD.index.get(), 269);
    }

    #[test]
    fn oracle_and_scryfall_ids() {
        assert_eq!(CARD.oracle_id, "b03de49d-246f-44e2-9487-9e4e43ec7be4");
        assert_eq!(CARD.scryfall_id, "5f411f08-45dd-4d73-8894-daf51c175150");
    }

    #[test]
    fn front_face_is_instant() {
        let face = &CARD.faces[0];
        assert_eq!(face.name, "Beyeen Veil");
        assert!(face.types.contains(TypeSet::INSTANT));
    }

    #[test]
    fn back_face_is_land_not_castable_from_hand() {
        let face = &CARD.faces[1];
        assert_eq!(face.name, "Beyeen Coast");
        assert!(face.types.contains(TypeSet::LAND));
        assert!(!face.castable_from_hand);
    }

    #[test]
    fn color_identity_is_blue() {
        assert_eq!(CARD.color_identity, ColorSet::from_slice(&[Color::Blue]));
    }

    #[test]
    fn coverage_is_implemented() {
        assert!(matches!(CARD.coverage, Coverage::Implemented));
    }

    // Engine-level tests belong in baylee-engine:
    // beyeen_veil_debuffs_opponent_creatures — spell gives all opponent
    //   creatures -2/-0 until end of turn; friendly creatures unaffected.
    // beyeen_coast_enters_tapped — land enters the battlefield tapped.
    // beyeen_coast_taps_for_blue — {T}: Add {U}.
}
