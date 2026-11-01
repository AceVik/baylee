//! Spike Cannibal — {1}{B}{B} — Creature — Spike
//! Oracle: This creature enters with a +1/+1 counter on it.
//! Oracle: When this creature enters, move all +1/+1 counters from all creatures onto it.
//! Set: EXO #75 — Exodus | Scryfall ID: 64003772-c62f-4728-a00c-48c78991c6ae | Oracle ID: c175ab0e-7085-41ec-83e4-38fc85c338fa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPIKE_CANNIBAL,
    oracle_id = "c175ab0e-7085-41ec-83e4-38fc85c338fa",
    scryfall_id = "64003772-c62f-4728-a00c-48c78991c6ae",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Spike Cannibal",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIKE],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
