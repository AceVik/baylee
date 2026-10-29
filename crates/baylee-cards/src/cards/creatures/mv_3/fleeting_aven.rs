//! Fleeting Aven — {1}{U}{U} — Creature — Bird Wizard
//! Oracle: Flying
//! Oracle: Whenever a player cycles a card, return this creature to its owner's hand.
//! Set: ONS #83 — Onslaught | Scryfall ID: 246a2758-0096-43b9-8193-d6ae5b41b6e6 | Oracle ID: 7215b1d9-eb8b-41bc-a7e2-1930d9cd5365
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FLEETING_AVEN,
    oracle_id = "7215b1d9-eb8b-41bc-a7e2-1930d9cd5365",
    scryfall_id = "246a2758-0096-43b9-8193-d6ae5b41b6e6",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Fleeting Aven",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
