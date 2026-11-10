//! Rootwater Mystic — {U} — Creature — Merfolk Wizard
//! Oracle: {1}{U}: Look at the top card of target player's library.
//! Set: EXO #44 — Exodus | Scryfall ID: 94af81f2-383c-4129-b8dc-60633c3f4ea1 | Oracle ID: 32d8f7c8-8680-4901-b31b-e0cb66e37548
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ROOTWATER_MYSTIC,
    oracle_id = "32d8f7c8-8680-4901-b31b-e0cb66e37548",
    scryfall_id = "94af81f2-383c-4129-b8dc-60633c3f4ea1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rootwater Mystic",
        mana_cost = mana!("{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{1}{U}"),
        &[Effect::LookAtTopOf {
            player: PlayerRel::Chosen,
            count: 1,
        }],
        target = Some(TargetSpec::AnyPlayer)
    ),],
);
