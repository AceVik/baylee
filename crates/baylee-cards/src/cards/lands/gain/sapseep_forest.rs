//! Sapseep Forest — (no cost) — Land — Forest
//! Oracle: ({T}: Add {G}.)
//! Oracle: This land enters tapped.
//! Oracle: {G}, {T}: You gain 1 life. Activate only if you control two or more green permanents.
//! Set: C21 #313 — Commander 2021 | Scryfall ID: 81d3099d-4f22-425c-8955-903b6cfb88d3 | Oracle ID: 8d4dcab0-86e5-4ff8-a90f-78a062664e16
// IMPLEMENTED — enters tapped, taps for {G}, and {G}, {T} gains 1 life while
// you control two or more green permanents (Condition::ControlCount).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// A green permanent — what the printed activation condition counts.
static GREEN_PERMANENT: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Green]));

card!(
    index = index::SAPSEEP_FOREST,
    oracle_id = "8d4dcab0-86e5-4ff8-a90f-78a062664e16",
    scryfall_id = "81d3099d-4f22-425c-8955-903b6cfb88d3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sapseep Forest",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::FOREST],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        activated!(
            cost!("{G}", TapSelf),
            &[Effect::gain_life(1)],
            condition = Some(Condition::ControlCount(&GREEN_PERMANENT, 2))
        ),
    ],
);
