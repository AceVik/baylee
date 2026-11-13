//! Angus Mackenzie — {G}{W}{U} — Legendary Creature — Human Cleric
//! Oracle: {G}{W}{U}, {T}: Prevent all combat damage that would be dealt this turn. Activate only before the combat damage step.
//! Set: ME3 #141 — Masters Edition III | Scryfall ID: bb3920ac-7d61-494e-b7dd-bd37d4dee143 | Oracle ID: d5e6ac6b-5786-4d98-8037-0867e365fc93
// IMPLEMENTED — {G}{W}{U}, {T} prevents all combat damage this turn, only before the combat damage step.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ANGUS_MACKENZIE,
    oracle_id = "d5e6ac6b-5786-4d98-8037-0867e365fc93",
    scryfall_id = "bb3920ac-7d61-494e-b7dd-bd37d4dee143",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Angus Mackenzie",
        mana_cost = mana!("{G}{W}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{G}{W}{U}", TapSelf),
        &[Effect::PreventAllCombatDamageThisTurn],
        condition = Some(Condition::BeforeStep(StepKind::CombatDamage)),
    )],
);
