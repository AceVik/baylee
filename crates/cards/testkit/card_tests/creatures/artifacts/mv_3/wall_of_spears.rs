//! `cards/creatures/artifacts/mv_3/wall_of_spears.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Wall of Spears` prints `Defender` and `First strike` on a 2/3 artifact creature wall
/// under `Coverage::Implemented`.
/// Seated on the battlefield from turn one, it possesses 2 power, 3 toughness, both
/// `TypeSet::ARTIFACT` and `TypeSet::CREATURE` types, and the combination of `KeywordSet::DEFENDER`
/// and `KeywordSet::FIRST_STRIKE`. When combat arrives, `KeywordSet::DEFENDER` ensures it cannot
/// attack.
#[test]
fn wall_of_spears_has_first_strike_and_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wall_of_spears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wall = on_battlefield(&engine, p0, wall_of_spears()).expect("wall is seated");
    assert_eq!(pt(&engine, wall), (2, 3));
    let t = types(&engine, wall);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));
    let kw = keywords(&engine, wall);
    assert!(kw.contains(KeywordSet::DEFENDER));
    assert!(kw.contains(KeywordSet::FIRST_STRIKE));

    // Defender prevents it from attacking when combat arrives.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on ChooseAttackers");
    };
    assert!(
        !attackers.contains(&wall),
        "defender prevents `Wall of Spears` from being declared as an attacker"
    );
}
