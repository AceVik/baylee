//! Counter replacement decisions stay inside the instruction that caused them.
use super::*;
use crate::choice::YesNoPrompt;
use baylee_cards_dsl::{
    Amount, CostPart, CounterKind, Duration, Filter, Modifier, ReplacementRule,
};

const ADDER: u32 = 7930;
const DOUBLER: u32 = 7931;
const PAYER: u32 = 7932;

static INSTALL: &[AbilityDef] = &[
    free(
        &[Effect::continuous(
            &Filter::ControlledByYou,
            Modifier::AdditionalCounterPlacement { n: 1 },
            Duration::UntilEndOfTurn,
        )],
        None,
    ),
    free(
        &[Effect::continuous(
            &Filter::ControlledByYou,
            Modifier::AdditionalCounterPlacement { n: 2 },
            Duration::UntilEndOfTurn,
        )],
        None,
    ),
];
static DOUBLE: &[AbilityDef] = &[AbilityDef::Replacement(
    ReplacementRule::DoubleCounterPlacement {
        object_filter: &Filter::ControlledByYou,
    },
)];
static PAY: &[AbilityDef] = &[
    paid(
        Cost {
            mana: ManaCost::ZERO,
            parts: &[
                CostPart::PutCounterSelf {
                    kind: CounterKind::P1P1,
                    n: 1,
                },
                CostPart::PayLife(2),
                CostPart::Sacrifice(&Filter::This),
            ],
        },
        &[Effect::GainLife {
            amount: Amount::SacrificedPower,
        }],
        None,
    ),
    free(
        &[
            Effect::AddCounterSource {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            },
            Effect::SacrificeSelf,
        ],
        None,
    ),
    paid(
        Cost {
            mana: ManaCost::ZERO,
            parts: &[
                CostPart::PayLife(2),
                CostPart::RemoveCounterSelf {
                    kind: CounterKind::P1P1,
                    n: 7,
                },
                CostPart::PutCounterSelf {
                    kind: CounterKind::P1P1,
                    n: 1,
                },
                CostPart::RemoveCounterSelf {
                    kind: CounterKind::P1P1,
                    n: 7,
                },
            ],
        },
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
        None,
    ),
];

fn setup(additions: u32) -> (Bench, ObjectId) {
    let mut e = bench(
        7930,
        vec![
            card(
                ADDER,
                face("Counter adder", "{0}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                INSTALL,
            ),
            card(
                DOUBLER,
                face("Counter doubler", "{0}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                DOUBLE,
            ),
            card(
                PAYER,
                creature_face("Counter payer", "{0}", 1, 1),
                KeywordSet::EMPTY,
                PAY,
            ),
        ],
        [Seat::with(&[ADDER, DOUBLER, PAYER]), Seat::default()],
    );
    to_main(&mut e, me());
    let adder = the(&e, ZoneLocation::Battlefield, ADDER);
    for ability in 0..additions {
        activate(&mut e, me(), adder, ability);
        settle(&mut e, me());
    }
    let payer = the(&e, ZoneLocation::Battlefield, PAYER);
    (e, payer)
}

fn ordering(e: &Bench, source: ObjectId) {
    assert!(
        matches!(e.pending(), Pending::YesNo { player, prompt: YesNoPrompt::ApplyAdditionalCountersNext { permanent, .. }, .. } if *player == me() && *permanent == source)
    );
    assert_eq!(
        e.state()
            .object(source)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );
    assert_eq!(e.state().object(source).unwrap().zone, Zone::Battlefield);
}

#[test]
fn counter_payment_order_precedes_later_life_and_sacrifice_costs() {
    for (add_second_first, expected) in [(true, 8), (false, 6)] {
        let (mut e, payer) = setup(2);
        let before = life(&e, me());
        activate(&mut e, me(), payer, 0);
        e.apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![payer],
            },
        )
        .unwrap();
        ordering(&e, payer);
        assert_eq!(life(&e, me()), before);
        assert!(e.state().zones.stack_is_empty());
        let hash = e.snapshot_hash();
        assert!(e.apply(them(), PlayerAction::YesNo(true)).is_err());
        assert_eq!(
            e.snapshot_hash(),
            hash,
            "wrong player cannot advance payment"
        );
        e.apply(me(), PlayerAction::YesNo(true)).unwrap();
        ordering(&e, payer);
        assert_eq!(life(&e, me()), before);
        e.apply(me(), PlayerAction::YesNo(add_second_first))
            .unwrap();
        assert_eq!(e.state().object(payer).unwrap().zone, Zone::Graveyard);
        assert_eq!(life(&e, me()), before - 2);
        settle(&mut e, me());
        assert_eq!(
            life(&e, me()),
            before - 2 + 1 + expected,
            "sacrificed power reads the ordered counters"
        );
    }
}

