//! `cards/sorceries/mv_3/winter_s_grasp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Winter's Grasp` is a sorcery costing `{1}{G}{G}` under `Coverage::Implemented` that destroys target land.
/// When cast from hand off three Forests, it asks for a target land across both players' battlefields.
/// Choosing the opponent's Mountain causes it to be destroyed and put into their graveyard upon resolution.
#[test]
fn winter_s_grasp_destroys_target_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[winter_s_grasp()])
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let their_land = on_battlefield(&engine, p1, mountain()).expect("opponent controls a Mountain");
    cast_from_hand(&mut engine, p0, winter_s_grasp());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for land destruction, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&their_land),
        "opponent's Mountain is among legal land targets: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![their_land],
                players: Vec::new(),
            },
        )
        .expect("targeting opponent's Mountain is legal");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, mountain()).is_none(),
        "opponent's Mountain was destroyed from the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, mountain()).is_some(),
        "destroyed Mountain is in opponent's graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, winter_s_grasp()).is_some(),
        "resolved Winter's Grasp is in its caster's graveyard"
    );
}
