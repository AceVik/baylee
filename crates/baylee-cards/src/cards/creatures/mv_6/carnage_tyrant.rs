//! Carnage Tyrant — {4}{G}{G} — Creature — Dinosaur
//! Oracle: This spell can't be countered.
//! Oracle: Trample, hexproof
//! Set: XLN #179 — Ixalan | Scryfall ID: 3bd78731-949c-464a-826a-92f86d784911 | Oracle ID: 8c411f4e-a091-447c-9450-d895b10b4985
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CARNAGE_TYRANT,
    oracle_id = "8c411f4e-a091-447c-9450-d895b10b4985",
    scryfall_id = "3bd78731-949c-464a-826a-92f86d784911",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Carnage Tyrant",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DINOSAUR],
        power = Some(7),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
