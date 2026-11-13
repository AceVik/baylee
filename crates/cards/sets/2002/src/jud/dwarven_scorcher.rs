//! Dwarven Scorcher — {R} — Creature — Dwarf
//! Oracle: Sacrifice this creature: This creature deals 1 damage to target creature unless that creature's controller has this creature deal 2 damage to them.
//! Set: JUD #86 — Judgment | Scryfall ID: 099873b1-7181-4b9d-8ce1-8ec63c814afe | Oracle ID: a896cedb-f10a-48ac-82e7-6ae1e8b5a439
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DWARVEN_SCORCHER,
    oracle_id = "a896cedb-f10a-48ac-82e7-6ae1e8b5a439",
    scryfall_id = "099873b1-7181-4b9d-8ce1-8ec63c814afe",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Dwarven Scorcher",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DWARF],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
