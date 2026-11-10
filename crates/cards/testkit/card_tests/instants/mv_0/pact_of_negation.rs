//! `cards/instants/mv_0/pact_of_negation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "f3e213a4-ba5a-468a-93b3-c0a34e1bd725"

/// Pact of Negation is a free counterspell paid for later: "Counter target
/// spell", and then "At the beginning of your next upkeep, pay {3}{U}{U}. If
/// you don't, you lose the game."
///
/// Both printed sentences are played here, because the first is what makes the
/// second reachable at all. The Pact answers a Llanowar Elves its own
/// controller has just cast — "target spell" names no controller, so the card
/// in the graveyard instead of on the battlefield is the whole of what
/// countered means — and the deferred price is then demanded in a *later*
/// turn's upkeep, where the engine's question is the only thing that proves the
/// second sentence is written at all. The five mana the answer spends are
/// floated in that same upkeep (CR 500.5 empties a pool only when a step ends),
/// and the seat is still in the game once the cost is paid.
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn pact_of_negation_counters_a_spell_now_and_charges_for_it_at_the_next_upkeep() {
    let p0 = PlayerId::new(0);
    let mut board = vec![island(); 8];
    board.push(forest());
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[llanowar_elves(), pact_of_negation()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A spell is put on the stack only so the Pact has something to answer: an
    // Elf is the cheapest card whose resolution leaves a mark in a different
    // zone than its countering does.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        9,
        "eight Islands and one Forest: every source on the board makes one"
    );
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    // {0}: the Pact is cast off a cost it does not pay into, and the caster
    // still holds priority with its own spell waiting below.
    cast_with_floating(&mut engine, p0, pact_of_negation());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one spell, and the Pact asks once");
    let elves_spell =
        on_stack(&engine, llanowar_elves()).expect("the Elves spell is waiting on the stack");
    assert!(
        options.contains(&elves_spell),
        "\"target spell\" puts the spell that has not resolved yet on the menu, \
         whichever seat cast it: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves_spell],
            },
        )
        .expect("the spell the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the spell was countered, so the creature never reached the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and a countered spell goes to its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, pact_of_negation()).is_some(),
        "the Pact itself resolved, which is where its second sentence starts"
    );
    let cast_turn = engine.state().turn.number;

    // Do not float mana speculatively in upkeep: the payment itself must
    // offer a mana window, even to a client that passed normal priority.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: baylee_engine::choice::YesNoPrompt::PayPact { .. },
                ..
            }
        )
    });
    assert!(engine.state().turn.number > cast_turn);
    assert_eq!(engine.state().turn.step, baylee_engine::turn::Step::Upkeep);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        engine.payment_window(),
        Some((
            p0,
            baylee_core::mana::ManaPayment::Fixed(baylee_core::mana::ManaCost::parse("{3}{U}{U}"))
        ))
    );
    tap_all_mana(&mut engine, p0);
    let before = engine.state().players[0].mana_pool.total();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(!engine.state().players[0].has_lost());
    assert_eq!(engine.state().players[0].mana_pool.total(), before - 5);
    assert!(engine.payment_window().is_none());
}

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn named(name: &str) -> CardIndex {
    baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"))
}

/// Seat 0 counters its own Elves with the Pact on its first turn, and the game
/// walks to its next upkeep, where the price has just gone on the stack.
/// `during` runs in seat 1's turn on the way.
fn price_due(
    board: &[CardIndex],
    theirs: &[CardIndex],
    during: impl FnOnce(&mut Engine<RegistryLookup>),
) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, board)
        .battlefield(1, theirs)
        .hand(0, &[llanowar_elves(), pact_of_negation()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0), "p0 reaches its own main");
    cast_from_hand(&mut engine, P0, llanowar_elves());
    cast_with_floating(&mut engine, P0, pact_of_negation());
    let elves = on_stack(&engine, llanowar_elves()).expect("the Elves are on the stack");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let cast_turn = engine.state().turn.number;
    pass_until(&mut engine, |e| e.state().turn.active == P1);
    during(&mut engine);
    pass_until(&mut engine, |e| {
        let turn = &e.state().turn;
        turn.number > cast_turn
            && turn.active == P0
            && turn.step == baylee_engine::turn::Step::Upkeep
    });
    engine
}

