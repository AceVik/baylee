//! `cards/lands/utility/shelldock_isle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shelldock Isle: "Hideaway 4" / "This land enters tapped." / "{T}: Add {U}." / "{U}, {T}: You may play the exiled card without paying its mana cost if a library has twenty or fewer cards in it."
/// Under `Coverage::Partial`, Hideaway 4 and the library-size conditional cast are omitted.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for blue mana.
#[test]
fn shelldock_isle_enters_tapped_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(230, forest())
        .hand(0, &[shelldock_isle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, shelldock_isle());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, shelldock_isle(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}
