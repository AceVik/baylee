//! `cards/lands/utility/fertile_thicket.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fertile Thicket: "This land enters tapped." / "When this land enters, you may look at the top five cards of your library..." / "{T}: Add {G}."
/// Under `Coverage::Partial`, the enters trigger searching the top five cards is omitted.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for green mana.
#[test]
fn fertile_thicket_enters_tapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(333, forest())
        .hand(0, &[fertile_thicket()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, fertile_thicket());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, fertile_thicket(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}
