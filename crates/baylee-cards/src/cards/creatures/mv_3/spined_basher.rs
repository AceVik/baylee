//! Spined Basher — {2}{B} — Creature — Zombie Beast
//! Oracle: Morph {2}{B} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #172 — Onslaught | Scryfall ID: 4d0d666a-8e31-466c-937f-54df910f664e | Oracle ID: 4891923b-9c8c-4816-9623-d9362e4eb503
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPINED_BASHER,
    oracle_id = "4891923b-9c8c-4816-9623-d9362e4eb503",
    scryfall_id = "4d0d666a-8e31-466c-937f-54df910f664e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Spined Basher",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
