//! Unleash Shell — {3}{R}{R} — Instant
//! Oracle: Unleash Shell deals 5 damage to target creature or planeswalker and 2 damage to that permanent's controller.
//! Set: BRO #155 — The Brothers' War | Scryfall ID: 1be13abf-b717-4122-ab2b-7e597a251d52 | Oracle ID: 9eaca93d-6c3d-4489-8652-b575a4cd931f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UNLEASH_SHELL,
    oracle_id = "9eaca93d-6c3d-4489-8652-b575a4cd931f",
    scryfall_id = "1be13abf-b717-4122-ab2b-7e597a251d52",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Unleash Shell",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
