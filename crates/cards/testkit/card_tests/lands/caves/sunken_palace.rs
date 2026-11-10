//! `cards/lands/caves/sunken_palace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sunken Palace: "This land enters tapped." / "{T}: Add {U}." / "{1}{U}, {T}, Exile seven cards..."
/// Under `Coverage::Partial`, the graveyard-exile copy activation is omitted because no cost part exiles cards from a graveyard.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for blue mana.
#[test]
fn sunken_palace_enters_tapped_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(301, forest()).hand(0, &[sunken_palace()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, sunken_palace());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, sunken_palace(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}
