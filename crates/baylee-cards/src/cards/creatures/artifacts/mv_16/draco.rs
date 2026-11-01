//! Draco — {16} — Artifact Creature — Dragon
//! Oracle: Domain — This spell costs {2} less to cast for each basic land type among lands you control.
//! Oracle: Flying
//! Oracle: Domain — At the beginning of your upkeep, sacrifice this creature unless you pay {10}. This cost is reduced by {2} for each basic land type among lands you control.
//! Set: PLS #131 — Planeshift | Scryfall ID: 212e3edb-62f1-4680-884f-70323547f8ad | Oracle ID: 6157ade6-9037-4339-869b-493fdee71dcf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DRACO,
    oracle_id = "6157ade6-9037-4339-869b-493fdee71dcf",
    scryfall_id = "212e3edb-62f1-4680-884f-70323547f8ad",
    faces = &[face!(
        name = "Draco",
        mana_cost = mana!("{16}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(9),
        toughness = Some(9),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
