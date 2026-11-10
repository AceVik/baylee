//! Yawgmoth Demon — {4}{B}{B} — Creature — Phyrexian Demon
//! Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.)
//! Oracle: First strike (This creature deals combat damage before creatures without first strike.)
//! Oracle: At the beginning of your upkeep, you may sacrifice an artifact. If you don't, tap this creature and it deals 2 damage to you.
//! Set: 9ED #170 — Ninth Edition | Scryfall ID: 696a9fe1-ce63-4638-8793-5187fc726731 | Oracle ID: 6c54fc14-2af8-46e8-a4dc-b2a0a88ef2e1
// IMPLEMENTED — flying, first strike, and the upkeep choice: pay an artifact
// or the Demon taps and deals 2 to its controller.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::YAWGMOTH_DEMON,
    oracle_id = "6c54fc14-2af8-46e8-a4dc-b2a0a88ef2e1",
    scryfall_id = "696a9fe1-ce63-4638-8793-5187fc726731",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FLYING.union(KeywordSet::FIRST_STRIKE),
    faces = &[face!(
        name = "Yawgmoth Demon",
        mana_cost = mana!("{4}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::DEMON],
        power = Some(6),
        toughness = Some(6),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::PlayerMayPayCostOr {
            player: PlayerRel::You,
            cost: &CostPart::Sacrifice(&Filter::ARTIFACT),
            effect: &Effect::Sequence(&[
                Effect::TapSelf,
                Effect::DealDamage {
                    amount: Amount::Fixed(2),
                    target: TargetSpec::Player(PlayerRel::You),
                },
            ]),
        }],
    )],
);
