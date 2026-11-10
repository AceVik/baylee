//! Swooping Talon — {4}{W}{W} — Creature — Bird Soldier
//! Oracle: Flying
//! Oracle: {1}: This creature loses flying until end of turn.
//! Oracle: Provoke (Whenever this creature attacks, you may have target creature defending player controls untap and block it if able.)
//! Set: LGN #23 — Legions | Scryfall ID: 34c3d19c-d4c6-4c5c-85eb-11d55959a89c | Oracle ID: c292d5cc-b6f4-4bb5-bc74-ff67e0643293
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SWOOPING_TALON,
    oracle_id = "c292d5cc-b6f4-4bb5-bc74-ff67e0643293",
    scryfall_id = "34c3d19c-d4c6-4c5c-85eb-11d55959a89c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Swooping Talon",
        mana_cost = mana!("{4}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
