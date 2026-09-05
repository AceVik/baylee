//! Barkchannel Pathway // Tidechannel Pathway — (no cost) — Land // Land
//! Set: KHM #251 — Kaldheim | Scryfall ID: b6de14ae-0132-4261-af00-630bf15918cd | Oracle ID: 59d22de5-e310-44d7-89cf-ef3529e40cef
//! Face: Barkchannel Pathway —  — Land
//! Face: Tidechannel Pathway —  — Land
// IMPLEMENTED — MDFC land-face choice on play (CR 712.4a) + per-face
// mana abilities.

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])];

use baylee_cards_dsl::prelude::*;

card! {
    index: 260,
    oracle_id: "59d22de5-e310-44d7-89cf-ef3529e40cef",
    scryfall_id: "b6de14ae-0132-4261-af00-630bf15918cd",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces: &[
        face! {
            name: "Barkchannel Pathway",
            types: TypeSet::LAND,
        },
        face! {
            name: "Tidechannel Pathway",
            types: TypeSet::LAND,
            abilities: BACK_MANA,
        },
    ],
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
}
