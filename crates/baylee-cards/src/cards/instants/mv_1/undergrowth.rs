//! Undergrowth — {G} — Instant
//! Oracle: As an additional cost to cast this spell, you may pay {2}{R}.
//! Oracle: Prevent all combat damage that would be dealt this turn. If this spell's additional cost was paid, this effect doesn't affect combat damage that would be dealt by red creatures.
//! Set: ALL #102a — Alliances | Scryfall ID: 9ade0829-8b90-4ed2-99f3-dd748e7706b8 | Oracle ID: f125b6b0-c5ff-44d1-bf99-ebc537d37fc3

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UNDERGROWTH,
    oracle_id = "f125b6b0-c5ff-44d1-bf99-ebc537d37fc3",
    scryfall_id = "9ade0829-8b90-4ed2-99f3-dd748e7706b8",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[face!(
        name = "Undergrowth",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
        additional_costs = &[cost!("{2}{R}")],
    ),],
    coverage = Coverage::Partial(
        "Prevents all combat damage that would be dealt this turn; paying the {2}{R} additional cost does not exempt combat damage dealt by red creatures."
    ),
    abilities = &[spell!(&[Effect::PreventAllCombatDamageThisTurn])],
);
