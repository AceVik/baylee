//! Sawtooth Loon — {2}{W}{U} — Creature — Bird
//! Oracle: Flying
//! Oracle: When this creature enters, return a white or blue creature you control to its owner's hand.
//! Oracle: When this creature enters, draw two cards, then put two cards from your hand on the bottom of your library.
//! Set: DMR #199 — Dominaria Remastered | Scryfall ID: 3f5882a5-ea4c-4b2b-bcf9-7d15087d8bd4 | Oracle ID: b917e472-2550-4381-b756-45487c2c187a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SAWTOOTH_LOON,
    oracle_id = "b917e472-2550-4381-b756-45487c2c187a",
    scryfall_id = "3f5882a5-ea4c-4b2b-bcf9-7d15087d8bd4",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(
        name = "Sawtooth Loon",
        mana_cost = mana!("{2}{W}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
