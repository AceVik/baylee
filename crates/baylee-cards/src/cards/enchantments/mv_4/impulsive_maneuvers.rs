//! Impulsive Maneuvers — {2}{R}{R} — Enchantment
//! Oracle: Whenever a creature attacks, flip a coin. If you win the flip, the next time that creature would deal combat damage this turn, it deals double that damage instead. If you lose the flip, the next time that creature would deal combat damage this turn, prevent that damage.
//! Set: ODY #197 — Odyssey | Scryfall ID: d9b404c0-9ce1-4491-82fd-6dba7e6895cd | Oracle ID: 39a9323d-dddc-42ac-929d-3f4fa7c87567
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::IMPULSIVE_MANEUVERS,
    oracle_id = "39a9323d-dddc-42ac-929d-3f4fa7c87567",
    scryfall_id = "d9b404c0-9ce1-4491-82fd-6dba7e6895cd",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Impulsive Maneuvers",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
