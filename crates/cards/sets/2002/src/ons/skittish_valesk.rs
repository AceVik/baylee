//! Skittish Valesk — {6}{R} — Creature — Beast
//! Oracle: At the beginning of your upkeep, flip a coin. If you lose the flip, turn this creature face down.
//! Oracle: Morph {5}{R} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #231 — Onslaught | Scryfall ID: 4cc8a6e6-ed62-4784-ba9a-b1f703fc6119 | Oracle ID: d094f78b-acec-4181-9fee-ca523fb8ccc5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SKITTISH_VALESK,
    oracle_id = "d094f78b-acec-4181-9fee-ca523fb8ccc5",
    scryfall_id = "4cc8a6e6-ed62-4784-ba9a-b1f703fc6119",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Skittish Valesk",
        mana_cost = mana!("{6}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
