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
                prompt: crate::choice::YesNoPrompt::PayPact { .. },
                ..
            }
        )
    });
    assert!(engine.state().turn.number > cast_turn);
    assert_eq!(engine.state().turn.step, crate::turn::Step::Upkeep);
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