fn eight_islands_and_a_forest() -> Vec<CardIndex> {
    let mut board = vec![island(); 8];
    board.push(forest());
    board
}

fn stack(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    engine.state().zones.list(ZoneLocation::Stack).clone()
}

fn warned(engine: &Engine<RegistryLookup>) -> bool {
    matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: baylee_engine::choice::YesNoPrompt::PayPact { .. },
            ..
        }
    )
}

/// Both seats pass, by name, in the order priority goes round.
fn both_pass(engine: &mut Engine<RegistryLookup>) {
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
}

/// The Pact's second sentence is a delayed triggered ability (CR 603.7): on
/// the stack in that upkeep as the card's own second ability, controlled by
/// the player who cast it (CR 603.7d), with priority going round over it, and
/// asked only as it resolves. It used to be set aside and answered when the
/// upkeep's round closed on an empty stack.
#[test]
fn the_price_is_on_the_stack_and_both_players_hold_priority_over_it() {
    let mut engine = price_due(&eight_islands_and_a_forest(), &[], |_| {});
    let pact = in_graveyard(&engine, P0, pact_of_negation()).expect("the Pact resolved");

    let [price] = stack(&engine)[..] else {
        panic!("one object on the stack: {:?}", stack(&engine))
    };
    let object = engine.state().object(price).expect("it is an object");
    assert_eq!(
        object.kind,
        baylee_engine::object::ObjectKind::AbilityOnStack
    );
    assert_eq!(object.controller, P0);
    let location = object.ability.expect("it names an ability");
    assert_eq!(
        (location.source, location.card, location.index),
        (pact, Some(pact_of_negation()), 1),
        "the sentence the card prints, by its place in the list"
    );

    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0));
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == P1));
    assert_eq!(stack(&engine), [price]);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    assert!(warned(&engine), "{:?}", engine.pending());
    let Pending::YesNo { source, .. } = engine.pending() else {
        unreachable!()
    };
    assert_eq!(*source, None, "no standing answer may give a loss away");
}

/// The report of 28.9.2026: eight untapped Islands are not an empty pool's
/// loss. Asked with nothing floating, a yes opens the window, and the Islands
/// pay.
#[test]
fn eight_untapped_islands_are_asked_and_pay_in_the_window() {
    let mut engine = price_due(&eight_islands_and_a_forest(), &[], |_| {});
    both_pass(&mut engine);
    assert!(warned(&engine), "{:?}", engine.pending());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(!engine.state().players[0].has_lost());
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        engine.payment_window(),
        Some((
            P0,
            baylee_core::mana::ManaPayment::Fixed(baylee_core::mana::ManaCost::parse("{3}{U}{U}"))
        ))
    );
    tap_all_mana(&mut engine, P0);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(!engine.state().players[0].has_lost());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "nine less five"
    );
    assert!(stack_is_empty(&engine));
}

/// Mana made with the trigger on the stack is in the pool as it resolves, and
/// "pay" takes it without a question.
#[test]
fn mana_made_over_the_trigger_pays_it_unasked() {
    let mut engine = price_due(&eight_islands_and_a_forest(), &[], |_| {});
    tap_all_mana(&mut engine, P0);
    both_pass(&mut engine);
    assert!(!warned(&engine), "{:?}", engine.pending());
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    assert!(stack_is_empty(&engine));
    assert!(!engine.state().players[0].has_lost());
}

#[test]
fn declining_the_price_loses_the_game() {
    let mut engine = price_due(&eight_islands_and_a_forest(), &[], |_| {});
    both_pass(&mut engine);
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    assert!(engine.state().players[0].has_lost());
}

/// A yes is not a payment: the window closed with the mana unmade has not paid.
#[test]
fn a_window_closed_short_loses_the_game() {
    let mut engine = price_due(&eight_islands_and_a_forest(), &[], |_| {});
    both_pass(&mut engine);
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(engine.state().players[0].has_lost());
}

