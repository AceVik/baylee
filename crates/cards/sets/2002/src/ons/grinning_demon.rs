//! Grinning Demon — {2}{B}{B} — Creature — Demon
//! Oracle: At the beginning of your upkeep, you lose 2 life.
//! Oracle: Morph {2}{B}{B} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #153 — Onslaught | Scryfall ID: 72de2f66-0b86-4c21-b4c8-c2d97e3fd095 | Oracle ID: 3db0207d-d80b-4915-b157-6a3e5b3a154e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GRINNING_DEMON,
    oracle_id = "3db0207d-d80b-4915-b157-6a3e5b3a154e",
    scryfall_id = "72de2f66-0b86-4c21-b4c8-c2d97e3fd095",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Grinning Demon",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DEMON],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
