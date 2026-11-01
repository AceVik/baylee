//! Mistform Skyreaver — {5}{U}{U} — Creature — Illusion
//! Oracle: Flying
//! Oracle: {1}: This creature becomes the creature type of your choice until end of turn.
//! Set: ONS #97 — Onslaught | Scryfall ID: e394e096-ea70-4813-9039-e4bd065d0a17 | Oracle ID: 6a291cf5-13cc-4e04-a565-f36650fbe5a1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MISTFORM_SKYREAVER,
    oracle_id = "6a291cf5-13cc-4e04-a565-f36650fbe5a1",
    scryfall_id = "e394e096-ea70-4813-9039-e4bd065d0a17",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mistform Skyreaver",
        mana_cost = mana!("{5}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
