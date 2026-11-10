//! Trickery Charm — {U} — Instant
//! Oracle: Choose one —
//! Oracle: • Target creature gains flying until end of turn.
//! Oracle: • Target creature becomes the creature type of your choice until end of turn.
//! Oracle: • Look at the top four cards of your library, then put them back in any order.
//! Set: ONS #119 — Onslaught | Scryfall ID: 32a2ee45-7f1d-40a8-82b4-ab3b705417ea | Oracle ID: bb97f7c6-6a75-4f4e-b74e-c57ce98a08fc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TRICKERY_CHARM,
    oracle_id = "bb97f7c6-6a75-4f4e-b74e-c57ce98a08fc",
    scryfall_id = "32a2ee45-7f1d-40a8-82b4-ab3b705417ea",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Trickery Charm",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
