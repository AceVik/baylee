//! Echo on the stack (CR 702.30a): "at the beginning of your upkeep, if this
//! permanent came under your control since the beginning of your last upkeep,
//! sacrifice it unless you pay [cost]". An intervening `if` (CR 603.4), so it
//! is asked as the upkeep begins and again as the ability resolves; a triggered
//! ability, so it goes on the stack in APNAP order with everything else that
//! triggered there (CR 603.3b) and can be answered. A language model reported
//! a Keldon Champion's echo missing from the upkeep's stack, which it was: it
//! was never on it.
//!
//! The question it asks is `echo_payment_tests`'.

use super::testkit::{
    Duel, RegistryLookup, basic_forest, cast_from_hand, in_graveyard, keep_mulligans,
    on_battlefield, pass_until, reach_main_phase, reach_their_main_phase, stack_is_empty,
    tap_all_mana,
};
use super::*;
use baylee_cards_dsl::AbilityDef;
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaCost, ManaPayment};

const SEED: u64 = 0xec40_0002;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<RegistryLookup>;

fn named(name: &str) -> CardIndex {
    baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"))
}

/// {G}, a 2/2 with echo {G}.
fn jaguar() -> CardIndex {
    named("Pouncing Jaguar")
}

/// Where echo stands in the Jaguar's list.
fn echo_index() -> u32 {
    baylee_cards::by_index(jaguar())
        .and_then(|card| {
            card.abilities
                .iter()
                .position(|ability| matches!(ability, AbilityDef::Echo { .. }))
        })
        .and_then(|index| u32::try_from(index).ok())
        .expect("the Jaguar prints echo")
}

/// What each seat brings beyond seat 0's two Forests and its Jaguar.
#[derive(Default)]
struct Table<'a> {
    my_board: &'a [CardIndex],
    my_hand: &'a [CardIndex],
    their_board: &'a [CardIndex],
    their_hand: &'a [CardIndex],
}

/// Seat 0's first turn, up to the Jaguar having resolved.
fn jaguar_cast(table: &Table<'_>) -> Game {
    let forest = basic_forest();
    let mut hand = vec![jaguar()];
    hand.extend_from_slice(table.my_hand);
    let mut board = vec![forest, forest];
    board.extend_from_slice(table.my_board);
    let mut engine = Duel::new(SEED, forest)
        .hand(0, &hand)
        .battlefield(0, &board)
        .hand(1, table.their_hand)
        .battlefield(1, table.their_board)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, jaguar());
    pass_until(&mut engine, stack_is_empty);
    engine
}

/// To the next upkeep of `seat`.
fn to_upkeep_of(engine: &mut Game, seat: PlayerId) {
    let now = engine.state().turn.number;
    pass_until(engine, |e| {
        let turn = &e.state().turn;
        turn.number > now && turn.active == seat && turn.step == Step::Upkeep
    });
}

/// Seat 0's next upkeep, with `during` run in seat 1's turn on the way.
fn upkeep_after(table: &Table<'_>, during: impl FnOnce(&mut Game)) -> Game {
    let mut engine = jaguar_cast(table);
    pass_until(&mut engine, |e| e.state().turn.active == P1);
    during(&mut engine);
    to_upkeep_of(&mut engine, P0);
    engine
}

fn upkeep(table: &Table<'_>) -> Game {
    upkeep_after(table, |_| {})
}

fn stack(engine: &Game) -> Vec<ObjectId> {
    engine.state().zones.list(ZoneLocation::Stack).clone()
}

fn priority_with(engine: &Game, seat: PlayerId) -> bool {
    matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
}

fn asked_to_pay(engine: &Game, seat: PlayerId) -> bool {
    matches!(
        engine.pending(),
        Pending::YesNo { player, prompt: crate::choice::YesNoPrompt::PayMana { .. }, .. }
            if *player == seat
    )
}

/// Both seats pass, by name, so that a question either was not expecting
/// fails here: `pass_until` would answer an echo no on its way past.
fn both_pass(engine: &mut Game) {
    assert!(priority_with(engine, P0), "{:?}", engine.pending());
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(priority_with(engine, P1), "{:?}", engine.pending());
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
}

