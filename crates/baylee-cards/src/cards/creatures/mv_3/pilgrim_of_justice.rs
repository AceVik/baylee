//! Pilgrim of Justice — {2}{W} — Creature — Human Cleric
//! Oracle: Protection from red
//! Oracle: {W}, Sacrifice this creature: The next time a red source of your choice would deal damage this turn, prevent that damage.
//! Set: ODY #40 — Odyssey | Scryfall ID: 6103a3a7-78ce-4a66-bd82-434e0bd9dea4 | Oracle ID: 959b672e-d4e4-49a1-874a-5148f040fadd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PILGRIM_OF_JUSTICE,
    oracle_id = "959b672e-d4e4-49a1-874a-5148f040fadd",
    scryfall_id = "6103a3a7-78ce-4a66-bd82-434e0bd9dea4",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Pilgrim of Justice",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
