//! Urborg Stalker — {3}{B} — Creature — Horror
//! Oracle: At the beginning of each player's upkeep, if that player controls a nonblack, nonland permanent, this creature deals 1 damage to that player.
//! Set: WTH #85 — Weatherlight | Scryfall ID: 2d33e3d5-c608-4ba8-8614-0b9d0385af64 | Oracle ID: e5ef45e4-8c19-40bf-8756-c1b408dcb2fb
// IMPLEMENTED — At the beginning of each player's upkeep, if that player controls a nonblack, nonland permanent, deals 1 damage to that player.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::URBORG_STALKER,
    oracle_id = "e5ef45e4-8c19-40bf-8756-c1b408dcb2fb",
    scryfall_id = "2d33e3d5-c608-4ba8-8614-0b9d0385af64",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Urborg Stalker",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HORROR],
        power = Some(2),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Player(PlayerRel::ActivePlayer),
        }],
        condition = Some(Condition::BattlefieldCount(
            &Filter::And(&[
                Filter::ControlledByActivePlayer,
                Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
                Filter::Not(&Filter::LAND),
            ]),
            1
        ))
    ),],
);
