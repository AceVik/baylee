//! Grave Defiler — {3}{B} — Creature — Zombie
//! Oracle: When this creature enters, reveal the top four cards of your library. Put all Zombie cards revealed this way into your hand and the rest on the bottom of your library in any order.
//! Oracle: {1}{B}: Regenerate this creature.
//! Set: APC #40 — Apocalypse | Scryfall ID: f8f76edc-6067-43bd-9582-1d59caf91597 | Oracle ID: 42c5c1aa-10a4-4cd5-b1f8-a393ed553f99
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GRAVE_DEFILER,
    oracle_id = "42c5c1aa-10a4-4cd5-b1f8-a393ed553f99",
    scryfall_id = "f8f76edc-6067-43bd-9582-1d59caf91597",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Grave Defiler",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
