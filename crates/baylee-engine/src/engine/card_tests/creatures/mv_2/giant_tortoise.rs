//! `cards/creatures/mv_2/giant_tortoise.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Giant Tortoise` prints a static ability granting +0/+3 as long as it is untapped under `Coverage::Implemented`.
/// While standing untapped before combat, its projected `pt` is (1, 4).
/// Upon declaring an attack in combat, the Tortoise taps, losing the conditional toughness bonus
/// and reverting to its printed `pt` of (1, 1).
#[test]
fn giant_tortoise_gets_bonus_only_while_untapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1624, forest())
        .battlefield(0, &[giant_tortoise()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tortoise = on_battlefield(&engine, p0, giant_tortoise()).expect("tortoise is seated");
    assert!(!is_tapped(&engine, tortoise));
    assert_eq!(
        pt(&engine, tortoise),
        (1, 4),
        "untapped tortoise gets +0/+3"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers prompt");
    };
    let defender = defenders.into_iter().next().expect("opponent is defender");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(tortoise, defender)],
            },
        )
        .unwrap();

    assert!(
        is_tapped(&engine, tortoise),
        "attacking tapped the tortoise"
    );
    assert_eq!(
        pt(&engine, tortoise),
        (1, 1),
        "tapped tortoise loses +0/+3 bonus"
    );
}
