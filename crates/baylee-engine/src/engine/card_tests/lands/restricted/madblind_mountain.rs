//! `cards/lands/restricted/madblind_mountain.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Madblind Mountain: "Madblind Mountain enters tapped." / "{R}, {T}: Shuffle your library. Activate only if you control two or more red permanents."
/// Under `Coverage::Partial`, no effect shuffles a library alone so the second ability is omitted.
/// Playing this land causes it to enter tapped, and after untapping it taps for red mana.
#[test]
fn madblind_mountain_enters_tapped_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(107, forest())
        .hand(0, &[madblind_mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, madblind_mountain());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, madblind_mountain(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}
