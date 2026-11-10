//! Rhystic Shield — {1}{W} — Instant
//! Oracle: Creatures you control get +0/+1 until end of turn. They get an additional +0/+2 until end of turn unless any player pays {2}.
//! Set: PCY #20 — Prophecy | Scryfall ID: 49af7b3f-f56a-4102-b398-5c215dd4fa11 | Oracle ID: a905dafa-1818-471a-a485-d84be447263b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RHYSTIC_SHIELD,
    oracle_id = "a905dafa-1818-471a-a485-d84be447263b",
    scryfall_id = "49af7b3f-f56a-4102-b398-5c215dd4fa11",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Rhystic Shield",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
