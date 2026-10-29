//! Do or Die — {1}{B} — Sorcery
//! Oracle: Separate all creatures target player controls into two piles. Destroy all creatures in the pile of that player's choice. They can't be regenerated.
//! Set: INV #102 — Invasion | Scryfall ID: 05f63cd9-e82b-4cf8-b8ce-f0aa0157692b | Oracle ID: f25af42d-f1bf-4bd3-aded-ecadafdbf6e6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DO_OR_DIE,
    oracle_id = "f25af42d-f1bf-4bd3-aded-ecadafdbf6e6",
    scryfall_id = "05f63cd9-e82b-4cf8-b8ce-f0aa0157692b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Do or Die",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
