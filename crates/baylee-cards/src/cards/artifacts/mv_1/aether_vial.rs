//! Aether Vial — {1} — Artifact
//! Oracle: At the beginning of your upkeep, you may put a charge counter on this artifact.
//! Oracle: {T}: You may put a creature card with mana value equal to the number of charge counters on this artifact from your hand onto the battlefield.
//! Set: 2X2 #298 — Double Masters 2022 | Scryfall ID: 11e8d2fd-b132-4807-9410-8edeffa519ed | Oracle ID: fc148e1e-dff0-448e-9f16-625341754356
// IMPLEMENTED — the optional upkeep counter, and a {T} ability putting a
// creature card from hand onto the battlefield whose mana value equals the
// charge counters (`Amount::CountersOnSource`) as the ability resolves.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AETHER_VIAL,
    oracle_id = "fc148e1e-dff0-448e-9f16-625341754356",
    scryfall_id = "11e8d2fd-b132-4807-9410-8edeffa519ed",
    faces = &[face!(
        name = "Aether Vial",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::MayDo {
                effects: &[Effect::AddCounter {
                    kind: CounterKind::Charge,
                    amount: Amount::Fixed(1),
                }],
            }]
        ),
        activated!(
            Cost::TAP,
            &[Effect::PutFromHandOntoBattlefield {
                filter: &Filter::CREATURE,
                mana_value: Some(ManaValueBound {
                    cmp: ManaValueCmp::Exactly,
                    amount: Amount::CountersOnSource(CounterKind::Charge),
                }),
                optional: true,
            }]
        ),
    ],
);
