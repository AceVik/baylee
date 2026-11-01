//! Bloodstoke Howler — {5}{R} — Creature — Beast
//! Oracle: Morph {6}{R} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, Beast creatures you control get +3/+0 until end of turn.
//! Set: LGN #89 — Legions | Scryfall ID: 743779d4-fee8-4b8d-a5ac-27f355e006e5 | Oracle ID: 41fa3954-119b-4ff2-ad56-9105482f6b7c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLOODSTOKE_HOWLER,
    oracle_id = "41fa3954-119b-4ff2-ad56-9105482f6b7c",
    scryfall_id = "743779d4-fee8-4b8d-a5ac-27f355e006e5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Bloodstoke Howler",
        mana_cost = mana!("{5}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
