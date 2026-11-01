//! Gangrenous Goliath — {3}{B}{B} — Creature — Zombie Giant
//! Oracle: Tap three untapped Clerics you control: Return this card from your graveyard to your hand.
//! Set: ONS #150 — Onslaught | Scryfall ID: 69b58b6b-24cd-4440-b99c-d88d44b3c41c | Oracle ID: 103c6037-18ff-44d8-84a1-fd6bdb3040c5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GANGRENOUS_GOLIATH,
    oracle_id = "103c6037-18ff-44d8-84a1-fd6bdb3040c5",
    scryfall_id = "69b58b6b-24cd-4440-b99c-d88d44b3c41c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Gangrenous Goliath",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::GIANT],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