#[test]
fn counter_payment_effect_only_doubling_needs_a_replacement_to_enable_it() {
    for (additions, expected) in [(0, 1), (1, 4)] {
        let (mut e, payer) = setup(additions);
        let before = life(&e, me());
        activate(&mut e, me(), payer, 0);
        e.apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![payer],
            },
        )
        .unwrap();
        assert!(!matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::ApplyAdditionalCountersNext { .. },
                ..
            }
        ));
        settle(&mut e, me());
        assert_eq!(life(&e, me()), before - 2 + 1 + expected);
    }
}

#[test]
fn counter_effect_order_precedes_following_sacrifice_instruction() {
    for (addition_first, expected) in [(true, 4), (false, 3)] {
        let (mut e, payer) = setup(1);
        activate(&mut e, me(), payer, 1);
        walk_until(&mut e, |game| {
            matches!(
                game.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::ApplyAdditionalCountersNext { .. },
                    ..
                }
            )
        });
        ordering(&e, payer);
        e.apply(me(), PlayerAction::YesNo(addition_first)).unwrap();
        settle(&mut e, me());
        assert_eq!(e.state().object(payer).unwrap().zone, Zone::Graveyard);
        assert!(e.journal().entries().iter().any(|entry| matches!(entry.event, crate::event::GameEvent::CounterChanged { object, old: 0, new, .. } if object == payer && new == expected)));
    }
}

#[test]
fn counter_payment_legal_order_that_cannot_finish_reverses_the_original_attempt() {
    for finishable in [false, true] {
        let (mut e, payer) = setup(2);
        // Establish the counter resource before starting the payment under test.
        let state = e.dev_state_mut(me()).unwrap();
        state
            .object_mut(payer)
            .unwrap()
            .counters
            .set(CounterKind::P1P1, 7);
        state.invalidate_projections();
        state.refresh_characteristics();
        e.refresh_offer();
        let before = life(&e, me());
        let original = e.state().snapshot_hash();
        activate(&mut e, me(), payer, 2);
        ordering(&e, payer);
        assert_eq!(life(&e, me()), before - 2);
        let pending = e.snapshot_hash();
        assert!(e.apply(me(), PlayerAction::ChooseNumber(0)).is_err());
        assert_eq!(
            e.snapshot_hash(),
            pending,
            "invalid answer preserves the payment prompt"
        );
        e.apply(me(), PlayerAction::YesNo(true)).unwrap();
        e.apply(me(), PlayerAction::YesNo(finishable)).unwrap();
        if finishable {
            settle(&mut e, me());
            assert_eq!(life(&e, me()), before - 1);
            assert_eq!(
                e.state()
                    .object(payer)
                    .unwrap()
                    .counters
                    .get(CounterKind::P1P1),
                1
            );
        } else {
            assert!(matches!(e.pending(), Pending::Priority { player, .. } if *player == me()));
            assert!(e.state().zones.stack_is_empty());
            assert_eq!(life(&e, me()), before);
            assert_eq!(
                e.state()
                    .object(payer)
                    .unwrap()
                    .counters
                    .get(CounterKind::P1P1),
                7
            );
            assert_eq!(
                e.state().snapshot_hash(),
                original,
                "all payment writes are reversed"
            );
            assert!(e.activation_payment.is_none());
        }
    }
}
