//! `cards/lands/towns/clive_s_hideaway.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Clive's Hideaway: "Hideaway 4..." / "{T}: Add {C}." / "{2}, {T}: You may play the exiled card..."
/// Under `Coverage::Partial`, Hideaway 4 and playing the exiled card are omitted.
/// Playing this land allows it to enter untapped and immediately tap for colorless mana.
#[test]
fn clive_s_hideaway_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(329, forest())
        .hand(0, &[clive_s_hideaway()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, clive_s_hideaway());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, clive_s_hideaway(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
