//! Simulacrum — {1}{B} — Instant
//! Oracle: You gain life equal to the damage dealt to you this turn. Simulacrum deals damage to target creature you control equal to the damage dealt to you this turn.
//! Set: 4ED #161 — Fourth Edition | Scryfall ID: 7c23232c-a264-4df9-824b-4111a5c6524c | Oracle ID: 20d69989-7250-40c7-a064-8ed78ccbe556
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SIMULACRUM,
    oracle_id = "20d69989-7250-40c7-a064-8ed78ccbe556",
    scryfall_id = "7c23232c-a264-4df9-824b-4111a5c6524c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Simulacrum",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
