//! Report 2026-09-28: an empty pool must not silently lose to a pact.
use super::testkit::*;
use super::*;
use baylee_core::mana::{ManaColor, ManaCost};

fn asked(cost: ManaCost, board: &[baylee_core::ids::CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::table(928, basic_forest(), 3)
        .battlefield(0, board)
        .start();
    keep_mulligans(&mut engine);
    engine
        .upkeep_payments
        .push_back(crate::state::DelayedAction::PayCostOrLose { cost });
    for _ in 0..12 {
        if matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::PayPact { .. },
                ..
            }
        ) {
            return engine;
        }
        let (player, action) = answer_one(&engine).unwrap();
        engine.apply(player, action).unwrap();
    }
    panic!("pact must warn before a loss: {:?}", engine.pending());
}

#[test]
fn pact_colored_payment_survives_a_nested_mana_color_question() {
    let me = PlayerId::new(0);
    let badlands = card_index("13ff3222-91cb-4796-a34e-899ed817694c");
    let cost = ManaCost::parse("{B}");
    let mut engine = asked(cost, &[badlands]);
    engine.apply(me, PlayerAction::YesNo(true)).unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("mana window")
    };
    assert!(legal.castable.is_empty() && legal.lands.is_empty());
    let (source, ability_index) = legal.abilities[0];
    engine
        .apply(
            me,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    assert!(matches!(engine.pending(), Pending::ChooseColor { .. }));
    assert_eq!(
        engine.payment_window(),
        Some((me, baylee_core::mana::ManaPayment::Fixed(cost)))
    );
    engine
        .apply(me, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
    engine.apply(me, PlayerAction::PassPriority).unwrap();
    assert!(!engine.state().players[0].has_lost());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(engine.payment_window().is_none());
}

#[test]
fn pact_wrong_color_cannot_pay_and_does_not_charge_a_partial_payment() {
    let me = PlayerId::new(0);
    let forest = basic_forest();
    let mut engine = asked(ManaCost::parse("{3}{U}{U}"), &[forest; 5]);
    engine.apply(me, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, me);
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);
    engine.apply(me, PlayerAction::PassPriority).unwrap();
    assert_eq!(
        engine.state().players[0].loss,
        Some(crate::event::LossReason::Effect)
    );
    assert!(engine.payment_window().is_none());
    assert_ne!(engine.pending().asked(), Some(me));
}

#[test]
fn pact_empty_board_still_warns_and_a_conceding_payer_does_not_stall_multiplayer() {
    let me = PlayerId::new(0);
    let mut engine = asked(ManaCost::parse("{3}{U}{U}"), &[]);
    assert!(!engine.state().players[0].has_lost());
    engine.apply(me, PlayerAction::YesNo(true)).unwrap();
    engine.apply(me, PlayerAction::Concede).unwrap();
    assert!(engine.payment_window().is_none());
    assert_ne!(engine.pending().asked(), Some(me));
}

#[test]
fn pact_cost_color_is_part_of_the_snapshot() {
    let me = PlayerId::new(0);
    let mut black = asked(ManaCost::parse("{B}"), &[]);
    let mut blue = asked(ManaCost::parse("{U}"), &[]);
    for engine in [&mut black, &mut blue] {
        engine.apply(me, PlayerAction::YesNo(true)).unwrap();
    }
    assert_eq!(black.state().snapshot_hash(), blue.state().snapshot_hash());
    assert_ne!(black.snapshot_hash(), blue.snapshot_hash());
}
