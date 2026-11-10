//! Loafing Giant — {4}{R} — Creature — Giant
//! Oracle: Whenever this creature attacks or blocks, mill a card. If a land card was milled this way, prevent all combat damage this creature would deal this turn.
//! Set: INV #153 — Invasion | Scryfall ID: fab5f738-04d0-44c9-88ec-28469b668040 | Oracle ID: 038dce4c-f754-45ef-98a4-4e16f931a65c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LOAFING_GIANT,
    oracle_id = "038dce4c-f754-45ef-98a4-4e16f931a65c",
    scryfall_id = "fab5f738-04d0-44c9-88ec-28469b668040",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Loafing Giant",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GIANT],
        power = Some(4),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
