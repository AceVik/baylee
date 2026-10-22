//! Minion of the Wastes — {3}{B}{B}{B} — Creature — Minion
//! Oracle: Trample
//! Oracle: As this creature enters, pay any amount of life.
//! Oracle: Minion of the Wastes's power and toughness are each equal to the life paid as it entered.
//! Set: TMP #146 — Tempest | Scryfall ID: d9f120fc-c681-47b6-827e-1cc7ead47a0f | Oracle ID: 2593944c-1bde-4307-b3ca-f7ecbeb027c1
// PARTIAL: has trample, chooses a number as it enters, and sets its power and toughness equal to the chosen number; paying that much life as it enters is not in EnterModifier.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MINION_OF_THE_WASTES,
    oracle_id = "2593944c-1bde-4307-b3ca-f7ecbeb027c1",
    scryfall_id = "d9f120fc-c681-47b6-827e-1cc7ead47a0f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "has trample, chooses a number as it enters, and sets its power and toughness equal to the chosen number; paying that much life as it enters is not in EnterModifier",
    ),
    faces = &[face!(
        name = "Minion of the Wastes",
        mana_cost = mana!("{3}{B}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MINION],
        power = Some(0),
        toughness = Some(0),
        keywords = KeywordSet::TRAMPLE,
        enter_modifiers = &[EnterModifier::ChooseNumber { min: 1, max: 20 }],
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CharacteristicPT {
            count: PtCount::ChosenNumber,
            toughness_plus: 0,
        },
    )],
);
