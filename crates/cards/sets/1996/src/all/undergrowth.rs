//! Undergrowth — {G} — Instant
//! Oracle: As an additional cost to cast this spell, you may pay {2}{R}.
//! Oracle: Prevent all combat damage that would be dealt this turn. If this spell's additional cost was paid, this effect doesn't affect combat damage that would be dealt by red creatures.
//! Set: ALL #102a — Alliances | Scryfall ID: 9ade0829-8b90-4ed2-99f3-dd748e7706b8 | Oracle ID: f125b6b0-c5ff-44d1-bf99-ebc537d37fc3

// PARTIAL — "Prevent all combat damage that would be dealt this turn", cast without the optional {2}{R} additional cost.
// NOT SUPPORTED: "As an additional cost to cast this spell, you may pay {2}{R}. ... If this spell's additional cost was paid, this effect doesn't affect combat damage that would be dealt by red creatures." No effect prevents all combat damage except by sources matching a filter, so the additional cost is not offered: paying it would change nothing, or would prevent the combat damage of red creatures that the printed spell lets through.

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
    ),],
    coverage = Coverage::Partial(
        "Only the plain cast is played: all combat damage that would be dealt this turn is prevented. The optional {2}{R} additional cost, which exempts the combat damage of red creatures, is not offered because no effect prevents all combat damage except by sources matching a filter."
    ),
    abilities = &[spell!(&[Effect::PreventAllCombatDamageThisTurn])],
);
