//! Cauldron Dance — {4}{B}{R} — Instant
//! Oracle: Cast this spell only during combat.
//! Oracle: Return target creature card from your graveyard to the battlefield. That creature gains haste. Return it to your hand at the beginning of the next end step.
//! Oracle: You may put a creature card from your hand onto the battlefield. That creature gains haste. Its controller sacrifices it at the beginning of the next end step.
//! Set: C17 #166 — Commander 2017 | Scryfall ID: 50e70e67-eba0-46f4-97d8-62c249b267c8 | Oracle ID: e9784eb4-96f2-409e-a7b5-eb6478644943
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CAULDRON_DANCE,
    oracle_id = "e9784eb4-96f2-409e-a7b5-eb6478644943",
    scryfall_id = "50e70e67-eba0-46f4-97d8-62c249b267c8",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(
        name = "Cauldron Dance",
        mana_cost = mana!("{4}{B}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
