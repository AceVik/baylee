//! Reward the Faithful — {W} — Instant
//! Oracle: Any number of target players each gain life equal to the greatest mana value among permanents you control.
//! Set: SCG #22 — Scourge | Scryfall ID: df6e8844-3736-4fb1-bedb-6a6bfa6ccdc8 | Oracle ID: 7fb7c848-377c-4476-9b3f-d95d145137f1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REWARD_THE_FAITHFUL,
    oracle_id = "7fb7c848-377c-4476-9b3f-d95d145137f1",
    scryfall_id = "df6e8844-3736-4fb1-bedb-6a6bfa6ccdc8",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Reward the Faithful",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
