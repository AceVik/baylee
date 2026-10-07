//! `cards/lands/utility/glimmervoid.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Glimmervoid: "At the beginning of the end step, if you control no artifacts, sacrifice this land." / "{T}: Add one mana of any color."
/// Under `Coverage::Partial`, the end-step zero-artifact sacrifice trigger is omitted.
/// Activating the land prompts for a color choice and adds one mana of the chosen color ({R}) to the pool.
#[test]
fn glimmervoid_taps_for_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(118, forest())
        .battlefield(0, &[glimmervoid()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, glimmervoid()).expect("Glimmervoid deployed");
    activate(&mut engine, p0, glimmervoid(), 0);

    let Pending::ChooseColor { player, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

/// Glimmervoid: "At the beginning of the end step, if you control no
/// artifacts, sacrifice this land." Two things are being asserted and the
/// second one is the reason the first is not enough — the land with an
/// artifact beside it survives the same end step, so the sacrifice is the
/// condition answering and not the trigger firing unconditionally.
#[test]
fn glimmervoid_sacrifices_itself_at_the_end_step_with_no_artifact_to_hold_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    let mut alone = Duel::new(9209, forest())
        .battlefield(0, &[glimmervoid()])
        .start();
    keep_mulligans(&mut alone);
    reach_main_phase(&mut alone, p0);
    assert!(
        on_battlefield(&alone, p0, glimmervoid()).is_some(),
        "the land is on the table for the whole main phase"
    );
    reach_their_main_phase(&mut alone, p1);
    assert!(
        on_battlefield(&alone, p0, glimmervoid()).is_none(),
        "the end step came and no artifact was controlled"
    );

    let mut held = Duel::new(9210, forest())
        .battlefield(0, &[glimmervoid(), basilisk_collar()])
        .start();
    keep_mulligans(&mut held);
    reach_main_phase(&mut held, p0);
    reach_their_main_phase(&mut held, p1);
    assert!(
        on_battlefield(&held, p0, glimmervoid()).is_some(),
        "one artifact is enough, and it is the same end step"
    );
}
