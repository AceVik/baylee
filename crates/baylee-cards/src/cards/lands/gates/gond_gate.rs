//! Gond Gate — (no cost) — Land — Gate
//! Oracle: Gates you control enter untapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color that a Gate you control could produce.
//! Set: CLB #353 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 746672d9-7c6b-415e-9f34-3cc3ac557008 | Oracle ID: 4306938b-c0db-4e63-a4fb-61628e5ff41f
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;
card!(
    index = index::GOND_GATE,
    oracle_id = "4306938b-c0db-4e63-a4fb-61628e5ff41f",
    scryfall_id = "746672d9-7c6b-415e-9f34-3cc3ac557008",
    faces = &[face!(
        name = "Gond Gate",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::GATE]
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[Effect::mana_from_lands(
            &Filter::HasSubtype(subtypes::land::GATE),
            true,
            false
        )]),
        AbilityDef::Replacement(ReplacementRule::EnterUntapped(&Filter::And(&[
            Filter::HasSubtype(subtypes::land::GATE),
            Filter::ControlledByYou,
        ]))),
    ],
);
