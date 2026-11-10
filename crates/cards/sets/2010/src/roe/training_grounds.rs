//! Training Grounds — {U} — Enchantment
//! Oracle: Activated abilities of creatures you control cost {2} less to activate. This effect can't reduce the mana in that cost to less than one mana.
//! Set: MAT #9 — March of the Machine: The Aftermath | Scryfall ID: 991c27ed-f53c-48c6-8c12-282f44b8d441 | Oracle ID: 8de3eabe-9a8f-4865-beef-a4888c081d39
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TRAINING_GROUNDS,
    oracle_id = "8de3eabe-9a8f-4865-beef-a4888c081d39",
    scryfall_id = "991c27ed-f53c-48c6-8c12-282f44b8d441",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Training Grounds",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
