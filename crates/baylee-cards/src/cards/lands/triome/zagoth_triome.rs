//! Zagoth Triome — (no cost) — Land — SWAMP FOREST ISLAND
//! Oracle: ({T}: Add {B}, {G}, or {U}.)
//! Zagoth Triome enters the battlefield tapped.
//! Cycling {2}
//! Set: IKO #259 — Ikoria: Lair of Behemoths | Scryfall ID: cc520518-2063-4b57-a0d4-10cf62a7175e | Oracle ID: fdd46004-eaba-4024-8687-39b23dc6a58c
// IMPLEMENTED — triome (3 land types → intrinsic mana, ETB tapped, cycling {2}).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{self};

card! {
    index: 193,
    oracle_id: "fdd46004-eaba-4024-8687-39b23dc6a58c",
    scryfall_id: "cc520518-2063-4b57-a0d4-10cf62a7175e",
    faces: &[face! {
        name: "Zagoth Triome",
        types: TypeSet::LAND,
        subtypes: &[
            subtypes::land::SWAMP,
            subtypes::land::FOREST,
            subtypes::land::ISLAND,
        ],
        enter_modifiers: &[EnterModifier::Tapped],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Black, Color::Green]),
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana_choice(&[
                ManaColor::Blue,
                ManaColor::Black,
                ManaColor::Green,
            ])]),
        activated!(Cost {
                mana: baylee_core::mana!("{2}"),
                parts: &[CostPart::DiscardSelf],
            }, &[Effect::DrawCards {
                amount: Amount::Fixed(1),
            }], zone: ActivationZone::Hand),
    ],
}