#[test]
fn the_echo_is_on_the_stack_and_both_players_hold_priority_over_it() {
    let mut engine = upkeep(&Table::default());
    let jaguar = on_battlefield(&engine, P0, jaguar()).expect("the Jaguar is in play");

    let [echo] = stack(&engine)[..] else {
        panic!("one object on the stack: {:?}", stack(&engine))
    };
    let object = engine.state().object(echo).expect("it is an object");
    assert_eq!(object.kind, ObjectKind::AbilityOnStack);
    assert_eq!(object.controller, P0);
    let location = object.ability.expect("it names an ability");
    assert_eq!(
        (location.source, location.index),
        (jaguar, echo_index()),
        "the Jaguar's own echo, and not a rule's ability of no card"
    );

    assert!(priority_with(&engine, P0), "the active player, first");
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(priority_with(&engine, P1), "{:?}", engine.pending());
    assert_eq!(stack(&engine), [echo], "and it is still there for seat 1");
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    assert!(asked_to_pay(&engine, P0), "{:?}", engine.pending());
    assert_eq!(stack(&engine), [echo], "asked as it resolves (CR 608.2)");
}

/// Priority over the trigger is a place to make the mana, and it stays in the
/// pool until the step ends (CR 500.5): the yes spends it, and needs no window.
#[test]
fn mana_made_over_the_trigger_pays_it_without_a_window() {
    let mut engine = upkeep(&Table::default());
    tap_all_mana(&mut engine, P0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    both_pass(&mut engine);
    assert!(asked_to_pay(&engine, P0), "{:?}", engine.pending());
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(engine.payment_window(), None);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1, "{{G}} paid");
    assert!(stack_is_empty(&engine), "and the ability is gone");
    assert!(on_battlefield(&engine, P0, jaguar()).is_some());
}

/// Seat 1 kills the Jaguar over its echo. The second check finds nothing to
/// ask about, and the ability leaves the stack without having resolved.
#[test]
fn a_permanent_that_leaves_in_response_asks_nothing() {
    let bolt = named("Lightning Bolt");
    let mut engine = upkeep(&Table {
        their_board: &[named("Mountain")],
        their_hand: &[bolt],
        ..Table::default()
    });
    let creature = on_battlefield(&engine, P0, jaguar()).expect("the Jaguar is in play");
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, P1, bolt);
    engine
        .apply(
            P1,
            PlayerAction::ChooseTargets {
                objects: vec![creature],
                players: vec![],
            },
        )
        .unwrap();
    assert_eq!(stack(&engine).len(), 2, "the Bolt over the echo");
    // The Bolt resolves, and the Jaguar goes with the state-based actions
    // that follow.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert_eq!(stack(&engine).len(), 1, "the echo waits");
    assert!(in_graveyard(&engine, P0, jaguar()).is_some());
    both_pass(&mut engine);
    assert!(stack_is_empty(&engine));
    assert!(
        priority_with(&engine, P0),
        "nothing is asked: {:?}",
        engine.pending()
    );
    assert!(
        engine
            .state()
            .journal
            .entries()
            .iter()
            .any(|entry| matches!(entry.event, GameEvent::StackObjectDidNotResolve { .. })),
        "removed and not resolved (CR 603.4)"
    );
}

/// A Jaguar blinked in response has come back as a new object (CR 400.7): the
/// ability on the stack is not about it, and it asks nothing. What came back
/// came under its controller's control now, so it owes at the next upkeep.
#[test]
fn a_permanent_blinked_in_response_is_a_new_object_that_owes_at_the_next_upkeep() {
    let ephemerate = named("Ephemerate");
    let mut engine = upkeep(&Table {
        my_board: &[named("Plains")],
        my_hand: &[ephemerate],
        ..Table::default()
    });
    let creature = on_battlefield(&engine, P0, jaguar()).expect("the Jaguar is in play");
    let [echo] = stack(&engine)[..] else {
        panic!("{:?}", stack(&engine))
    };
    cast_from_hand(&mut engine, P0, ephemerate);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![creature],
            },
        )
        .unwrap();
    both_pass(&mut engine);
    assert_eq!(stack(&engine), [echo], "the Ephemerate has resolved");
    both_pass(&mut engine);
    assert!(stack_is_empty(&engine));
    assert!(priority_with(&engine, P0), "{:?}", engine.pending());
    assert!(on_battlefield(&engine, P0, jaguar()).is_some());

    to_upkeep_of(&mut engine, P0);
    assert_eq!(stack(&engine).len(), 1, "the new object's own echo");
}

