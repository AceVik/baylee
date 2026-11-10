//! Moonring Island — (no cost) — Land — Island
//! Oracle: ({T}: Add {U}.)
//! Oracle: This land enters tapped.
//! Oracle: {U}, {T}: Look at the top card of target player's library. Activate only if you control two or more blue permanents.
//! Set: SHM #276 — Shadowmoor | Scryfall ID: 64b36993-666c-40d0-b61a-1d162bd06dcc | Oracle ID: cf620c66-7db1-4db8-ae56-ee4bc2f77d74

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MOONRING_ISLAND,
    oracle_id = "cf620c66-7db1-4db8-ae56-ee4bc2f77d74",
    scryfall_id = "64b36993-666c-40d0-b61a-1d162bd06dcc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Moonring Island",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::ISLAND],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::intrinsic_mana()]),
        activated!(
            cost!("{U}", TapSelf),
            &[Effect::LookAtTopOf {
                player: PlayerRel::Chosen,
                count: 1
            }],
            target = Some(TargetSpec::AnyPlayer),
            condition = Some(Condition::ControlCount(
                &Filter::And(&[
                    Filter::ControlledByYou,
                    Filter::HasColor(ColorSet::from_slice(&[Color::Blue])),
                ]),
                2
            )),
        ),
    ],
);
