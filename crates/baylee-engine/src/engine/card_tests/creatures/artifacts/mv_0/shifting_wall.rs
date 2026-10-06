//! `cards/creatures/artifacts/mv_0/shifting_wall.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Shifting Wall` is an artifact creature wall with `Coverage::Implemented`.
/// It prints `{X}` as its mana cost, `KeywordSet::DEFENDER`, and enters the battlefield
/// with `X` `+1/+1` counters. Casting it with three floating mana from three `forest()`
/// lands prompts for `Pending::ChooseNumber`, and choosing 3 puts a 3/3 artifact wall with
/// defender onto the battlefield that cannot be declared as an attacker.
#[test]
fn shifting_wall_enters_with_x_counters_and_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[shifting_wall()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, shifting_wall());

    let Pending::ChooseNumber { .. } = engine.pending().clone() else {
        panic!(
            "expected `Pending::ChooseNumber`, got {:?}",
            engine.pending()
        );
    };
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let wall = on_battlefield(&engine, p0, shifting_wall()).expect("wall resolved");
    assert_eq!(counters_on(&engine, wall, CounterKind::P1P1), 3);
    assert_eq!(pt(&engine, wall), (3, 3));
    let kw = keywords(&engine, wall);
    assert!(kw.contains(KeywordSet::DEFENDER));
    let t = types(&engine, wall);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));

    // Defender prevents it from attacking when combat arrives.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on ChooseAttackers");
    };
    assert!(
        !attackers.contains(&wall),
        "defender prevents `Shifting Wall` from being offered as an attacker"
    );
}
