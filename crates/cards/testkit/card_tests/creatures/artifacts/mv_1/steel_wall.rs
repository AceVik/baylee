//! `cards/creatures/artifacts/mv_1/steel_wall.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Steel Wall` prints `Defender` on a 0/4 artifact creature wall with `Coverage::Implemented`.
/// Cast from hand using mana from a `forest()`, it resolves onto the battlefield with 0 power,
/// 4 toughness, and both `TypeSet::ARTIFACT` and `TypeSet::CREATURE` types.
/// When combat arrives, `KeywordSet::DEFENDER` ensures it cannot be declared as an attacker.
#[test]
fn steel_wall_enters_as_a_zero_four_defender_and_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[steel_wall()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, steel_wall());
    pass_until(&mut engine, stack_is_empty);

    let wall = on_battlefield(&engine, p0, steel_wall()).expect("wall resolved");
    assert_eq!(pt(&engine, wall), (0, 4));
    let t = types(&engine, wall);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));
    assert!(keywords(&engine, wall).contains(KeywordSet::DEFENDER));

    // Defender prevents it from attacking when combat arrives.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on ChooseAttackers");
    };
    assert!(
        !attackers.contains(&wall),
        "`Steel Wall` cannot attack because it has defender"
    );
}
