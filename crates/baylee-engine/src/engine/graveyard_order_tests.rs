//! Graveyard ordering through the public activation and choice flow.
use super::synthetic::{SyntheticLookup, land, preset, walk_past};
use super::*;
use baylee_cards_dsl::prelude::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const SOURCE: u32 = 4_000_131;
const VICTIM: u32 = 4_000_132;
static ABILITIES: &[AbilityDef] = &[
    activated!(
        cost!("", Sacrifice(&Filter::Any), Sacrifice(&Filter::Any)),
        &[Effect::GainLife {
            amount: Amount::Fixed(1)
        }]
    ),
    triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[],
        condition = Some(Condition::GraveyardCardsAbove(&Filter::Any, 2))
    ),
];

fn fixture() -> (Engine<SyntheticLookup>, ObjectId, Vec<ObjectId>) {
    let lookup = SyntheticLookup::new(vec![
        land(SOURCE, "Sacrificer", ABILITIES),
        land(VICTIM, "Borrowed", &[]),
    ]);
    let mut engine = Engine::new(&preset(33, &[SOURCE, VICTIM, VICTIM]), lookup).unwrap();
    let source = engine.state.zones.list(ZoneLocation::Battlefield)[0];
    let victims = engine.state.zones.list(ZoneLocation::Battlefield)[1..].to_vec();
    // A controller can sacrifice a borrowed permanent, but its owner orders
    // the cards arriving in their graveyard (CR 404.3).
    for &id in &victims {
        engine.state.object_mut(id).unwrap().owner = P1;
    }
    for _ in 0..100 {
        if matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0)
            && engine.state.turn.step == crate::turn::Step::Main
        {
            return (engine, source, victims);
        }
        let question = engine.pending().clone();
        if let Pending::Mulligan { player, .. } = question {
            engine.apply(player, PlayerAction::MulliganKeep).unwrap();
        } else {
            assert!(walk_past(&mut engine, &question), "{question:?}");
        }
    }
    panic!("never reached priority");
}

#[test]
fn graveyard_multi_sacrifice_cost_asks_the_owner_not_the_activator() {
    let (mut engine, source, victims) = fixture();
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .unwrap();
    for &card in &victims {
        assert!(matches!(engine.pending(), Pending::ChooseCards { player, .. } if *player == P0));
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![card],
                },
            )
            .unwrap();
    }
    assert!(matches!(engine.pending(), Pending::Arrange { player, .. } if *player == P1));
    let pending_hash = engine.snapshot_hash();
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::Arrange {
                    piles: vec![vec![victims[0], victims[0]]]
                }
            )
            .is_err()
    );
    assert_eq!(
        engine.snapshot_hash(),
        pending_hash,
        "a bad order changes nothing"
    );
    engine
        .apply(
            P1,
            PlayerAction::Arrange {
                piles: vec![victims.clone()],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Graveyard(P1)),
        &[victims[1], victims[0]]
    );
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Stack).len(),
        1,
        "the activated ability still awaits resolution"
    );
}

#[test]
fn graveyard_failed_payment_never_leaves_an_ordering_choice() {
    let (mut engine, source, victims) = fixture();
    let before = engine.snapshot_hash();
    let cost = cost!(
        "",
        Sacrifice(&Filter::Any),
        Sacrifice(&Filter::Any),
        Sacrifice(&Filter::Any)
    );
    assert!(engine.pay_cost(P0, source, &cost, &victims, 0).is_err());
    assert_eq!(engine.snapshot_hash(), before);
    assert!(engine.state.graveyard_order.batches.is_empty());
}

#[test]
fn graveyard_independent_discard_and_sacrifice_costs_are_not_one_batch() {
    let (mut engine, source, _) = fixture();
    engine.state.draw_cards(P0, 1);
    let discarded = *engine
        .state
        .zones
        .list(ZoneLocation::Hand(P0))
        .last()
        .unwrap();
    let cost = cost!("", Discard(&Filter::Any), SacrificeSelf);
    engine.pay_cost(P0, source, &cost, &[discarded], 0).unwrap();
    assert!(crate::graveyard_order::pending(&mut engine.state).is_none());
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Graveyard(P0)),
        &[discarded, source]
    );
}

#[test]
fn graveyard_an_intervening_payment_ends_a_sacrifice_group() {
    let (mut engine, source, victims) = fixture();
    let cost = cost!(
        "",
        Sacrifice(&Filter::Any),
        PayLife(1),
        Sacrifice(&Filter::Any)
    );
    engine.pay_cost(P0, source, &cost, &victims, 0).unwrap();
    assert!(crate::graveyard_order::pending(&mut engine.state).is_none());
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Graveyard(P1)),
        &victims
    );
}
