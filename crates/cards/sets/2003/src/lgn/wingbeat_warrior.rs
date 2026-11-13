//! Wingbeat Warrior — {2}{W} — Creature — Bird Soldier Warrior
//! Oracle: Flying
//! Oracle: Morph {2}{W} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, target creature gains first strike until end of turn.
//! Set: LGN #29 — Legions | Scryfall ID: cd58d164-861d-4c80-ad2f-6283ed82faa1 | Oracle ID: 857392fe-1ce6-4d53-b039-98339d292ebb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WINGBEAT_WARRIOR,
    oracle_id = "857392fe-1ce6-4d53-b039-98339d292ebb",
    scryfall_id = "cd58d164-861d-4c80-ad2f-6283ed82faa1",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Wingbeat Warrior",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::BIRD,
            subtypes::creature::SOLDIER,
            subtypes::creature::WARRIOR
        ],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
