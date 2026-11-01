//! Tectonic Break — {X}{R}{R} — Sorcery
//! Oracle: Each player sacrifices X lands of their choice.
//! Set: MMQ #216 — Mercadian Masques | Scryfall ID: 9de0ee5d-10f6-4152-8416-1f2b749b439d | Oracle ID: 4a3c6d0e-6d77-4434-95f4-737879de026f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TECTONIC_BREAK,
    oracle_id = "4a3c6d0e-6d77-4434-95f4-737879de026f",
    scryfall_id = "9de0ee5d-10f6-4152-8416-1f2b749b439d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Tectonic Break",
        mana_cost = mana!("{X}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
