//! Clearwater Pathway // Murkwater Pathway — (no cost) — Land // Land
//! Oracle: Clearwater Pathway: {T}: Add {U}. // Murkwater Pathway: {T}: Add {B}.
//! Set: ZNR #260 — Zendikar Rising | Scryfall ID: b4b99ebb-0d54-4fe5-a495-979aaa564aa8 | Oracle ID: 144119bc-7fd1-45c5-9e29-f742e7c255ac
// IMPLEMENTED — MDFC land-face choice on play (CR 712.4a) + per-face
// mana abilities.

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])];

use baylee_cards_dsl::prelude::*;

card! {
    index: 21,
    oracle_id: "144119bc-7fd1-45c5-9e29-f742e7c255ac",
    scryfall_id: "b4b99ebb-0d54-4fe5-a495-979aaa564aa8",
    faces: &[
        face! {
            name: "Clearwater Pathway",
            types: TypeSet::LAND,
        },
        face! {
            name: "Murkwater Pathway",
            types: TypeSet::LAND,
            abilities: BACK_MANA,
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])],
}
