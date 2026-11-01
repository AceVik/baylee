//! Mistform Warchief — {2}{U} — Creature — Illusion
//! Oracle: Creature spells you cast that share a creature type with this creature cost {1} less to cast.
//! Oracle: {T}: This creature becomes the creature type of your choice until end of turn.
//! Set: SCG #43 — Scourge | Scryfall ID: a633d85b-4be1-44a2-8fd8-1ccec4a95ecb | Oracle ID: 97f1ac0d-a3a6-46d8-ae99-b17b636fe47c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MISTFORM_WARCHIEF,
    oracle_id = "97f1ac0d-a3a6-46d8-ae99-b17b636fe47c",
    scryfall_id = "a633d85b-4be1-44a2-8fd8-1ccec4a95ecb",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mistform Warchief",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
