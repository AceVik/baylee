//! Trap Digger — {3}{W} — Creature — Human Soldier
//! Oracle: {2}{W}, {T}: Put a trap counter on target land you control.
//! Oracle: Sacrifice a land with a trap counter on it: This creature deals 3 damage to target attacking creature without flying.
//! Set: SCG #24 — Scourge | Scryfall ID: 05cd76bf-db08-45f8-b3ae-501bcca6df3c | Oracle ID: 91857f68-4b0e-4fdf-8782-74324b496648
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TRAP_DIGGER,
    oracle_id = "91857f68-4b0e-4fdf-8782-74324b496648",
    scryfall_id = "05cd76bf-db08-45f8-b3ae-501bcca6df3c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Trap Digger",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
