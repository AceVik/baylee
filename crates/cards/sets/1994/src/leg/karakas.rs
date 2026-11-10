//! Karakas — (no cost) — Legendary Land
//! Oracle: {T}: Add {W}.
//! Oracle: {T}: Return target legendary creature to its owner's hand.
//! Set: UMA #244 — Ultimate Masters | Scryfall ID: e52214e1-404a-405a-b08e-20e13c087338 | Oracle ID: 59119143-c0fa-49dd-adf0-e2fd3029c48b
// IMPLEMENTED.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KARAKAS,
    oracle_id = "59119143-c0fa-49dd-adf0-e2fd3029c48b",
    scryfall_id = "e52214e1-404a-405a-b08e-20e13c087338",
    faces = &[face!(
        name = "Karakas",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    )],
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        activated!(
            Cost::TAP,
            &[Effect::bounce(TargetSpec::Object(
                &Filter::LEGENDARY_CREATURE
            ))],
            target = Some(TargetSpec::Object(&Filter::LEGENDARY_CREATURE))
        ),
    ],
);
