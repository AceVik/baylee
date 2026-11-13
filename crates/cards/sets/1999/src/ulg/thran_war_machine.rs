//! Thran War Machine — {4} — Artifact Creature — Construct
//! Oracle: Echo {4} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: This creature attacks each combat if able.
//! Set: ULG #134 — Urza's Legacy | Scryfall ID: 5908714a-be91-4279-b87e-e2bc09dbaaba | Oracle ID: b3b8fe34-4f69-4555-bcac-04563d02c99f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THRAN_WAR_MACHINE,
    oracle_id = "b3b8fe34-4f69-4555-bcac-04563d02c99f",
    scryfall_id = "5908714a-be91-4279-b87e-e2bc09dbaaba",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Thran War Machine",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(4),
        toughness = Some(5),
    ),],
    abilities = &[
        AbilityDef::Echo { cost: mana!("{4}") },
        static_ability!(Filter::This, Modifier::AttacksEachCombat),
    ],
);
