//! Flame Burst — {1}{R} — Instant
//! Oracle: Flame Burst deals X damage to any target, where X is 2 plus the number of cards named Flame Burst in all graveyards.
//! Set: ODY #194 — Odyssey | Scryfall ID: 64bbd438-7df2-4d7b-88ad-4531ebaf3931 | Oracle ID: 37e44e1c-481d-4f5d-b76b-036f40e9124e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FLAME_BURST,
    oracle_id = "37e44e1c-481d-4f5d-b76b-036f40e9124e",
    scryfall_id = "64bbd438-7df2-4d7b-88ad-4531ebaf3931",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Flame Burst",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
