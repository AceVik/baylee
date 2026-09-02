//! Supreme Verdict — {1}{W}{U}{U} — Sorcery
//! Oracle: This spell can't be countered.
//! Oracle: Destroy all creatures.
//! Set: RVR #67 — Ravnica Remastered | Scryfall ID: 3892f1c5-937e-4ef4-b6f9-e0c0ded070d0 | Oracle ID: 0230de18-8d15-4cfa-9d42-7ccddd9f9570
// IMPLEMENTED — uncounterable wrath.

use baylee_cards_dsl::prelude::*;

card! {
    index: 160,
    oracle_id: "0230de18-8d15-4cfa-9d42-7ccddd9f9570",
    scryfall_id: "3892f1c5-937e-4ef4-b6f9-e0c0ded070d0",
    faces: &[face! {
        name: "Supreme Verdict",
        mana_cost: baylee_core::mana!("{1}{W}{U}{U}"),
        types: TypeSet::SORCERY,
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue]),
    keywords: KeywordSet::UNCOUNTERABLE,
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::DestroyAll { filter: &Filter::CREATURE }])],
}
