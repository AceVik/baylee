//! Daily Bugle Building — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color.
//! Oracle: Smear Campaign — {1}, {T}: Target legendary creature gains menace until end of turn. Activate only as a sorcery.
//! Set: SPM #179 — Marvel's Spider-Man | Scryfall ID: 669bbcb1-0981-40e7-905e-b94e74bc4861 | Oracle ID: 483e0c6c-8131-486c-b482-cc3396c9786b
// IMPLEMENTED — tap for {C}; {1},{T} for any color; {1},{T} at sorcery speed
// gives target legendary creature menace until end of turn.

use baylee_cards_dsl::prelude::*;

static TARGET_LEGENDARY_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSupertype(SupertypeSet::LEGENDARY),
]);

card! {
    index: 397,
    oracle_id: "483e0c6c-8131-486c-b482-cc3396c9786b",
    scryfall_id: "669bbcb1-0981-40e7-905e-b94e74bc4861",
    faces: &[
    face! {
        name: "Daily Bugle Building",
        types: TypeSet::LAND,
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        // {T}: Add {C}.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // {1}, {T}: Add one mana of any color.
        mana_ability!(
            Cost {
                mana: baylee_core::mana!("{1}"),
                parts: &[CostPart::TapSelf],
            },
            &[Effect::mana_of_any_color()],
        ),
        // Smear Campaign — {1}, {T}: Target legendary creature gains menace
        // until end of turn. Activate only as a sorcery.
        activated!(
            Cost {
                mana: baylee_core::mana!("{1}"),
                parts: &[CostPart::TapSelf],
            },
            &[Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::MENACE,
                duration: Duration::UntilEndOfTurn,
            }],
            target: Some(TargetSpec::Object(&TARGET_LEGENDARY_CREATURE)),
            timing: ActivationTiming::SorcerySpeed,
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 397);
        assert_eq!(CARD.oracle_id, "483e0c6c-8131-486c-b482-cc3396c9786b");
        assert_eq!(CARD.scryfall_id, "669bbcb1-0981-40e7-905e-b94e74bc4861");
        assert_eq!(CARD.faces[0].name, "Daily Bugle Building");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 3);
    }
}

// Engine-level tests (baylee-engine): daily_bugle_building_abilities —
// tap for {C}; tap and pay {1} for any color of mana; tap and pay {1} at
// sorcery speed to give target legendary creature menace until end of turn.
