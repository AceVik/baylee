//! Raugrin Triome — (no cost) — Land — Island Mountain Plains
//! Oracle: ({T}: Add {U}, {R}, or {W}.)
//! Oracle: This land enters tapped.
//! Oracle: Cycling {3} ({3}, Discard this card: Draw a card.)
//! Set: IKO #251 — Ikoria: Lair of Behemoths | Scryfall ID: 02138fbb-3962-4348-8d31-faaefba0b8b2 | Oracle ID: c7fa1dda-9312-4ec8-82cd-a1ba7bc33497
// IMPLEMENTED — triome (3 land types → intrinsic mana, ETB tapped, cycling {3}).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{self};

card!(
    index = index::RAUGRIN_TRIOME,
    oracle_id = "c7fa1dda-9312-4ec8-82cd-a1ba7bc33497",
    scryfall_id = "02138fbb-3962-4348-8d31-faaefba0b8b2",
    faces = &[face!(
        name = "Raugrin Triome",
        types = TypeSet::LAND,
        subtypes = &[
            subtypes::land::ISLAND,
            subtypes::land::MOUNTAIN,
            subtypes::land::PLAINS,
        ],
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::Red, Color::White]),
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[
            ManaColor::Blue,
            ManaColor::Red,
            ManaColor::White,
        ])]),
        activated!(
            cost!("{3}", DiscardSelf),
            &[Effect::DrawCards {
                amount: Amount::Fixed(1),
            }],
            zone = ActivationZone::Hand
        ),
    ],
);
