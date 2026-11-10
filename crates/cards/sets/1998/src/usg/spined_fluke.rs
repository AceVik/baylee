//! Spined Fluke — {2}{B} — Creature — Worm Horror
//! Oracle: When this creature enters, sacrifice a creature.
//! Oracle: {B}: Regenerate this creature.
//! Set: USG #160 — Urza's Saga | Scryfall ID: 8a14524b-e690-4f77-b43e-416d5ec3cbb9 | Oracle ID: 3c880ac2-0b22-4f7e-a55a-381de235e971
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPINED_FLUKE,
    oracle_id = "3c880ac2-0b22-4f7e-a55a-381de235e971",
    scryfall_id = "8a14524b-e690-4f77-b43e-416d5ec3cbb9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Spined Fluke",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WORM, subtypes::creature::HORROR],
        power = Some(5),
        toughness = Some(1),
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::SacrificeFilter {
                who: PlayerRel::You,
                filter: &Filter::CREATURE,
            }],
        ),
        activated!(cost!("{B}"), &[Effect::regenerate(TargetSpec::ThisObject)]),
    ],
);