/// A delayed trigger comes due in the same batch as the upkeep's other
/// triggers (CR 603.3b): the active player's price goes on first, and seat
/// 1's Copper Tablet above it.
#[test]
fn the_price_and_the_other_players_trigger_go_on_the_stack_in_apnap_order() {
    let mut engine = price_due(
        &eight_islands_and_a_forest(),
        &[named("Copper Tablet")],
        |_| {},
    );
    let [bottom, top] = stack(&engine)[..] else {
        panic!("two triggers: {:?}", stack(&engine))
    };
    let of = |id| engine.state().object(id).map(|o| o.controller);
    assert_eq!((of(bottom), of(top)), (Some(P0), Some(P1)));
    both_pass(&mut engine);
    assert_eq!(stack(&engine), [bottom], "the Tablet resolved first");
    assert!(!warned(&engine), "{:?}", engine.pending());
    both_pass(&mut engine);
    assert!(warned(&engine), "{:?}", engine.pending());
}

/// A controlled turn (CR 724.5): the debt is the controlled player's and so
/// is the mana (CR 724.5a), and the player controlling them says whether it
/// is paid. Declining is theirs to do, and it loses the controlled player the
/// game.
#[test]
fn the_controller_of_a_turn_decides_the_price_for_the_controlled_player() {
    let mut engine = price_due(&eight_islands_and_a_forest(), &[], |engine| {
        engine
            .state
            .turn_controls
            .push(baylee_engine::state::PlayerTurnControl {
                controller: P1,
                player: P0,
                turn: engine.state.turn.number + 1,
                began: None,
                source: None,
            });
    });
    assert_eq!(engine.decision_actor(), Some(P1));
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    assert!(warned(&engine), "{:?}", engine.pending());
    assert_eq!(engine.pending().asked(), Some(P0));
    assert_eq!(engine.decision_actor(), Some(P1));
    assert!(engine.apply(P0, PlayerAction::YesNo(true)).is_err());
    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
    assert!(engine.state().players[0].has_lost());
}

/// A copy of the Pact has made a price of its own (CR 603.7d) and is gone by
/// the time it is due: the ability is still that card's by name and by
/// sentence, read off what the delayed trigger kept and not off its source.
/// Here the copy counters the Elves and the original loses its only target,
/// so the copy's is the only price.
#[test]
fn a_copied_pacts_price_outlives_the_copy() {
    let fork = named("Fork");
    let mut board = vec![island(); 5];
    board.extend([forest(), mountain(), mountain()]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[llanowar_elves(), pact_of_negation(), fork])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0), "p0 reaches its own main");
    cast_from_hand(&mut engine, P0, llanowar_elves());
    cast_with_floating(&mut engine, P0, pact_of_negation());
    let elves = on_stack(&engine, llanowar_elves()).expect("the Elves are on the stack");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    cast_with_floating(&mut engine, P0, fork);
    let pact = on_stack(&engine, pact_of_negation()).expect("the Pact is on the stack");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![pact],
            },
        )
        .unwrap();
    for _ in 0..40 {
        if stack_is_empty(&engine) {
            break;
        }
        let (player, action) = answer_one(&engine).unwrap();
        engine.apply(player, action).unwrap();
    }
    assert!(stack_is_empty(&engine), "{:?}", engine.pending());
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        let now = &e.state().turn;
        now.number > turn && now.active == P0 && now.step == baylee_engine::turn::Step::Upkeep
    });

    let [price] = stack(&engine)[..] else {
        panic!("the copy's price: {:?}", stack(&engine))
    };
    let object = engine.state().object(price).expect("it is an object");
    assert_eq!(
        engine.state().names.get(object.characteristics().name),
        "Pact of Negation"
    );
    let location = object.ability.expect("it names an ability");
    assert_eq!(
        (location.card, location.index),
        (Some(pact_of_negation()), 1)
    );
    both_pass(&mut engine);
    assert!(warned(&engine), "{:?}", engine.pending());
}
