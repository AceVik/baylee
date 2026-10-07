//! `cards/lands/utility/hostile_hostel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hostile Hostel // Creeping Inn: "{T}: Add {C}." / "{1}, {T}, Sacrifice a creature: Put a soul counter on this land..."
/// Under `Coverage::Partial`, the soul-counter transform and linked-exile attack trigger are omitted.
/// Playing the front face enters untapped and immediately taps for colorless mana.
#[test]
fn hostile_hostel_enters_untapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(227, forest())
        .hand(0, &[hostile_hostel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, hostile_hostel());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, hostile_hostel(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
