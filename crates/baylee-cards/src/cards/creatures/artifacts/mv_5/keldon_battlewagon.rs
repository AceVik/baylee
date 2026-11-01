//! Keldon Battlewagon — {5} — Artifact Creature — Juggernaut
//! Oracle: Trample
//! Oracle: This creature can't block.
//! Oracle: When this creature attacks, sacrifice it at end of combat.
//! Oracle: Tap an untapped creature you control: This creature gets +X/+0 until end of turn, where X is the power of the creature tapped this way.
//! Set: PCY #139 — Prophecy | Scryfall ID: 5c810aa1-e367-4102-a5bd-6dc02d3023e8 | Oracle ID: 4a2df44b-5d36-4c4e-b982-836d9d7e0505
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KELDON_BATTLEWAGON,
    oracle_id = "4a2df44b-5d36-4c4e-b982-836d9d7e0505",
    scryfall_id = "5c810aa1-e367-4102-a5bd-6dc02d3023e8",
    faces = &[face!(
        name = "Keldon Battlewagon",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::JUGGERNAUT],
        power = Some(0),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
