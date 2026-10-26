//! Glittering Lion — {2}{W} — Creature — Cat
//! Oracle: Prevent all damage that would be dealt to this creature.
//! Oracle: {3}: Until end of turn, this creature loses "Prevent all damage that would be dealt to this creature." Any player may activate this ability.
//! Set: PCY #10 — Prophecy | Scryfall ID: ab4be296-33a6-46b1-9748-5b0d335f40ee | Oracle ID: 549e6de7-56e9-4f5c-8c88-30e446bc53bb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GLITTERING_LION,
    oracle_id = "549e6de7-56e9-4f5c-8c88-30e446bc53bb",
    scryfall_id = "ab4be296-33a6-46b1-9748-5b0d335f40ee",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Glittering Lion",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CAT],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
