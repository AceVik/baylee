//! Indatha Triome — (no cost) — Land — Plains Swamp Forest
//! Oracle: ({T}: Add {W}, {B}, or {G}.)
//! Oracle: This land enters tapped.
//! Oracle: Cycling {3} ({3}, Discard this card: Draw a card.)
//! Set: IKO #248 — Ikoria: Lair of Behemoths | Scryfall ID: 2b74bb81-fb9a-40e5-a941-e517430b52f5 | Oracle ID: ec2b3779-55f7-4169-aa66-6312fb52721f
// IMPLEMENTED — triome (3 land types → intrinsic mana, ETB tapped, cycling {3}).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{self};

card!(
    index = 19990,
    oracle_id = "ec2b3779-55f7-4169-aa66-6312fb52721f",
    scryfall_id = "2b74bb81-fb9a-40e5-a941-e517430b52f5",
    faces = &[face!(
        name = "Indatha Triome",
        types = TypeSet::LAND,
        subtypes = &[
            subtypes::land::PLAINS,
            subtypes::land::SWAMP,
            subtypes::land::FOREST,
        ],
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    color_identity = ColorSet::from_slice(&[Color::White, Color::Black, Color::Green]),
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[
            ManaColor::White,
            ManaColor::Black,
            ManaColor::Green,
        ])]),
        // Cycling {3} (hand-zone ability: discard to draw).
        activated!(
            cost!("{3}", DiscardSelf),
            &[Effect::DrawCards {
                amount: Amount::Fixed(1),
            }],
            zone = ActivationZone::Hand
        ),
    ],
);
