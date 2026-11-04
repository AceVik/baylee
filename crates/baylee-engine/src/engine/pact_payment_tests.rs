//! Report 2026-09-28: an empty pool must not silently lose to a pact.
//!
//! The debt is the delayed triggered ability a pact leaves (CR 603.7), put on
//! the stack by hand here because the question is the subject and not a card:
//! its controller is asked as it resolves. `card_tests` plays the real one.
use super::testkit::*;
use super::*;
use baylee_cards_dsl::Effect;
use baylee_core::mana::{ManaColor, ManaCost};

static OWES_BLACK: &[Effect] = &[Effect::PayManaOrLose {
    cost: ManaCost::parse("{B}"),
}];
static OWES_BLUE: &[Effect] = &[Effect::PayManaOrLose {
    cost: ManaCost::parse("{U}"),
}];
static OWES_NEGATION: &[Effect] = &[Effect::PayManaOrLose {
    cost: ManaCost::parse("{3}{U}{U}"),
}];

fn asked(owed: &'static [Effect], board: &[baylee_core::ids::CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::table(928, basic_forest(), 3)
        .battlefield(0, board)
        .start();
    keep_mulligans(&mut engine);
    let me = PlayerId::new(0);
    let source = engine.state.zones.list(ZoneLocation::Library(me))[0];
    engine.delayed_queue.push_back((
        me,
        crate::state::DelayedAction::Trigger {
            source,
            source_version: engine.state.object(source).map_or(0, |o| o.version),
            effects: owed,
            text: crate::text_changes::TextChangeMap::IDENTITY,
        },
    ));
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
    let mut engine = asked(OWES_BLACK, &[badlands]);
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
        Some((
            me,
            baylee_core::mana::ManaPayment::Fixed(ManaCost::parse("{B}"))
        ))
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
    let mut engine = asked(OWES_NEGATION, &[forest; 5]);
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

/// Nothing on the board makes mana, and the loss is still theirs to answer:
/// asked, and a yes with nothing to press is the no it has to be.
#[test]
fn pact_empty_board_still_warns() {
    let me = PlayerId::new(0);
    let mut engine = asked(OWES_NEGATION, &[]);
    assert!(!engine.state().players[0].has_lost());
    engine.apply(me, PlayerAction::YesNo(true)).unwrap();
    assert!(engine.payment_window().is_none());
    assert_eq!(
        engine.state().players[0].loss,
        Some(crate::event::LossReason::Effect)
    );
    assert_ne!(engine.pending().asked(), Some(me));
}

#[test]
fn a_payer_conceding_in_the_window_does_not_stall_multiplayer() {
    let me = PlayerId::new(0);
    let mut engine = asked(OWES_NEGATION, &[basic_forest()]);
    engine.apply(me, PlayerAction::YesNo(true)).unwrap();
    assert!(engine.payment_window().is_some());
    engine.apply(me, PlayerAction::Concede).unwrap();
    assert!(engine.payment_window().is_none());
    assert_ne!(engine.pending().asked(), Some(me));
}

#[test]
fn pact_cost_color_is_part_of_the_snapshot() {
    let me = PlayerId::new(0);
    let mut black = asked(OWES_BLACK, &[basic_forest()]);
    let mut blue = asked(OWES_BLUE, &[basic_forest()]);
    for engine in [&mut black, &mut blue] {
        engine.apply(me, PlayerAction::YesNo(true)).unwrap();
    }
    assert_eq!(black.state().snapshot_hash(), blue.state().snapshot_hash());
    assert_ne!(black.snapshot_hash(), blue.snapshot_hash());
}
