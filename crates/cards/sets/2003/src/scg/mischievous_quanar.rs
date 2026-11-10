//! Mischievous Quanar — {4}{U} — Creature — Beast
//! Oracle: {3}{U}{U}: Turn this creature face down.
//! Oracle: Morph {1}{U}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, copy target instant or sorcery spell. You may choose new targets for that copy.
//! Set: SCG #42 — Scourge | Scryfall ID: dc48c2db-f5b4-4c24-a5fa-00750b7ff56f | Oracle ID: 30e3e48c-98dc-4d27-b7ea-e81d39f7d24e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MISCHIEVOUS_QUANAR,
    oracle_id = "30e3e48c-98dc-4d27-b7ea-e81d39f7d24e",
    scryfall_id = "dc48c2db-f5b4-4c24-a5fa-00750b7ff56f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mischievous Quanar",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
