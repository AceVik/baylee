//! `cards/creatures/mv_8/copper_host_crusher.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn copper_host_crusher() -> CardIndex {
    card_index("6971eac1-afd0-46c3-a37f-4a26d50ef656")
}

/// Copper Host Crusher is an 8/8 for {6}{G}{G} with trample and hexproof.
#[test]
fn copper_host_crusher_is_cast_for_eight_with_trample_and_hexproof() {
    let mut engine = at_main(&[forest(); 8], &[copper_host_crusher()]);
    let crusher = cast_and_settle(&mut engine, copper_host_crusher());
    assert_eq!(pt(&engine, crusher), (8, 8));
    assert!(keywords(&engine, crusher).contains(KeywordSet::TRAMPLE));
    assert!(keywords(&engine, crusher).contains(KeywordSet::HEXPROOF));
}

/// Hexproof: the opponent's Vindicate (target permanent) is offered the
/// opponent's own Bears and the controller's Bears, but not the Crusher.
#[test]
fn copper_host_crusher_cannot_be_targeted_by_an_opponent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[copper_host_crusher(), grizzly_bears()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    let crusher = on_battlefield(&engine, p0, copper_host_crusher()).expect("the Crusher");
    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("the bear");
    cast_from_hand(&mut engine, p1, vindicate());
    let options = pass_until_targets(&mut engine, p1);
    assert!(options.contains(&bear), "a plain permanent is a target");
    assert!(
        !options.contains(&crusher),
        "hexproof keeps the opponent off it: {options:?}"
    );
}

/// Hexproof binds opponents only: its own controller may target it.
#[test]
fn copper_host_crusher_can_be_targeted_by_its_controller() {
    let p0 = PlayerId::new(0);
    let mut engine = at_main(
        &[copper_host_crusher(), plains(), swamp(), plains()],
        &[vindicate()],
    );
    let crusher = on_battlefield(&engine, p0, copper_host_crusher()).expect("the Crusher");
    cast_from_hand(&mut engine, p0, vindicate());
    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&crusher), "{options:?}");
}
