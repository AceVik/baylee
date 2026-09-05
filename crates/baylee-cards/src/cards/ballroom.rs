//! Ballroom — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W} or {B}.
//! Oracle: {4}, {T}: Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
//! Set: CLU #12 — Ravnica: Clue Edition | Scryfall ID: 6e982bf8-382f-4987-bc39-28e1ce290340 | Oracle ID: cb91e842-9f06-4863-a328-2cabe1bcfe27
// IMPLEMENTED — enters tapped, taps for {W} or {B}, {4}, {T}: investigate.

use baylee_cards_dsl::prelude::*;

use crate::tokens::CLUE;

card! {
    index: 256,
    oracle_id: "cb91e842-9f06-4863-a328-2cabe1bcfe27",
    scryfall_id: "6e982bf8-382f-4987-bc39-28e1ce290340",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::White]),
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Ballroom",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::Tapped],
    },
    ],
    abilities: &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::White, ManaColor::Black])]),
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
