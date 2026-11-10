//! `cards/creatures/artifacts/mv_5/coal_golem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Coal Golem` prints `{{3}}, Sacrifice this creature: Add {{R}}{{R}}{{R}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Coal Golem` and three Forests.
/// Tapping the Forests floats three green mana to pay the activation cost.
/// As a mana ability, activating it does not use the stack, immediately sacrificing the golem and adding three red mana.
#[test]
fn coal_golem_sacrifices_itself_to_produce_three_red_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[coal_golem(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let golem = on_battlefield(&engine, p0, coal_golem()).expect("coal golem is seated");
    assert_eq!(pt(&engine, golem), (3, 3));

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        3
    );

    activate(&mut engine, p0, coal_golem(), 0);
    assert!(
        stack_is_empty(&engine),
        "a mana ability resolves immediately without using the stack"
    );
    assert!(
        on_battlefield(&engine, p0, coal_golem()).is_none(),
        "`Coal Golem` was sacrificed as cost"
    );
    assert!(
        in_graveyard(&engine, p0, coal_golem()).is_some(),
        "`Coal Golem` is in the graveyard"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        3,
        "the ability added three red mana"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the three green mana were spent on the cost"
    );
    assert_eq!(pool.total(), 3, "only the three red mana remain");
}
