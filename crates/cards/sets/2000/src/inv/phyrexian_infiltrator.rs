//! Phyrexian Infiltrator — {2}{B} — Creature — Phyrexian Minion
//! Oracle: {2}{U}{U}: Exchange control of this creature and target creature. (This effect lasts indefinitely.)
//! Set: INV #116 — Invasion | Scryfall ID: 224b8254-553d-4d88-8163-1f15e1244bd2 | Oracle ID: f444a0f8-32d8-4748-af58-51a1a8ed72b4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHYREXIAN_INFILTRATOR,
    oracle_id = "f444a0f8-32d8-4748-af58-51a1a8ed72b4",
    scryfall_id = "224b8254-553d-4d88-8163-1f15e1244bd2",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Phyrexian Infiltrator",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::MINION],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