/// Taken in response: the ability is seat 0's and the Jaguar is not, so there
/// is nothing it could sacrifice (CR 701.21a) and nothing to ask. The Jaguar
/// going home has come under seat 0's control again, and owes.
#[test]
fn a_permanent_taken_in_response_asks_nothing_and_owes_again_when_it_comes_back() {
    let ray = named("Ray of Command");
    let island = named("Island");
    let mut engine = upkeep(&Table {
        their_board: &[island, island, island, island],
        their_hand: &[ray],
        ..Table::default()
    });
    let creature = on_battlefield(&engine, P0, jaguar()).expect("the Jaguar is in play");
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, P1, ray);
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![creature],
            },
        )
        .unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert_eq!(
        engine.state().object(creature).map(|o| o.controller),
        Some(P1)
    );
    both_pass(&mut engine);
    assert!(stack_is_empty(&engine));
    assert!(priority_with(&engine, P0), "{:?}", engine.pending());
    assert_eq!(
        engine.state().object(creature).map(|o| o.controller),
        Some(P1),
        "and it is still there"
    );

    to_upkeep_of(&mut engine, P1);
    assert!(
        stack_is_empty(&engine),
        "seat 1 gave it back as the turn ended, so its upkeep owes nothing"
    );
    to_upkeep_of(&mut engine, P0);
    assert_eq!(stack(&engine).len(), 1, "seat 0 took it back, and owes");
}

/// The intervening `if`, as the upkeep begins: paid once, the Jaguar has been
/// under its controller since before their last upkeep, and that is that.
#[test]
fn a_permanent_under_control_since_before_the_last_upkeep_does_not_trigger() {
    let mut engine = upkeep(&Table::default());
    tap_all_mana(&mut engine, P0);
    both_pass(&mut engine);
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    to_upkeep_of(&mut engine, P0);
    assert!(
        stack_is_empty(&engine),
        "no echo a second time: {:?}",
        stack(&engine)
    );
    assert!(on_battlefield(&engine, P0, jaguar()).is_some());
}

/// Control Magic takes the Jaguar in seat 1's turn: it is under seat 1 at seat
/// 0's upkeep, where nothing happens, and it came under seat 1 since their last
/// upkeep, so theirs is where the echo is owed. It used to stay with the player
/// the permanent entered under, who was asked to pay for a creature they no
/// longer had.
#[test]
fn a_stolen_permanent_owes_its_echo_to_the_player_who_took_it() {
    let magic = named("Control Magic");
    let island = named("Island");
    let mut engine = jaguar_cast(&Table {
        their_board: &[island, island, island, island],
        their_hand: &[magic],
        ..Table::default()
    });
    reach_their_main_phase(&mut engine, P1);
    let creature = on_battlefield(&engine, P0, jaguar()).expect("the Jaguar is in play");
    cast_from_hand(&mut engine, P1, magic);
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![creature],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(creature).map(|o| o.controller),
        Some(P1)
    );

    to_upkeep_of(&mut engine, P0);
    assert!(stack_is_empty(&engine), "seat 0 no longer has it");

    to_upkeep_of(&mut engine, P1);
    let [echo] = stack(&engine)[..] else {
        panic!("seat 1's echo: {:?}", stack(&engine))
    };
    assert_eq!(engine.state().object(echo).map(|o| o.controller), Some(P1));
    assert_eq!(
        engine
            .state()
            .object(echo)
            .and_then(|o| o.ability)
            .map(|l| l.source),
        Some(creature)
    );
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(asked_to_pay(&engine, P1), "{:?}", engine.pending());
    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
    assert!(
        in_graveyard(&engine, P0, jaguar()).is_some(),
        "unpaid, it goes to its owner's graveyard"
    );
}

/// Borrowed for a turn after the echo was paid: Threaten's end hands it back,
/// and a change of control in either direction is a coming under control.
#[test]
fn a_permanent_handed_back_owes_its_echo_again() {
    let threaten = named("Threaten");
    let mountain = named("Mountain");
    let mut engine = upkeep(&Table {
        their_board: &[mountain, mountain, mountain],
        their_hand: &[threaten],
        ..Table::default()
    });
    tap_all_mana(&mut engine, P0);
    both_pass(&mut engine);
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    let creature = on_battlefield(&engine, P0, jaguar()).expect("the echo was paid");

    reach_their_main_phase(&mut engine, P1);
    cast_from_hand(&mut engine, P1, threaten);
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![creature],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(creature).map(|o| o.controller),
        Some(P1)
    );

    to_upkeep_of(&mut engine, P0);
    assert_eq!(
        engine.state().object(creature).map(|o| o.controller),
        Some(P0)
    );
    assert_eq!(
        stack(&engine).len(),
        1,
        "it came under seat 0's control again"
    );
}

