//! Plains — (no cost) — Basic Land — Plains
//! Oracle: ({T}: Add {W}.)
//! Set: TRK #317 — Star Trek | Scryfall ID: 8ab0f4c0-b331-4c57-b68f-2e24bb5ba06c | Oracle ID: bc71ebf6-2056-41f7-be35-b2e5c34afa99
// IMPLEMENTED — basic land mana ability.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

card! {
    index: 115,
    oracle_id: "bc71ebf6-2056-41f7-be35-b2e5c34afa99",
    scryfall_id: "8ab0f4c0-b331-4c57-b68f-2e24bb5ba06c",
    faces: &[face! {
        name: "Plains",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::BASIC,
        subtypes: &[land::PLAINS],
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])],
}
