//! Brightclimb Pathway // Grimclimb Pathway — (no cost) — Land // Land
//! Oracle: Brightclimb Pathway: {T}: Add {W}. // Grimclimb Pathway: {T}: Add {B}.
//! Set: ZNR #259 — Zendikar Rising | Scryfall ID: d24c3d51-795d-4c01-a34a-3280fccd2d78 | Oracle ID: 1c633e02-95ef-445e-b4e0-fbfbc5ed9cc9
// IMPLEMENTED — MDFC land-face choice on play (CR 712.4a) + per-face
// mana abilities.

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])];

use baylee_cards_dsl::prelude::*;

card! {
    index: 16,
    oracle_id: "1c633e02-95ef-445e-b4e0-fbfbc5ed9cc9",
    scryfall_id: "d24c3d51-795d-4c01-a34a-3280fccd2d78",
    faces: &[
        face! {
            name: "Brightclimb Pathway",
            types: TypeSet::LAND,
        },
        face! {
            name: "Grimclimb Pathway",
            types: TypeSet::LAND,
            abilities: BACK_MANA,
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])],
}
