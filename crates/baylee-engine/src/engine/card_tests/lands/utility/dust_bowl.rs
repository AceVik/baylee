//! `cards/lands/utility/dust_bowl.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dust Bowl: "{3}, {T}, Sacrifice a land: Destroy target nonbasic land."
/// The target is the opponent's Dust Bowl (nonbasic); the sacrifice is a
/// Forest, and a basic land is not on offer as a target.
#[test]
fn dust_bowl_sacrifices_a_land_to_destroy_a_nonbasic_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let bowl = card_index("d3df7128-31dd-4d71-90be-87e2e9ff51b4");
    let mut engine = Duel::new(2102, forest())
        .battlefield(0, &[bowl, forest(), forest(), forest(), forest()])
        .battlefield(1, &[bowl, island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, bowl).expect("my bowl");
    let theirs = on_battlefield(&engine, p1, bowl).expect("their bowl");
    let their_island = on_battlefield(&engine, p1, island()).expect("their island");
    let sac = on_battlefield(&engine, p0, forest()).expect("forest");
    tap_mana_where(&mut engine, p0, |id| id != mine && id != sac);

    activate(&mut engine, p0, bowl, 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&theirs), "a nonbasic land is a target");
    assert!(!options.contains(&their_island), "a basic land is not");
    unf_aim_and_pay(&mut engine, p0, Some(theirs), Some(sac));

    assert!(on_battlefield(&engine, p1, bowl).is_none(), "destroyed");
    assert!(on_battlefield(&engine, p1, island()).is_some());
    assert!(in_graveyard(&engine, p0, forest()).is_some(), "the cost");
}
