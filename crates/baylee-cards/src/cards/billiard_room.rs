//! Billiard Room — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B} or {R}.
//! Oracle: {4}, {T}: Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
//! Set: CLU #13 — Ravnica: Clue Edition | Scryfall ID: dc2a3de1-01ac-4425-8534-e5019e01f2cd | Oracle ID: a4fc174e-7fa6-41a8-ae03-255f226840f9
// IMPLEMENTED — enters tapped, taps for {B} or {R}, {4}, {T}: investigate.

use baylee_cards_dsl::prelude::*;

use crate::tokens::CLUE;

card! {
    index: 271,
    oracle_id: "a4fc174e-7fa6-41a8-ae03-255f226840f9",
    scryfall_id: "dc2a3de1-01ac-4425-8534-e5019e01f2cd",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Billiard Room",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::Tapped],
    },
    ],
    abilities: &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Black, ManaColor::Red])]),
        activated!(
            Cost {
                mana: baylee_core::mana!("{4}"),
                parts: &[CostPart::TapSelf],
            },
            &[Effect::CreateToken {
                token: &CLUE,
            }],
        ),
    ],
}
