//! Charging Slateback — {4}{R} — Creature — Beast
//! Oracle: This creature can't block.
//! Oracle: Morph {4}{R} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #194 — Onslaught | Scryfall ID: d2cfff37-655f-4107-abf3-e6f63d0e4de2 | Oracle ID: 67457ecb-5ff2-4f5d-9039-b9e80f54bba8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHARGING_SLATEBACK,
    oracle_id = "67457ecb-5ff2-4f5d-9039-b9e80f54bba8",
    scryfall_id = "d2cfff37-655f-4107-abf3-e6f63d0e4de2",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Charging Slateback",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
