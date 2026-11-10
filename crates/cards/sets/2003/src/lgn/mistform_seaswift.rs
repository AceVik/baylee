//! Mistform Seaswift — {3}{U} — Creature — Illusion
//! Oracle: Flying
//! Oracle: {1}: This creature becomes the creature type of your choice until end of turn.
//! Oracle: Morph {1}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: LGN #45 — Legions | Scryfall ID: b2f6c73c-8162-499f-8d16-92f17c0c2bee | Oracle ID: 268f423e-b368-4561-ae24-db0f7ca775f4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MISTFORM_SEASWIFT,
    oracle_id = "268f423e-b368-4561-ae24-db0f7ca775f4",
    scryfall_id = "b2f6c73c-8162-499f-8d16-92f17c0c2bee",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mistform Seaswift",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(3),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
