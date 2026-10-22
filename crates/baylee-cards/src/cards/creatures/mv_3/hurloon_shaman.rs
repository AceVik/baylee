//! Hurloon Shaman — {1}{R}{R} — Creature — Minotaur Shaman
//! Oracle: When this creature dies, each player sacrifices a land of their choice.
//! Set: WTH #108 — Weatherlight | Scryfall ID: 70a359c9-1889-426d-acaf-074cfd9f274d | Oracle ID: 9fc74fc9-bb02-4d9e-bb8e-f44f219d8946
// IMPLEMENTED — hand-authored in `baylee-cards`.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HURLOON_SHAMAN,
    oracle_id = "9fc74fc9-bb02-4d9e-bb8e-f44f219d8946",
    scryfall_id = "70a359c9-1889-426d-acaf-074cfd9f274d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hurloon Shaman",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MINOTAUR, subtypes::creature::SHAMAN],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::SacrificeFilter {
            who: PlayerRel::EachPlayer,
            filter: &Filter::LAND
        }]
    ),],
);
