//! Cascade Bluffs — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {U/R}, {T}: Add {U}{U}, {U}{R}, or {R}{R}.
//! Set: SOC #364 — Secrets of Strixhaven Commander | Scryfall ID: 22acccd2-4e9d-46de-a060-863b08152e50 | Oracle ID: f1603384-4361-49c9-98aa-7785fc3504c4
// IMPLEMENTED — filter land (colorless tap + {U/R},{T} for two combination mana of {U} and/or {R}).

use baylee_cards_dsl::prelude::*;

card! {
    index: 330,
    oracle_id: "f1603384-4361-49c9-98aa-7785fc3504c4",
    scryfall_id: "22acccd2-4e9d-46de-a060-863b08152e50",
    faces: &[face! {
        name: "Cascade Bluffs",
        types: TypeSet::LAND,
    }],
    color_identity: ColorSet::from_slice(&[Color::Red, Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(Cost {
                mana: baylee_core::mana!("{U/R}"),
                parts: &[CostPart::TapSelf],
            }, &[Effect::mana_combination(
                &[ManaColor::Blue, ManaColor::Red],
                Amount::Fixed(2),
            )]),
    ],
}
