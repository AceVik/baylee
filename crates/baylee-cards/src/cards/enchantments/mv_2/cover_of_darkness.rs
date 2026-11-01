//! Cover of Darkness — {1}{B} — Enchantment
//! Oracle: As this enchantment enters, choose a creature type.
//! Oracle: Creatures of the chosen type have fear. (They can't be blocked except by artifact creatures and/or black creatures.)
//! Set: ONS #133 — Onslaught | Scryfall ID: 0d6d7d88-d82b-40f4-bf57-ec5d7c480689 | Oracle ID: d78654fc-7d90-40ce-9c36-a211424d1a88
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COVER_OF_DARKNESS,
    oracle_id = "d78654fc-7d90-40ce-9c36-a211424d1a88",
    scryfall_id = "0d6d7d88-d82b-40f4-bf57-ec5d7c480689",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Cover of Darkness",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
