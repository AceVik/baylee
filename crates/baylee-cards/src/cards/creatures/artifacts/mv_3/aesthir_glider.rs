//! Aesthir Glider — {3} — Artifact Creature — Bird Construct
//! Oracle: Flying
//! Oracle: This creature can't block.
//! Set: DOM #209 — Dominaria | Scryfall ID: 25afd4a7-4253-4a4e-a105-e92f64460faa | Oracle ID: 99dcde0b-4f4d-4d7a-89aa-8a89c256e38f

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AESTHIR_GLIDER,
    oracle_id = "99dcde0b-4f4d-4d7a-89aa-8a89c256e38f",
    scryfall_id = "25afd4a7-4253-4a4e-a105-e92f64460faa",
    keywords = KeywordSet::FLYING.union(KeywordSet::CANT_BLOCK),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Aesthir Glider",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::CONSTRUCT],
        power = Some(2),
        toughness = Some(1),
    ),],
);
