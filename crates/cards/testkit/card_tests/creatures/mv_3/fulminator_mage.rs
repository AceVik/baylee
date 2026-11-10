//! `cards/creatures/mv_3/fulminator_mage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "Sacrifice this creature: Destroy target nonbasic land." A basic is not
/// on the menu; the nonbasic is destroyed and the Mage is in its owner's
/// graveyard as the cost.
#[test]
fn fulminator_mage_sacrifices_itself_to_destroy_a_nonbasic_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[fulminator_mage()])
        .battlefield(1, &[rogue_s_passage(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let passage = on_battlefield(&engine, p1, rogue_s_passage()).unwrap();
    activate(&mut engine, p0, fulminator_mage(), 0);
    let menu = aim_at(&mut engine, p0, passage);
    assert_eq!(menu, vec![passage], "the basic Forest is no target");
    assert!(
        in_graveyard(&engine, p0, fulminator_mage()).is_some(),
        "sacrificed as the cost"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, rogue_s_passage()).is_some());
    assert!(on_battlefield(&engine, p1, forest()).is_some());
}
