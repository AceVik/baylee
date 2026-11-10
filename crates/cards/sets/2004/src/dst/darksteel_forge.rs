//! Darksteel Forge — {9} — Artifact
//! Oracle: Artifacts you control have indestructible. (Effects that say "destroy" don't destroy them. Artifact creatures with indestructible can't be destroyed by damage.)
//! Set: 2XM #248 — Double Masters | Scryfall ID: 421089c4-c8d3-48c5-b313-fb1741546271 | Oracle ID: 9b3bec05-441f-4fdf-8b51-69fa8613fcd4
// IMPLEMENTED — indestructible grant to your artifacts (layer 6).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DARKSTEEL_FORGE,
    oracle_id = "9b3bec05-441f-4fdf-8b51-69fa8613fcd4",
    scryfall_id = "421089c4-c8d3-48c5-b313-fb1741546271",
    faces = &[face!(
        name = "Darksteel Forge",
        mana_cost = mana!("{9}"),
        types = TypeSet::ARTIFACT,
    )],
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::YOUR_ARTIFACT,
        Modifier::AddKeyword(KeywordSet::INDESTRUCTIBLE)
    )],
);
