//! Soldevi Golem — {4} — Artifact Creature — Golem
//! Oracle: This creature doesn't untap during your untap step.
//! Oracle: At the beginning of your upkeep, you may untap target tapped creature an opponent controls. If you do, untap this creature.
//! Set: ME4 #228 — Masters Edition IV | Scryfall ID: 5d51269c-5b79-4833-aed4-2934d5e15e1a | Oracle ID: cd444267-712c-450f-a349-e27faf4f4c7c

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static TAPPED_OPPONENT_CREATURE: Filter = Filter::And(&[Filter::OPPONENT_CREATURE, Filter::Tapped]);

card!(
    index = index::SOLDEVI_GOLEM,
    oracle_id = "cd444267-712c-450f-a349-e27faf4f4c7c",
    scryfall_id = "5d51269c-5b79-4833-aed4-2934d5e15e1a",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Soldevi Golem",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GOLEM],
        power = Some(5),
        toughness = Some(3),
    ),],
    abilities = &[
        // CR 502.3: Doesn't untap during your untap step.
        static_ability!(Filter::This, Modifier::DoesNotUntap),
        // CR 603.1: At the beginning of your upkeep, you may untap target tapped creature an opponent controls. If you do, untap this creature.
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::MayDo {
                effects: &[Effect::UntapTarget, Effect::UntapSelf],
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(
                &TAPPED_OPPONENT_CREATURE
            ))),
        ),
    ],
);
