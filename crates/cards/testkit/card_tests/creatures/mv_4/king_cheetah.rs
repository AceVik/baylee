//! `cards/creatures/mv_4/king_cheetah.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "f9ca35e1-24c5-46f4-a666-0dfe2c920e8b"

/// King Cheetah is {3}{G} for a 3/2 Cat whose entire printed text is Flash, so
/// the only thing a board can hold it to is the window it may be cast in —
/// every offer the engine publishes is filtered by timing, and a creature with
/// no flash is offered in nobody's main phase but its controller's. Llanowar
/// Elves in the same hand is the control that makes that claim exact: five
/// green are already floating when the offer is read, so the Elves is
/// affordable and missing for the one reason that is left, while the Cheetah
/// resolving on a turn that belongs to the other seat is the keyword itself.
#[test]
fn king_cheetah_flashes_in_on_the_opponents_turn_where_a_plain_creature_may_not() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 5])
        // `quiet_creature` is Llanowar Elves: a {G} 1/1 with no text but a mana
        // ability, so the two cards differ in nothing but the word under test.
        .hand(0, &[king_cheetah(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);

    // Walked into a priority window this seat does not own: the opponent's
    // first main phase, which is a moment no creature without Flash may be
    // cast in at all.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    // Mana before the claim: `can_afford` reads the pool and not the untapped
    // Forests, so an empty pool would withhold both cards and prove nothing.
    // Neither card is on the battlefield, so nothing here makes mana but the
    // five Forests.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Forests tapped on a turn this seat is not taking"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let cheetah = in_hand(&engine, p0, king_cheetah()).expect("the Cheetah is in hand");
    let elves = in_hand(&engine, p0, quiet_creature()).expect("the Elves are in hand");
    assert!(
        legal.castable.contains(&cheetah),
        "Flash: a creature with it is cast any time its controller could cast \
         an instant, and the opponent's main phase with an empty stack is one: {:?}",
        legal.castable
    );
    assert!(
        !legal.castable.contains(&elves),
        "same seat, same full pool, and a creature without Flash is not offered \
         at all — which is what says the entry above is the keyword and not an \
         affordability this board happens to have: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, king_cheetah());
    pass_until(&mut engine, stack_is_empty);

    let landed = on_battlefield(&engine, p0, king_cheetah())
        .expect("the Cheetah resolved under the seat that is not taking the turn");
    assert_eq!(pt(&engine, landed), (3, 2), "the body the card prints");
    assert_eq!(
        engine.state().turn.active,
        p1,
        "and the turn still belongs to the other seat, so the spell resolved out \
         of the window Flash opened and not out of a turn that came round"
    );
    assert!(
        in_hand(&engine, p0, quiet_creature()).is_some(),
        "the Elves never left the hand: the window was the whole difference"
    );
}
