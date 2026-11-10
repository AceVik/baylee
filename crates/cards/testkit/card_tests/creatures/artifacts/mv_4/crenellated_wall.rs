//! `cards/creatures/artifacts/mv_4/crenellated_wall.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Crenellated Wall` prints `Defender` and `{{T}}: Target creature gets +0/+4 until end of turn.`
/// on a 0/4 artifact creature wall with `Coverage::Implemented`.
/// In this scenario, `Crenellated Wall` taps to target `quiet_creature()`. Upon resolution,
/// the target's toughness increases by 4 (from 1/1 to 1/5) until end of turn, while `KeywordSet::DEFENDER`
/// prevents the wall itself from being declared as an attacker.
#[test]
fn crenellated_wall_taps_to_give_target_creature_four_toughness() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[crenellated_wall(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wall = on_battlefield(&engine, p0, crenellated_wall()).expect("wall is seated");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("elf is seated");
    assert_eq!(pt(&engine, elf), (1, 1));
    assert!(keywords(&engine, wall).contains(KeywordSet::DEFENDER));
    assert!(!is_tapped(&engine, wall));

    activate(&mut engine, p0, crenellated_wall(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elf));
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    assert!(is_tapped(&engine, wall));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, elf), (1, 5));

    // Defender prevents it from attacking when combat arrives.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on ChooseAttackers");
    };
    assert!(
        !attackers.contains(&wall),
        "`Crenellated Wall` cannot attack because it has defender"
    );
}
