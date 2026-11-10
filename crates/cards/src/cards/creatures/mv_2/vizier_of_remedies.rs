//! Vizier of Remedies — {1}{W} — Creature — Human Cleric
//! Oracle: If one or more -1/-1 counters would be put on a creature you control, that many -1/-1 counters minus one are put on it instead.
//! Set: AKH #38 — Amonkhet | Scryfall ID: 36ab760e-93e0-4dbc-aaa1-02316f62ed3f | Oracle ID: 79770e65-740a-44c7-bea2-a24e6a722c22
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VIZIER_OF_REMEDIES,
    oracle_id = "79770e65-740a-44c7-bea2-a24e6a722c22",
    scryfall_id = "36ab760e-93e0-4dbc-aaa1-02316f62ed3f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Vizier of Remedies",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
