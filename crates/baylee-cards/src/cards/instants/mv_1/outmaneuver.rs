//! Outmaneuver — {X}{R} — Instant
//! Oracle: X target blocked creatures assign their combat damage this turn as though they weren't blocked.
//! Set: USG #205 — Urza's Saga | Scryfall ID: 4a5a69fa-71ff-4e4f-9406-7cfebccb3384 | Oracle ID: 42b44108-4a03-4333-bf5a-e42493f4d5e4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OUTMANEUVER,
    oracle_id = "42b44108-4a03-4333-bf5a-e42493f4d5e4",
    scryfall_id = "4a5a69fa-71ff-4e4f-9406-7cfebccb3384",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Outmaneuver",
        mana_cost = mana!("{X}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
