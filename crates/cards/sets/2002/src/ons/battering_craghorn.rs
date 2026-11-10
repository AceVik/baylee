//! Battering Craghorn — {2}{R}{R} — Creature — Goat Beast
//! Oracle: First strike
//! Oracle: Morph {1}{R}{R} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ARC #30 — Archenemy | Scryfall ID: 69786e07-1b2c-4af9-82f1-222fb9fb4e85 | Oracle ID: a084bca4-fe72-443e-adf4-ab5456c7e699
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BATTERING_CRAGHORN,
    oracle_id = "a084bca4-fe72-443e-adf4-ab5456c7e699",
    scryfall_id = "69786e07-1b2c-4af9-82f1-222fb9fb4e85",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Battering Craghorn",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOAT, subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
