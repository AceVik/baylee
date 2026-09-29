//! Dragon Whelp — {2}{R}{R} — Creature — Dragon
//! Oracle: Flying
//! Oracle: {R}: This creature gets +1/+0 until end of turn. If this ability has been activated four or more times this turn, sacrifice this creature at the beginning of the next end step.
//! Set: PLST #DMR-116 — The List | Scryfall ID: 7f7bdfb7-1dc1-4ddf-8146-9276dff37b2b | Oracle ID: 705a1985-ed39-4a4b-812e-a677170b596e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DRAGON_WHELP,
    oracle_id = "705a1985-ed39-4a4b-812e-a677170b596e",
    scryfall_id = "7f7bdfb7-1dc1-4ddf-8146-9276dff37b2b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Dragon Whelp",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
