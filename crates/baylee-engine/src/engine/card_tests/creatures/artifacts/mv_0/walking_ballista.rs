//! `cards/creatures/artifacts/mv_0/walking_ballista.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Walking Ballista — {X}{X} artifact creature: "This creature enters with X +1/+1
/// counters on it. {4}: Put a +1/+1 counter on this creature. Remove a +1/+1 counter
/// from this creature: It deals 1 damage to any target."
///
/// It is seeded straight onto the battlefield here and never cast, so there is no
/// announced X for its entry clause to read (CR 107.3g: a card outside the stack has
/// an X of 0) and the harness plants the one counter the 0/0 body needs to survive
/// the state-based action. That the clause *does* work off a real cast is proved
/// beside the rule, in [`enter_tests`], which is where it belongs — this test is
/// about the two activated abilities: paying {4} adds a counter (growing it to 2/2),
/// and removing a counter pays the cost to deal 1 damage to the opponent.
///
/// [`enter_tests`]: crate::engine::enter_tests
#[test]
fn walking_ballista_grows_with_mana_and_removes_a_counter_to_deal_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[walking_ballista(), forest(), forest(), forest(), forest()],
        )
        .life(1, 20)
        .start();

    let ballista =
        on_battlefield(&engine, p0, walking_ballista()).expect("the Ballista is on the table");

    // Nobody cast it, so its entry clause read an X of nothing: the counter it needs
    // to survive the SBA 0-toughness check is planted by the harness before mulligans.
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        crate::replacement::put_counters(state, ballista, CounterKind::P1P1, 1);
    }

    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    assert_eq!(
        counters_on(&engine, ballista, CounterKind::P1P1),
        1,
        "one counter planted by the harness"
    );
    assert_eq!(pt(&engine, ballista), (1, 1), "starts as a 1/1");

    // Ability 0: {4}: Put a +1/+1 counter on this creature.
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    activate(&mut engine, p0, walking_ballista(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, ballista, CounterKind::P1P1),
        2,
        "{{4}} added a second +1/+1 counter"
    );
    assert_eq!(pt(&engine, ballista), (2, 2), "grew to 2/2");

    // Ability 1: Remove a +1/+1 counter from this creature: It deals 1 damage to any target.
    activate(&mut engine, p0, walking_ballista(), 1);
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("ability 1 targets any target, got {:?}", engine.pending())
    };
    assert!(
        player_options.contains(&p1),
        "the opponent is a legal target for any target"
    );
    // The target is chosen before the cost is paid (CR 601.2c against CR
    // 601.2h, the last step of an activation), so the counter is still on
    // the creature while this question is open — it is not a cost the
    // engine takes as the ability is announced.
    assert_eq!(
        counters_on(&engine, ballista, CounterKind::P1P1),
        2,
        "the counter is still there while the target is being chosen"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("targeting the opponent is legal");

    // Now the activation is complete and the cost has been paid, with the
    // ability still on the stack: the second counter is gone and the body
    // it was holding up is a 1/1 again.
    assert_eq!(
        counters_on(&engine, ballista, CounterKind::P1P1),
        1,
        "removing a +1/+1 counter is the whole cost, and it is paid here"
    );
    assert_eq!(pt(&engine, ballista), (1, 1), "shrank back to 1/1");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "opponent took 1 damage from the ping"
    );
    assert_eq!(
        counters_on(&engine, ballista, CounterKind::P1P1),
        1,
        "Ballista retains its remaining counter"
    );
    assert!(
        on_battlefield(&engine, p0, walking_ballista()).is_some(),
        "Ballista survived on the battlefield"
    );
}
