//! Quicksilver Dragon — {4}{U}{U} — Creature — Dragon
//! Oracle: Flying
//! Oracle: {U}: If target spell has only one target and that target is this creature, change that spell's target to another creature.
//! Oracle: Morph {4}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: JVC #19 — Duel Decks Anthology: Jace vs. Chandra | Scryfall ID: a8943e2c-8a85-4ca4-a7b1-87b89e697877 | Oracle ID: f9701c1f-66c7-4ac7-8106-074d6f557ad6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::QUICKSILVER_DRAGON,
    oracle_id = "f9701c1f-66c7-4ac7-8106-074d6f557ad6",
    scryfall_id = "a8943e2c-8a85-4ca4-a7b1-87b89e697877",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Quicksilver Dragon",
        mana_cost = mana!("{4}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
