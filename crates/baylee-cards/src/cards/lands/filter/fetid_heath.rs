//! Fetid Heath — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! {1}, {T}: Add two mana in any combination of {White} and/or {Black}.
//! Set: SHM #272 — Shadowmoor | Scryfall ID: f465ded8-0d38-42ac-bafc-a12185013c5d | Oracle ID: 42bf259d-4bb9-49c3-b4ec-223dca62f4d6
// IMPLEMENTED — filter land (colorless tap + {1},{T} for two combination mana).

use baylee_cards_dsl::prelude::*;

card! {
    index: 49,
    oracle_id: "42bf259d-4bb9-49c3-b4ec-223dca62f4d6",
    scryfall_id: "f465ded8-0d38-42ac-bafc-a12185013c5d",
    faces: &[face! {
        name: "Fetid Heath",
        types: TypeSet::LAND,
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(Cost {
                mana: baylee_core::mana!("{1}"),
                parts: &[CostPart::TapSelf],
            }, &[Effect::mana_combination(
                &[ManaColor::White, ManaColor::Black],
                Amount::Fixed(2),
            )]),
    ],
}
