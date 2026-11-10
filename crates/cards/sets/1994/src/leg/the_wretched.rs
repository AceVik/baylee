//! The Wretched — {3}{B}{B} — Creature — Demon
//! Oracle: At end of combat, gain control of all creatures blocking this creature for as long as you control this creature.
//! Set: ME3 #78 — Masters Edition III | Scryfall ID: edf30a58-f77c-4887-a0b2-9d939243d86e | Oracle ID: 27af77e5-6680-43d3-9202-40be9b5d14a0

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THE_WRETCHED,
    oracle_id = "27af77e5-6680-43d3-9202-40be9b5d14a0",
    scryfall_id = "edf30a58-f77c-4887-a0b2-9d939243d86e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::CombatEnd,
            whose: PlayerRel::EachPlayer
        },
        &[Effect::continuous(
            &Filter::BlockingSource,
            Modifier::GainControl,
            Duration::WhileYouControlSource
        ),]
    )],
    faces = &[face!(
        name = "The Wretched",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DEMON],
        power = Some(2),
        toughness = Some(5),
    ),],
);
