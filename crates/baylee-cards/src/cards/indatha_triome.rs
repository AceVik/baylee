//! Indatha Triome — (no cost) — Land — PLAINS SWAMP FOREST
//! Oracle: ({T}: Add {W}, {B}, or {G}.)
//! Indatha Triome enters the battlefield tapped.
//! Cycling {2}
//! Set: IKO #248 — Ikoria: Lair of Behemoths | Scryfall ID: 2b74bb81-fb9a-40e5-a941-e517430b52f5 | Oracle ID: ec2b3779-55f7-4169-aa66-6312fb52721f
// IMPLEMENTED — triome (3 land types → intrinsic mana, ETB tapped, cycling {2}).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{self};

card! {
    index: 72,
    oracle_id: "ec2b3779-55f7-4169-aa66-6312fb52721f",
    scryfall_id: "2b74bb81-fb9a-40e5-a941-e517430b52f5",
    faces: &[face! {
        name: "Indatha Triome",
        types: TypeSet::LAND,
        subtypes: &[
            subtypes::land::PLAINS,
            subtypes::land::SWAMP,
            subtypes::land::FOREST,
        ],
        enter_modifiers: &[EnterModifier::Tapped],
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Black, Color::Green]),
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana_choice(&[
                ManaColor::White,
                ManaColor::Black,
                ManaColor::Green,
            ])]),
        // Cycling {2} (hand-zone ability: discard to draw).
        activated!(Cost {
                mana: baylee_core::mana!("{2}"),
                parts: &[CostPart::DiscardSelf],
            }, &[Effect::DrawCards {
                amount: Amount::Fixed(1),
            }], zone: ActivationZone::Hand),
    ],
}
