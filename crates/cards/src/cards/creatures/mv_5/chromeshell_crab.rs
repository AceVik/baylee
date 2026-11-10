//! Chromeshell Crab — {4}{U} — Creature — Crab Beast
//! Oracle: Morph {4}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, you may exchange control of target creature you control and target creature an opponent controls.
//! Set: C19 #81 — Commander 2019 | Scryfall ID: c91cf95f-5007-409c-b891-00e10a3477e0 | Oracle ID: 8cbac2e8-5755-433b-9f42-7fd5c214aab3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHROMESHELL_CRAB,
    oracle_id = "8cbac2e8-5755-433b-9f42-7fd5c214aab3",
    scryfall_id = "c91cf95f-5007-409c-b891-00e10a3477e0",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Chromeshell Crab",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CRAB, subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
