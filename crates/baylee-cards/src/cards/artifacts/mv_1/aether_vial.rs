//! Aether Vial — {1} — Artifact
//! Oracle: At the beginning of your upkeep, you may put a charge counter on this artifact.
//! Oracle: {T}: You may put a creature card with mana value equal to the number of charge counters on this artifact from your hand onto the battlefield.
//! Set: 2X2 #298 — Double Masters 2022 | Scryfall ID: 11e8d2fd-b132-4807-9410-8edeffa519ed | Oracle ID: fc148e1e-dff0-448e-9f16-625341754356
// PARTIAL — the upkeep trigger is built in full; the {T} ability is dropped,
// see the NOT SUPPORTED line above the ability list.

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
    coverage = Coverage::Partial(
        "the {T} ability is dropped: no Effect moves a card from hand onto the \
         battlefield, and no filter can compare a card's mana value against the \
         charge counters on the source"
    ),
    // NOT SUPPORTED: "{T}: You may put a creature card with mana value equal to
    // the number of charge counters on this artifact from your hand onto the
    // battlefield." — there is no Effect whose destination is the battlefield
    // from a hand (`PutFromHandOnTop` goes to the library), and "mana value
    // equal to the number of charge counters on this artifact" has no filter
    // (`Filter::CmcAtMost`/`CmcAtLeast` are constants) nor an `Amount`
    // (`Condition::CountersOnSelf` is a condition, not a value a filter reads).
    abilities = &[triggered!(
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
    )],
);
