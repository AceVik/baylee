//! Armageddon Clock — {6} — Artifact
//! Oracle: At the beginning of your upkeep, put a doom counter on this artifact.
//! Oracle: At the beginning of your draw step, this artifact deals damage equal to the number of doom counters on it to each player.
//! Oracle: {4}: Remove a doom counter from this artifact. Any player may activate this ability but only during any upkeep step.
//! Set: ME4 #180 — Masters Edition IV | Scryfall ID: ab55bb84-03c2-4989-8db4-0d5578ea0431 | Oracle ID: 70d90ef4-0cda-405f-abf1-734fa909efa6
use baylee_cards_dsl::prelude::*;

card!(
    index = index::ARMAGEDDON_CLOCK,
    oracle_id = "70d90ef4-0cda-405f-abf1-734fa909efa6",
    scryfall_id = "ab55bb84-03c2-4989-8db4-0d5578ea0431",
    faces = &[face!(
        name = "Armageddon Clock",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::AddCounter {
                kind: counters::DOOM,
                amount: Amount::Fixed(1)
            }]
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Draw,
                whose: PlayerRel::You
            },
            &[Effect::DealDamage {
                amount: Amount::CountersOnSource(counters::DOOM),
                target: TargetSpec::Player(PlayerRel::EachPlayer)
            }]
        ),
        activated!(
            cost!("{4}"),
            &[Effect::RemoveCounterSelf {
                kind: counters::DOOM,
                n: 1
            }],
            condition = Some(Condition::DuringStep(StepKind::Upkeep)),
            zone = ActivationZone::BattlefieldAnyPlayer
        ),
    ],
);
