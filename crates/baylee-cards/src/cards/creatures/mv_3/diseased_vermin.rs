//! Diseased Vermin — {2}{B} — Creature — Rat
//! Oracle: Whenever this creature deals combat damage to a player, put an infection counter on it.
//! Oracle: At the beginning of your upkeep, this creature deals X damage to target opponent previously dealt damage by it, where X is the number of infection counters on it.
//! Set: ALL #46 — Alliances | Scryfall ID: 39703080-524d-4aa1-8c58-d512c41ae5d4 | Oracle ID: 4d18bea4-c9b2-4fc7-948f-2583589994d7

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DISEASED_VERMIN,
    oracle_id = "4d18bea4-c9b2-4fc7-948f-2583589994d7",
    scryfall_id = "39703080-524d-4aa1-8c58-d512c41ae5d4",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "'target opponent previously dealt damage by it' is not in TargetSpec: the upkeep ability targets any opponent and runs only while you have exactly one opponent (then the only opponent is the one it hit), so at a table of three or more the upkeep damage is never dealt; a player who has left the game can leave the lone remaining opponent as a target it never damaged",
    ),
    faces = &[face!(
        name = "Diseased Vermin",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::RAT],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        triggered!(
            Trigger::DealsCombatDamageToPlayer(&Filter::This),
            &[Effect::AddCounter {
                kind: counters::INFECTION,
                amount: Amount::Fixed(1),
            }],
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::DealDamage {
                amount: Amount::CountersOnSource(counters::INFECTION),
                target: TargetSpec::AnyOpponent,
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
            // NOT SUPPORTED: "target opponent previously dealt damage by it". With one opponent that opponent is the only one it can have damaged; with more, any of them could be a target it never hit, so the ability is off whenever there are two or more opponents (2 minus the number of opponents is at least 1 only with one or none).
            condition = Some(Condition::AmountAtLeast(
                Amount::ConstantMinus {
                    constant: 2,
                    subtract: &Amount::Players(PlayerRel::EachOpponent),
                },
                1,
            )),
        ),
    ],
);
