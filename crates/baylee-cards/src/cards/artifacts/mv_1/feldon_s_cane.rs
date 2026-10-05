//! Feldon's Cane — {1} — Artifact
//! Oracle: {T}, Exile this artifact: Shuffle your graveyard into your library.
//! Set: FDN #673 — Foundations | Scryfall ID: 026f55c3-8e65-4520-b26d-6208344f73b2 | Oracle ID: 9b884dfd-59f4-45c0-bf1e-6ad9f5b58895
// IMPLEMENTED — {T}, exile this artifact shuffles your graveyard into your
// library.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FELDON_S_CANE,
    oracle_id = "9b884dfd-59f4-45c0-bf1e-6ad9f5b58895",
    scryfall_id = "026f55c3-8e65-4520-b26d-6208344f73b2",
    faces = &[face!(
        name = "Feldon's Cane",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!(TapSelf, ExileSelf),
        &[Effect::ShuffleGraveyardIntoLibrary]
    )],
);
