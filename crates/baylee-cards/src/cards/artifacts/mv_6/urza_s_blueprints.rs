//! Urza's Blueprints — {6} — Artifact
//! Oracle: Echo {6} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: {T}: Draw a card.
//! Set: DMR #238 — Dominaria Remastered | Scryfall ID: 1fd2b6a7-b93a-42d3-b66a-a70b573fc58a | Oracle ID: ad75efdb-3e36-4253-be47-1df5a3cf8204
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::URZA_S_BLUEPRINTS,
    oracle_id = "ad75efdb-3e36-4253-be47-1df5a3cf8204",
    scryfall_id = "1fd2b6a7-b93a-42d3-b66a-a70b573fc58a",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Urza's Blueprints",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        AbilityDef::Echo { cost: mana!("{6}") },
        activated!(cost!(TapSelf), &[Effect::draw(1)]),
    ],
);
