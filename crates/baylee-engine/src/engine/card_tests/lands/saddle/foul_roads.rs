//! `cards/lands/saddle/foul_roads.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Foul Roads: "This land enters tapped unless you control a Mount or Vehicle." / "{T}: Add {B}." / "{1}{B}, {T}, Sacrifice this land..."
/// Under `Coverage::Partial`, the sacrifice ability creating a Pilot token is omitted.
/// Playing this land without a Mount or Vehicle enters tapped, and after untapping on a subsequent turn it taps for black mana.
#[test]
fn foul_roads_enters_tapped_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(323, forest()).hand(0, &[foul_roads()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, foul_roads());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, foul_roads(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, land));
}
