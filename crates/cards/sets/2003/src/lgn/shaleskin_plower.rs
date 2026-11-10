//! Shaleskin Plower — {3}{R} — Creature — Beast
//! Oracle: Morph {4}{R} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, destroy target land.
//! Set: LGN #110 — Legions | Scryfall ID: 42658b33-9a12-403b-bc7d-807fbe1f1a36 | Oracle ID: 859502df-127f-4577-88c0-923119d37b9d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHALESKIN_PLOWER,
    oracle_id = "859502df-127f-4577-88c0-923119d37b9d",
    scryfall_id = "42658b33-9a12-403b-bc7d-807fbe1f1a36",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Shaleskin Plower",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
