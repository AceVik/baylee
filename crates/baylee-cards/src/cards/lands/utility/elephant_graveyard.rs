//! Elephant Graveyard — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Regenerate target Elephant.
//! Set: ME4 #244 — Masters Edition IV | Scryfall ID: 88e7d9d5-3bca-4791-b850-5ae104706042 | Oracle ID: 8ada7388-fd8b-434c-a17a-bce19cf3e615
// PARTIAL — {T}: Add {C} built; the regenerate ability has no effect to hang on.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ELEPHANT_GRAVEYARD,
    oracle_id = "8ada7388-fd8b-434c-a17a-bce19cf3e615",
    scryfall_id = "88e7d9d5-3bca-4791-b850-5ae104706042",
    faces = &[face!(name = "Elephant Graveyard", types = TypeSet::LAND,),],
    coverage = Coverage::Partial("regenerate is not in the Effect vocabulary"),
    // NOT SUPPORTED: {T}: Regenerate target Elephant — no `Effect` variant
    // creates a regeneration shield, and `Modifier` has none either.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
