//! Voidmage Prodigy — {U}{U} — Creature — Human Wizard
//! Oracle: {U}{U}, Sacrifice a Wizard: Counter target spell.
//! Oracle: Morph {U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: TSB #34 — Time Spiral Timeshifted | Scryfall ID: b26d5fc8-c148-4383-a1b9-f80da4d875aa | Oracle ID: 7e2cacae-2934-4ff5-81c6-dfce7a2820ef
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VOIDMAGE_PRODIGY,
    oracle_id = "7e2cacae-2934-4ff5-81c6-dfce7a2820ef",
    scryfall_id = "b26d5fc8-c148-4383-a1b9-f80da4d875aa",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Voidmage Prodigy",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
