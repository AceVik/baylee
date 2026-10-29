//! Pilgrim of Virtue — {2}{W} — Creature — Human Cleric
//! Oracle: Protection from black
//! Oracle: {W}, Sacrifice this creature: The next time a black source of your choice would deal damage this turn, prevent that damage.
//! Set: ODY #41 — Odyssey | Scryfall ID: be58f0dd-3856-4960-83de-06eed78ed703 | Oracle ID: e2f73743-95e7-4f03-9b98-5ab5ebc4b724
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PILGRIM_OF_VIRTUE,
    oracle_id = "e2f73743-95e7-4f03-9b98-5ab5ebc4b724",
    scryfall_id = "be58f0dd-3856-4960-83de-06eed78ed703",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Pilgrim of Virtue",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
