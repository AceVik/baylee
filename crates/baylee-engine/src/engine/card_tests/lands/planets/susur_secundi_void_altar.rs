//! `cards/lands/planets/susur_secundi_void_altar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Susur Secundi, Void Altar: "This land enters tapped." / "{T}: Add {B}." / "Station..." / "12+ | {1}{B}, {T}, Pay 2 life, Sacrifice a creature: Draw cards equal to the sacrificed creature's power."
/// Under `Coverage::Partial`, Station and the `12+` draw ability are omitted because no amount reads the power of a creature tapped or sacrificed in a cost.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for black mana.
#[test]
fn susur_secundi_void_altar_enters_tapped_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(220, forest())
        .hand(0, &[susur_secundi_void_altar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, susur_secundi_void_altar());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, susur_secundi_void_altar(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, land));
}
