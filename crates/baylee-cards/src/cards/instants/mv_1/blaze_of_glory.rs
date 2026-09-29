//! Blaze of Glory — {W} — Instant
//! Oracle: Cast this spell only during combat before blockers are declared.
//! Oracle: Target creature defending player controls can block any number of creatures this turn. It blocks each attacking creature this turn if able.
//! Set: ME4 #7 — Masters Edition IV | Scryfall ID: 2f6ed7bd-2186-4e92-9fbc-e2abbfd3bc23 | Oracle ID: b330ac89-790e-4cc9-96a5-532c48252088
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLAZE_OF_GLORY,
    oracle_id = "b330ac89-790e-4cc9-96a5-532c48252088",
    scryfall_id = "2f6ed7bd-2186-4e92-9fbc-e2abbfd3bc23",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Blaze of Glory",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
