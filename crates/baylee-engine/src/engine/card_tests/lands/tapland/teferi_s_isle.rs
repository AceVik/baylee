//! `cards/lands/tapland/teferi_s_isle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Teferi's Isle prints `Phasing`, `Teferi's Isle enters tapped.`, and
/// `{{T}}: Add {{U}}{{U}}.`
///
/// Under `Coverage::Partial`, phasing is omitted because there is no keyword bit
/// and no untap-step phasing trigger. Playing the land causes it to enter tapped.
/// Advancing past the opponent's turn untaps it on its controller's next turn,
/// where activating ability 0 adds two blue mana to the pool.
#[test]
fn teferis_isle_enters_tapped_and_taps_for_two_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[teferi_s_isle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, teferi_s_isle());
    assert!(entered_tapped(&engine, land));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, teferi_s_isle(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 2);
    assert_eq!(pool.total(), 2);
    assert!(is_tapped(&engine, land));
}
