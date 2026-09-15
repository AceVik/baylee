//! Raffine's Tower — (no cost) — Land — Plains Island Swamp
//! Oracle: ({T}: Add {W}, {U}, or {B}.)
//! Oracle: This land enters tapped.
//! Oracle: Cycling {3} ({3}, Discard this card: Draw a card.)
//! Set: SNC #254 — Streets of New Capenna | Scryfall ID: a2c56479-4bee-4edb-80d7-4af010b7c793 | Oracle ID: 6e9ef5ef-6aed-4d3e-a59b-9e3dc8740b1b
// IMPLEMENTED — triome (3 land types → intrinsic mana, ETB tapped, cycling {3}).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = 23338,
    oracle_id = "6e9ef5ef-6aed-4d3e-a59b-9e3dc8740b1b",
    scryfall_id = "a2c56479-4bee-4edb-80d7-4af010b7c793",
    faces = &[face!(
        name = "Raffine's Tower",
        types = TypeSet::LAND,
        subtypes = &[
            subtypes::land::PLAINS,
            subtypes::land::ISLAND,
            subtypes::land::SWAMP,
        ],
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    color_identity = ColorSet::from_slice(&[Color::White, Color::Blue, Color::Black]),
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
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
