//! Diseased Vermin — {2}{B} — Creature — Rat
//! Oracle: Whenever this creature deals combat damage to a player, put an infection counter on it.
//! Oracle: At the beginning of your upkeep, this creature deals X damage to target opponent previously dealt damage by it, where X is the number of infection counters on it.
//! Set: ALL #46 — Alliances | Scryfall ID: 39703080-524d-4aa1-8c58-d512c41ae5d4 | Oracle ID: 4d18bea4-c9b2-4fc7-948f-2583589994d7
// NOT SUPPORTED: restricting the upkeep damage target to an opponent previously dealt damage by this creature is not supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DISEASED_VERMIN,
    oracle_id = "4d18bea4-c9b2-4fc7-948f-2583589994d7",
    scryfall_id = "39703080-524d-4aa1-8c58-d512c41ae5d4",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "puts an infection counter on itself whenever it deals combat damage to a player, and at the beginning of your upkeep deals X damage to target opponent, where X is the number of infection counters on it; restricting the target to an opponent previously dealt damage by it is not supported",
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
        ),
    ],
);
