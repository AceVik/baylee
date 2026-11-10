//! `cards/creatures/mv_2/basal_thrull.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Basal Thrull — {B}{B} 1/2 Thrull whose entire printed text is
/// "{T}, Sacrifice this creature: Add {B}{B}." The price is its own tap plus
/// itself, so the creature leaving the battlefield is the only evidence the
/// second half was actually paid, and the two black in the pool is the only
/// evidence the ability resolved at all. The pool is empty before the
/// activation — the two Swamps that cast it were spent doing so — which is
/// what makes "two black and nothing else" an exact claim rather than a
/// count of every source on the board.
#[test]
fn basal_thrull_taps_and_sacrifices_itself_for_two_black() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(8801, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[basal_thrull()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, basal_thrull());
    pass_until(&mut engine, stack_is_empty);
    let thrull = on_battlefield(&engine, p0, basal_thrull()).expect("the Thrull resolved");
    assert_eq!(pt(&engine, thrull), (1, 2), "the body the card prints");

    // Its controller's next turn: the cost is `{T}`, and a creature that
    // arrived this turn cannot pay one (CR 302.6). Asserting the offer on the
    // turn it was cast would have been asserting summoning sickness away.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let thrull = on_battlefield(&engine, p0, basal_thrull()).expect("the Thrull is still out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the two Swamps that cast it are spent, so whatever shows up below is the ability's"
    );
    assert!(
        !is_tapped(&engine, thrull),
        "and it is untapped, so {{T}} is payable"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(thrull, 0)),
        "the whole price is its own tap and itself, so the one line it prints is offered: {:?}",
        legal.abilities
    );
    assert!(
        !legal.mana_abilities.contains(&thrull),
        "a printed mana ability is an ordinary (source, index) entry and not the CR 305.6 shortcut"
    );

    activate(&mut engine, p0, basal_thrull(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 2, "Add {{B}}{{B}}");
    assert_eq!(pool.total(), 2, "and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and the seat holds priority again, got {:?}",
        engine.pending()
    );
    assert!(
        on_battlefield(&engine, p0, basal_thrull()).is_none(),
        "the other half of the cost put the Thrull somewhere"
    );
    assert!(
        in_graveyard(&engine, p0, basal_thrull()).is_some(),
        "its owner's graveyard, which is where a sacrificed permanent goes"
    );
}
