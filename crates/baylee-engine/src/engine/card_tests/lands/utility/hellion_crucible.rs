//! `cards/lands/utility/hellion_crucible.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hellion Crucible: "{T}: Add {C}." / "{1}{R}, {T}: Put a pressure counter on this land..." / "{1}{R}, {T}, Remove two pressure counters..."
/// Under `Coverage::Partial`, pressure counter abilities are omitted because pressure counters have no assigned id.
/// Playing this land allows it to enter untapped and immediately tap for colorless mana.
#[test]
fn hellion_crucible_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(335, forest())
        .hand(0, &[hellion_crucible()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, hellion_crucible());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, hellion_crucible(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
