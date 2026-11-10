//! `cards/creatures/mv_4/souldrinker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Souldrinker` is a `{3}{B}` 2/2 Spirit under `Coverage::Implemented`.
/// Its activated ability costs 3 life and puts a +1/+1 counter on itself without tapping or requiring mana.
/// Activating the ability deducts 3 life as an immediate cost, places the ability on the stack,
/// and upon resolution places a `CounterKind::P1P1` counter on the permanent to increase its power and toughness to 3/3.
#[test]
fn souldrinker_pays_life_to_gain_plus_one_plus_one_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[souldrinker()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let drinker =
        on_battlefield(&engine, p0, souldrinker()).expect("Souldrinker is on battlefield");
    assert_eq!(pt(&engine, drinker), (2, 2), "initially a 2/2");
    assert_eq!(
        counters_on(&engine, drinker, CounterKind::P1P1),
        0,
        "starts with zero counters"
    );
    let initial_life = engine.state().players[0].life;

    activate(&mut engine, p0, souldrinker(), 0);

    assert_eq!(
        engine.state().players[0].life,
        initial_life - 3,
        "paying 3 life is deducted as an activation cost"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, drinker, CounterKind::P1P1),
        1,
        "one +1/+1 counter is added upon resolution"
    );
    assert_eq!(
        pt(&engine, drinker),
        (3, 3),
        "grows to a 3/3 with the counter"
    );
    assert!(
        !is_tapped(&engine, drinker),
        "ability does not require tapping"
    );
}
