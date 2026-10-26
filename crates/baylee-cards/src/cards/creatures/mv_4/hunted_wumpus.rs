//! Hunted Wumpus — {3}{G} — Creature — Beast
//! Oracle: When this creature enters, each other player may put a creature card from their hand onto the battlefield.
//! Set: BBD #202 — Battlebond | Scryfall ID: 2bf7258f-83c0-4cb0-8b1c-1f0c131abcd3 | Oracle ID: 698c9882-4117-4919-8a19-b1cbffc16692
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HUNTED_WUMPUS,
    oracle_id = "698c9882-4117-4919-8a19-b1cbffc16692",
    scryfall_id = "2bf7258f-83c0-4cb0-8b1c-1f0c131abcd3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Hunted Wumpus",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
