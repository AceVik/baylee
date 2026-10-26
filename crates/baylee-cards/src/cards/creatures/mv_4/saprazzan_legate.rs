//! Saprazzan Legate — {3}{U} — Creature — Merfolk Soldier
//! Oracle: If an opponent controls a Mountain and you control an Island, you may cast this spell without paying its mana cost.
//! Oracle: Flying
//! Set: MMQ #100 — Mercadian Masques | Scryfall ID: db9adf84-ee7e-472b-bd96-9abf853afa83 | Oracle ID: d20b62fc-e248-4c84-87ac-7319b92531d4
// NOT SUPPORTED: opponent-land plus your-land free-cast alternative cost not yet in DSL
// IMPLEMENTED — hand-written with tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SAPRAZZAN_LEGATE,
    oracle_id = "d20b62fc-e248-4c84-87ac-7319b92531d4",
    scryfall_id = "db9adf84-ee7e-472b-bd96-9abf853afa83",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage =
        Coverage::Partial("opponent-land plus your-land free-cast alternative cost not yet in DSL"),
    faces = &[face!(
        name = "Saprazzan Legate",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(3),
    ),],
);
