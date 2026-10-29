//! Nantuko Shrine — {1}{G}{G} — Enchantment
//! Oracle: Whenever a player casts a spell, that player creates X 1/1 green Squirrel creature tokens, where X is the number of cards in all graveyards with the same name as that spell.
//! Set: ODY #256 — Odyssey | Scryfall ID: 4b66bab3-ee6f-4f74-bbc1-8099c9c4904b | Oracle ID: fc32ec2b-5432-41cc-ab4c-d9f986126392
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NANTUKO_SHRINE,
    oracle_id = "fc32ec2b-5432-41cc-ab4c-d9f986126392",
    scryfall_id = "4b66bab3-ee6f-4f74-bbc1-8099c9c4904b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Nantuko Shrine",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
