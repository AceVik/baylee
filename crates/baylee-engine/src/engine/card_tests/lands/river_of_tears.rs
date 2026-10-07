//! `cards/lands/river_of_tears.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// River of Tears prints `{T}: Add {U}. If you played a land this turn, add {B} instead.`
/// The card is marked `Coverage::Partial` because conditional replacement of mana by a land played
/// this turn is not expressible in the `DSL`.
/// In this scenario, River of Tears itself is played as the land drop for the turn, and tapping it
/// adds `{U}` rather than `{B}`, producing one blue mana and zero black mana in the player's mana pool.
#[test]
fn river_of_tears_adds_blue_mana_even_when_a_land_was_played_this_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[river_of_tears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, river_of_tears());
    assert!(on_battlefield(&engine, p0, river_of_tears()).is_some());
    assert!(!is_tapped(&engine, land));

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 1);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.available(ManaColor::Black), 0);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, land));
}
