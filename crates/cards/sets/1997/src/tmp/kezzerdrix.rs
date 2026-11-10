//! Kezzerdrix — {2}{B}{B} — Creature — Rabbit Beast
//! Oracle: First strike
//! Oracle: At the beginning of your upkeep, if your opponents control no creatures, this creature deals 4 damage to you.
//! Set: TPR #107 — Tempest Remastered | Scryfall ID: 2744639b-f7fa-435b-9275-026eae1dd0b7 | Oracle ID: 1523671c-0cf4-442c-a5f8-c74ae31016e6
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KEZZERDRIX,
    oracle_id = "1523671c-0cf4-442c-a5f8-c74ae31016e6",
    scryfall_id = "2744639b-f7fa-435b-9275-026eae1dd0b7",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FIRST_STRIKE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Kezzerdrix",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::RABBIT, subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::DealDamage {
            target: TargetSpec::Player(PlayerRel::You),
            amount: Amount::Fixed(4),
        }],
        condition = Some(Condition::OpponentControlCountAtMost(&Filter::CREATURE, 0)),
    )],
);
