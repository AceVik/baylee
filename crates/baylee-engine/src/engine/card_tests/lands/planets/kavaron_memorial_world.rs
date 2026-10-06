//! `cards/lands/planets/kavaron_memorial_world.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kavaron, Memorial World: "This land enters tapped." / "{T}: Add {R}." / "Station..." / "12+ | {1}{R}, {T}, Sacrifice a land: Create a 2/2 colorless Robot artifact creature token..."
/// Under `Coverage::Partial`, Station is omitted because no amount reads the power of a creature tapped to pay a cost.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for red mana.
#[test]
fn kavaron_memorial_world_enters_tapped_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(219, forest())
        .hand(0, &[kavaron_memorial_world()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, kavaron_memorial_world());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, kavaron_memorial_world(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}
