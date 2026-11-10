//! Lowland Tracker — {4}{W} — Creature — Human Soldier
//! Oracle: First strike
//! Oracle: Provoke (Whenever this creature attacks, you may have target creature defending player controls untap and block it if able.)
//! Set: LGN #17 — Legions | Scryfall ID: f8eaaded-b18a-4614-b5b5-b4bb49a2e1b1 | Oracle ID: de06b33f-0793-419a-bf65-1961634af8e7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LOWLAND_TRACKER,
    oracle_id = "de06b33f-0793-419a-bf65-1961634af8e7",
    scryfall_id = "f8eaaded-b18a-4614-b5b5-b4bb49a2e1b1",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Lowland Tracker",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
