//! `cards/lands/utility/windbrisk_heights.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Windbrisk Heights: "Hideaway 4" / "This land enters tapped." / "{T}: Add {W}." / "{W}, {T}: You may play the exiled card without paying its mana cost if you attacked with three or more creatures this turn."
/// Under `Coverage::Partial`, Hideaway 4 and the attack-count conditional cast are omitted.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for white mana.
#[test]
fn windbrisk_heights_enters_tapped_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(232, forest())
        .hand(0, &[windbrisk_heights()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, windbrisk_heights());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, windbrisk_heights(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}
