//! `cards/creatures/artifacts/mv_4/wakandan_drone_flock.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::ArrangePrompt;

fn wakandan_drone_flock() -> CardIndex {
    card_index("282d6e61-3240-4f1e-9c2d-8ddb59b03af4")
}

/// Wakandan Drone Flock — {3}{W} 3/3 with flying: "When this creature enters,
/// scry 2." The scry is asked of its controller about their own top two
/// cards; sending the first to the bottom leaves the second on top and the
/// first at the bottom of the library.
#[test]
fn wakandan_drone_flock_scries_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), forest(), forest(), forest()])
        .hand(0, &[wakandan_drone_flock()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let top_two = library_top(&engine, p0, 2);
    let size = library_size(&engine, p0);

    cast_from_hand(&mut engine, p0, wakandan_drone_flock());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        prompt,
        piles,
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for this");
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, ArrangePrompt::Scry);
    assert_eq!(piles, scry_piles(2), "scry 2");
    assert_eq!(cards, top_two, "their own top two");
    engine
        .apply(p0, look_answer(&cards, &[cards[0]]))
        .expect("a legal scry");
    pass_until(&mut engine, stack_is_empty);

    let drone = on_battlefield(&engine, p0, wakandan_drone_flock()).expect("it resolved");
    assert_eq!(pt(&engine, drone), (3, 3));
    assert!(keywords(&engine, drone).contains(KeywordSet::FLYING));
    assert_eq!(library_size(&engine, p0), size, "a scry draws nothing");
    assert_eq!(library_top(&engine, p0, 1), vec![top_two[1]], "kept on top");
    let bottom = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .first()
        .expect("a library");
    assert_eq!(bottom, top_two[0], "the other went to the bottom");
}
