//! Goblin Machinist — {4}{R} — Creature — Goblin
//! Oracle: {2}{R}: Reveal cards from the top of your library until you reveal a nonland card. This creature gets +X/+0 until end of turn, where X is that card's mana value. Put the revealed cards on the bottom of your library in any order.
//! Set: ONS #204 — Onslaught | Scryfall ID: 5874e312-1010-43f2-b330-82bc9fcc9f53 | Oracle ID: c6c4cc65-6ccb-437b-ae97-3201be95b6df
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_MACHINIST,
    oracle_id = "c6c4cc65-6ccb-437b-ae97-3201be95b6df",
    scryfall_id = "5874e312-1010-43f2-b330-82bc9fcc9f53",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Goblin Machinist",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(0),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
