//! Shield Dancer — {2}{W} — Creature — Human Rebel
//! Oracle: {2}{W}: The next time target attacking creature would deal combat damage to this creature this turn, that creature deals that damage to itself instead.
//! Set: PCY #23 — Prophecy | Scryfall ID: 1d885360-1ce1-4b80-8928-29437731993f | Oracle ID: 0ca9c400-c62a-49d8-8dce-5caca7f92ed9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHIELD_DANCER,
    oracle_id = "0ca9c400-c62a-49d8-8dce-5caca7f92ed9",
    scryfall_id = "1d885360-1ce1-4b80-8928-29437731993f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Shield Dancer",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::REBEL],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
