//! `cards/lands/planets/adagia_windswept_bastion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Adagia, Windswept Bastion: "This land enters tapped." / "{T}: Add {W}." / "Station..." / "12+ | {3}{W}, {T}: Create a token that's a copy of target artifact or enchantment you control, except it's legendary."
/// Under `Coverage::Partial`, Station and the `12+` copy ability are omitted because power-scaled counters and legendary copy modifications are not supported.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for white mana.
#[test]
fn adagia_windswept_bastion_enters_tapped_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(218, forest())
        .hand(0, &[adagia_windswept_bastion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, adagia_windswept_bastion());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, adagia_windswept_bastion(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}
