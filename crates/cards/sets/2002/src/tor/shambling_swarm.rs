//! Shambling Swarm — {1}{B}{B}{B} — Creature — Horror
//! Oracle: When this creature dies, distribute three -1/-1 counters among one, two, or three target creatures. For each -1/-1 counter you put on a creature this way, remove a -1/-1 counter from that creature at the beginning of the next end step.
//! Set: TOR #82 — Torment | Scryfall ID: b5b93715-985f-4719-a3c3-044c2a150e96 | Oracle ID: 7807122e-917f-4a79-876e-d1c10f7a38ed
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHAMBLING_SWARM,
    oracle_id = "7807122e-917f-4a79-876e-d1c10f7a38ed",
    scryfall_id = "b5b93715-985f-4719-a3c3-044c2a150e96",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Shambling Swarm",
        mana_cost = mana!("{1}{B}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HORROR],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
