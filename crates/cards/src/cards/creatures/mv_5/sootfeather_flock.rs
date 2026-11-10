//! Sootfeather Flock — {4}{B} — Creature — Bird
//! Oracle: Flying
//! Oracle: Morph {3}{B} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: LGN #82 — Legions | Scryfall ID: 216a2ccc-8847-452b-b030-27d8506675bd | Oracle ID: 54a4bde3-c61f-44ea-bd2f-cdbf6a76eefa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SOOTFEATHER_FLOCK,
    oracle_id = "54a4bde3-c61f-44ea-bd2f-cdbf6a76eefa",
    scryfall_id = "216a2ccc-8847-452b-b030-27d8506675bd",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Sootfeather Flock",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
