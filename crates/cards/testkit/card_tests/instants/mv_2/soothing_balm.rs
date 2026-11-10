//! `cards/instants/mv_2/soothing_balm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Soothing Balm` is an instant costing `{1}{W}` under `Coverage::Implemented`.
/// It prints "Target player gains 5 life."
/// Because its target specification is `TargetSpec::AnyPlayer`, casting it prompts
/// with `Pending::ChoosePlayer`. Answering with `PlayerAction::ChoosePlayer` increases
/// the target player's life total by 5 upon resolution.
#[test]
fn soothing_balm_gains_five_life_for_target_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[soothing_balm()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, soothing_balm());

    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!(
            "expected ChoosePlayer prompt for Soothing Balm, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0, "caster chooses the target player");
    assert!(
        options.contains(&p0) && options.contains(&p1),
        "target player reaches either seat: {options:?}"
    );

    engine.apply(p0, PlayerAction::ChoosePlayer(p0)).unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        25,
        "target player gained 5 life, increasing total from 20 to 25"
    );
    assert!(
        in_graveyard(&engine, p0, soothing_balm()).is_some(),
        "Soothing Balm resolves and goes to graveyard"
    );
}
