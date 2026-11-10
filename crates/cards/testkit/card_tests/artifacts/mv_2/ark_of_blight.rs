//! `cards/artifacts/mv_2/ark_of_blight.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Ark of Blight` prints `{{3}}, {{T}}, Sacrifice this artifact: Destroy target land.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Ark of Blight` and three copies of `forest()`, while seat 1 controls a `forest()`.
/// Floating three mana pays to activate `Ark of Blight`, targeting the opponent's land, sacrificing the artifact,
/// and destroying the targeted land upon resolution.
#[test]
fn ark_of_blight_sacrifices_to_destroy_target_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), ark_of_blight()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target_land = on_battlefield(&engine, p1, forest()).expect("opponent controls a forest");
    tap_all_mana_but(&mut engine, p0, Some(ark_of_blight()));
    activate(&mut engine, p0, ark_of_blight(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&target_land),
        "opponent's land is a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_land],
            },
        )
        .expect("targeted opponent land");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, ark_of_blight()).is_some(),
        "`Ark of Blight` was sacrificed"
    );
    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "opponent's land was destroyed"
    );
}
