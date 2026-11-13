//! Otarian Juggernaut — {4} — Artifact Creature — Juggernaut
//! Oracle: This creature can't be blocked by Walls.
//! Oracle: Threshold — As long as there are seven or more cards in your graveyard, this creature gets +3/+0 and attacks each combat if able.
//! Set: ODY #305 — Odyssey | Scryfall ID: 752420dc-d69a-4c9d-b563-f8928a4a0920 | Oracle ID: c19f79bb-a608-4455-98ef-aac129cea8c5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::OTARIAN_JUGGERNAUT,
    oracle_id = "c19f79bb-a608-4455-98ef-aac129cea8c5",
    scryfall_id = "752420dc-d69a-4c9d-b563-f8928a4a0920",
    faces = &[face!(
        name = "Otarian Juggernaut",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::JUGGERNAUT],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