/// Seat 0's echo and seat 1's Copper Tablet trigger at one upkeep: the active
/// player's goes on first and the other's above it (CR 603.3b), so it is the
/// Tablet that resolves first, and the echo is still waiting after it.
#[test]
fn two_upkeep_triggers_go_on_the_stack_in_apnap_order() {
    let mut engine = upkeep(&Table {
        their_board: &[named("Copper Tablet")],
        ..Table::default()
    });
    let [bottom, top] = stack(&engine)[..] else {
        panic!("two triggers: {:?}", stack(&engine))
    };
    let of = |id| engine.state().object(id).map(|o| o.controller);
    assert_eq!(of(bottom), Some(P0), "the active player's, underneath");
    assert_eq!(of(top), Some(P1), "the other player's, on top");
    assert_eq!(
        engine
            .state()
            .object(bottom)
            .and_then(|o| o.ability)
            .map(|l| l.index),
        Some(echo_index())
    );

    let life = engine.state().players[0].life;
    both_pass(&mut engine);
    assert_eq!(engine.state().players[0].life, life - 1, "the Tablet first");
    assert_eq!(stack(&engine), [bottom], "the echo waits");
    assert!(priority_with(&engine, P0), "{:?}", engine.pending());
    both_pass(&mut engine);
    assert!(asked_to_pay(&engine, P0), "{:?}", engine.pending());
}

/// A controlled turn (CR 723.5): the permanent, the ability and the mana are
/// the controlled player's (CR 723.3, 723.5a), and it is the player
/// controlling them who answers, and who taps their lands for it.
#[test]
fn the_controller_of_a_turn_decides_the_payment_for_the_controlled_player() {
    let mut engine = upkeep_after(&Table::default(), |engine| {
        engine
            .state
            .turn_controls
            .push(crate::state::PlayerTurnControl {
                controller: P1,
                player: P0,
                turn: engine.state.turn.number + 1,
                began: None,
                source: None,
            });
    });
    assert_eq!(stack(&engine).len(), 1, "the echo triggered all the same");
    assert_eq!(engine.decision_actor(), Some(P1));
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    assert!(asked_to_pay(&engine, P0), "{:?}", engine.pending());
    assert_eq!(engine.decision_actor(), Some(P1));
    assert!(
        engine.apply(P0, PlayerAction::YesNo(false)).is_err(),
        "the controlled player has no say in it"
    );

    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        engine.payment_window(),
        Some((P0, ManaPayment::Fixed(ManaCost::parse("{G}"))))
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("{:?}", engine.pending())
    };
    let source = legal.mana_abilities[0];
    engine
        .apply(P1, PlayerAction::ActivateManaAbility { source })
        .unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    assert_eq!(engine.payment_window(), None);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(on_battlefield(&engine, P0, jaguar()).is_some(), "paid");
}

/// Echo is a printed ability, so it is what the permanent has that triggers:
/// a Humility in play takes it away and there is nothing to trigger. It used
/// to be registered as the permanent entered and then come due regardless.
#[test]
fn a_permanent_that_lost_its_abilities_has_no_echo() {
    let humility = named("Humility");
    let plains = named("Plains");
    let mut engine = jaguar_cast(&Table {
        their_board: &[plains, plains, plains, plains],
        their_hand: &[humility],
        ..Table::default()
    });
    reach_their_main_phase(&mut engine, P1);
    cast_from_hand(&mut engine, P1, humility);
    pass_until(&mut engine, stack_is_empty);
    to_upkeep_of(&mut engine, P0);
    assert!(stack_is_empty(&engine), "{:?}", stack(&engine));
    assert!(on_battlefield(&engine, P0, jaguar()).is_some());
}

/// A Clone of the Jaguar has its echo as a copiable value (CR 707.2), and owes
/// it the way the Jaguar does.
#[test]
fn a_copy_of_a_permanent_with_echo_owes_it() {
    let clone = named("Clone");
    let island = named("Island");
    let mut engine = jaguar_cast(&Table {
        their_board: &[island, island, island, island],
        their_hand: &[clone],
        ..Table::default()
    });
    reach_their_main_phase(&mut engine, P1);
    let creature = on_battlefield(&engine, P0, jaguar()).expect("the Jaguar is in play");
    cast_from_hand(&mut engine, P1, clone);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![creature],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    to_upkeep_of(&mut engine, P1);
    let [echo] = stack(&engine)[..] else {
        panic!("the copy's echo: {:?}", stack(&engine))
    };
    assert_eq!(engine.state().object(echo).map(|o| o.controller), Some(P1));
}
