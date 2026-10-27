//! Blistering Firecat — {1}{R}{R}{R} — Creature — Elemental Cat
//! Oracle: Trample, haste
//! Oracle: At the beginning of the end step, sacrifice this creature.
//! Oracle: Morph {R}{R} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #189 — Onslaught | Scryfall ID: e0ddcf4a-1943-49dd-a02c-75804ce4bc3e | Oracle ID: b3595359-0d40-495e-85da-6a56fbce7357
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLISTERING_FIRECAT,
    oracle_id = "b3595359-0d40-495e-85da-6a56fbce7357",
    scryfall_id = "e0ddcf4a-1943-49dd-a02c-75804ce4bc3e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Blistering Firecat",
        mana_cost = mana!("{1}{R}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL, subtypes::creature::CAT],
        power = Some(7),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
