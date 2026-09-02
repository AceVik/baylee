//! Wasteland — (no cost) — Land
//! Oracle: {T}: Add {C}. {T}, Sacrifice this land: Destroy target nonbasic land.
//! Set: C17 #264 — Commander 2017 | Scryfall ID: aaafb9bc-7cea-4624-a227-595544fa42b0 | Oracle ID: 09a70ae8-3859-4a09-901d-dce063fa3b5f
// IMPLEMENTED.

use baylee_cards_dsl::prelude::*;

static NONBASIC_LAND: Filter = Filter::And(&[
    Filter::LAND,
    Filter::Not(&Filter::HasSupertype(SupertypeSet::BASIC)),
]);

card! {
    index: 188,
    oracle_id: "09a70ae8-3859-4a09-901d-dce063fa3b5f",
    scryfall_id: "aaafb9bc-7cea-4624-a227-595544fa42b0",
    faces: &[face! {
        name: "Wasteland",
        types: TypeSet::LAND,
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(Cost {
                mana: ManaCost::ZERO,
                parts: &[CostPart::TapSelf, CostPart::SacrificeSelf],
            }, &[Effect::Destroy {
                target: TargetSpec::Object(&NONBASIC_LAND),
            }], target: Some(TargetSpec::Object(&NONBASIC_LAND))),
    ],
}
