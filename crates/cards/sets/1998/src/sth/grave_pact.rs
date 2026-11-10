//! Grave Pact — {1}{B}{B}{B} — Enchantment
//! Oracle: Whenever a creature you control dies, each other player sacrifices a creature of their choice.
//! Set: CMM #165 — Commander Masters | Scryfall ID: f5a4970b-2ba6-4c91-a301-369369cdf360 | Oracle ID: 6f4ac4a4-53ec-4bc9-8f5c-d4b801d867b2
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GRAVE_PACT,
    oracle_id = "6f4ac4a4-53ec-4bc9-8f5c-d4b801d867b2",
    scryfall_id = "f5a4970b-2ba6-4c91-a301-369369cdf360",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Grave Pact",
        mana_cost = mana!("{1}{B}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::YOUR_CREATURE),
        &[Effect::SacrificeFilter {
            who: PlayerRel::EachOtherPlayer,
            filter: &Filter::CREATURE,
        }],
    )],
);
