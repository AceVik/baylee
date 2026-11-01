//! Mogg Toady — {1}{R} — Creature — Goblin
//! Oracle: This creature can't attack unless you control more creatures than defending player.
//! Oracle: This creature can't block unless you control more creatures than attacking player.
//! Set: NEM #95 — Nemesis | Scryfall ID: ee8edaf6-d46e-4efb-8bc0-ec11e06eb499 | Oracle ID: 77c7ba06-d247-421d-9e0c-54c3000c1b78
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MOGG_TOADY,
    oracle_id = "77c7ba06-d247-421d-9e0c-54c3000c1b78",
    scryfall_id = "ee8edaf6-d46e-4efb-8bc0-ec11e06eb499",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Mogg Toady",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
