//! Kjeldoran Royal Guard — {3}{W}{W} — Creature — Human Soldier
//! Oracle: {T}: All combat damage that would be dealt to you by unblocked creatures this turn is dealt to this creature instead.
//! Set: 10E #25 — Tenth Edition | Scryfall ID: ea3d742a-52a1-45c8-961a-85b6ff120c44 | Oracle ID: 8137e765-4df9-469f-a527-dee91d58fb7e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KJELDORAN_ROYAL_GUARD,
    oracle_id = "8137e765-4df9-469f-a527-dee91d58fb7e",
    scryfall_id = "ea3d742a-52a1-45c8-961a-85b6ff120c44",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Kjeldoran Royal Guard",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
