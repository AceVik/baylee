//! Raugrin Triome — (no cost) — Land — ISLAND MOUNTAIN PLAINS
//! Oracle: ({T}: Add {U}, {R}, or {W}.)
//! Raugrin Triome enters the battlefield tapped.
//! Cycling {2}
//! Set: IKO #251 — Ikoria: Lair of Behemoths | Scryfall ID: 02138fbb-3962-4348-8d31-faaefba0b8b2 | Oracle ID: c7fa1dda-9312-4ec8-82cd-a1ba7bc33497
// IMPLEMENTED — triome (3 land types → intrinsic mana, ETB tapped, cycling {2}).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{self};

card! {
    index: 123,
    oracle_id: "c7fa1dda-9312-4ec8-82cd-a1ba7bc33497",
    scryfall_id: "02138fbb-3962-4348-8d31-faaefba0b8b2",
    faces: &[face! {
        name: "Raugrin Triome",
        types: TypeSet::LAND,
        subtypes: &[
            subtypes::land::ISLAND,
            subtypes::land::MOUNTAIN,
            subtypes::land::PLAINS,
        ],
        enter_modifiers: &[EnterModifier::Tapped],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Red, Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana_choice(&[
                ManaColor::Blue,
                ManaColor::Red,
                ManaColor::White,
            ])]),
        activated!(Cost {
                mana: baylee_core::mana!("{2}"),
                parts: &[CostPart::DiscardSelf],
            }, &[Effect::DrawCards {
                amount: Amount::Fixed(1),
            }], zone: ActivationZone::Hand),
    ],
}
