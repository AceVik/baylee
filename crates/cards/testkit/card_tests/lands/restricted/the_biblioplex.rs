//! `cards/lands/restricted/the_biblioplex.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Biblioplex: "{T}: Add {C}." / "{2}, {T}: Look at the top card of your library..."
/// Under `Coverage::Partial`, the `{2}, {T}` top-card library ability is omitted.
/// Playing this land allows it to enter untapped and immediately tap for colorless mana.
#[test]
fn the_biblioplex_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(320, forest())
        .hand(0, &[the_biblioplex()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, the_biblioplex());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, the_biblioplex(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
