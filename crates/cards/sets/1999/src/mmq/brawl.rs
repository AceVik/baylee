//! Brawl — {3}{R}{R} — Instant
//! Oracle: Until end of turn, all creatures gain "{T}: This creature deals damage equal to its power to target creature."
//! Set: MMQ #178 — Mercadian Masques | Scryfall ID: 3f4e783c-0717-4127-bd7b-885ca617ca29 | Oracle ID: 6d53f58c-6cf3-4ecc-8227-8c264fecc836
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BRAWL,
    oracle_id = "6d53f58c-6cf3-4ecc-8227-8c264fecc836",
    scryfall_id = "3f4e783c-0717-4127-bd7b-885ca617ca29",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Brawl",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
