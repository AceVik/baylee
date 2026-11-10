//! Dark Sphere — {0} — Artifact
//! Oracle: {T}, Sacrifice this artifact: The next time a source of your choice would deal damage to you this turn, prevent half that damage, rounded down.
//! Set: DRK #100 — The Dark | Scryfall ID: 72cfe9b9-677d-4ecb-83ab-67fb6481371d | Oracle ID: 397f53f7-f801-4442-a778-2f26ac246b62

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DARK_SPHERE,
    oracle_id = "397f53f7-f801-4442-a778-2f26ac246b62",
    scryfall_id = "72cfe9b9-677d-4ecb-83ab-67fb6481371d",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dark Sphere",
        mana_cost = mana!("{0}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!(TapSelf, SacrificeSelf),
        &[Effect::PreventNextFromChosenSource {
            sources: &Filter::Any,
            combat_only: false,
            all_but: u8::MAX,
            gain_life: false,
        }]
    )],
);
