//! `cards/lands/check/archway_of_innovation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Archway of Innovation: "This land enters tapped unless you control an Island." / "{T}: Add {U}." / "{U}, {T}: The next spell you cast this turn has improvise."
/// Under `Coverage::Partial`, the improvise grant is omitted because no modifier grants improvise.
/// Controlling an Island allows Archway of Innovation to enter untapped and immediately tap for blue mana.
#[test]
fn archway_of_innovation_enters_untapped_with_island_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(302, forest())
        .battlefield(0, &[island()])
        .hand(0, &[archway_of_innovation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, archway_of_innovation());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, archway_of_innovation(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}
