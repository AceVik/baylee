//! Trap Runner — {2}{W}{W} — Creature — Human Soldier
//! Oracle: {T}: Target unblocked attacking creature becomes blocked. Activate only during combat after blockers are declared. (This ability works on creatures that can't be blocked.)
//! Set: MMQ #55 — Mercadian Masques | Scryfall ID: eba97681-1d1f-4ab6-a21b-fbbbe63a1c74 | Oracle ID: 730fbd6a-096c-4d25-9372-c05f90088f83
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TRAP_RUNNER,
    oracle_id = "730fbd6a-096c-4d25-9372-c05f90088f83",
    scryfall_id = "eba97681-1d1f-4ab6-a21b-fbbbe63a1c74",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Trap Runner",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
