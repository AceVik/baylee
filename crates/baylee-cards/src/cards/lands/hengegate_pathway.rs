//! Hengegate Pathway // Mistgate Pathway — (no cost) — Land // Land
//! Oracle: Hengegate Pathway: {T}: Add {W}. // Mistgate Pathway: {T}: Add {U}.
//! Set: ZNR #261 — Zendikar Rising | Scryfall ID: 7ef37cb3-d803-47d7-8a01-9c803aa2eadc | Oracle ID: 461b3f2f-fcee-4160-abfa-061f8b6a784f
// IMPLEMENTED — MDFC land-face choice on play (CR 712.4a) + per-face
// mana abilities.

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])];

use baylee_cards_dsl::prelude::*;

card! {
    index: 69,
    oracle_id: "461b3f2f-fcee-4160-abfa-061f8b6a784f",
    scryfall_id: "7ef37cb3-d803-47d7-8a01-9c803aa2eadc",
    faces: &[
        face! {
            name: "Hengegate Pathway",
            types: TypeSet::LAND,
        },
        face! {
            name: "Mistgate Pathway",
            types: TypeSet::LAND,
            abilities: BACK_MANA,
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])],
}
