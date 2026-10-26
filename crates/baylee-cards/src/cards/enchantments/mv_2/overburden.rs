//! Overburden — {1}{U} — Enchantment
//! Oracle: Whenever a player puts a nontoken creature onto the battlefield, that player returns a land they control to its owner's hand.
//! Set: PCY #39 — Prophecy | Scryfall ID: 7e6cf6b6-b4c1-4742-9be4-b3b15fbb0202 | Oracle ID: 8104e454-77f5-4d38-894e-41ed0d74c17c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OVERBURDEN,
    oracle_id = "8104e454-77f5-4d38-894e-41ed0d74c17c",
    scryfall_id = "7e6cf6b6-b4c1-4742-9be4-b3b15fbb0202",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Overburden",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
