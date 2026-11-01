//! Forgotten Ancient — {3}{G} — Creature — Elemental
//! Oracle: Whenever a player casts a spell, you may put a +1/+1 counter on this creature.
//! Oracle: At the beginning of your upkeep, you may move any number of +1/+1 counters from this creature onto other creatures.
//! Set: SOC #267 — Secrets of Strixhaven Commander | Scryfall ID: f8c9d744-8294-4747-ae7e-99be2b6e7479 | Oracle ID: 2ef75cf3-30dc-4852-a6d1-1d6391fad022
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FORGOTTEN_ANCIENT,
    oracle_id = "2ef75cf3-30dc-4852-a6d1-1d6391fad022",
    scryfall_id = "f8c9d744-8294-4747-ae7e-99be2b6e7479",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Forgotten Ancient",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(0),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
