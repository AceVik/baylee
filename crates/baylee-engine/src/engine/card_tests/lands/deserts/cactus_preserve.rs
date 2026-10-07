//! `cards/lands/deserts/cactus_preserve.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cactus Preserve: "This land enters tapped." / "{T}: Add one mana of any type that a land you control could produce." / "{3}: Until end of turn, this land becomes an X/X green Plant creature..."
/// Under `Coverage::Partial`, the `{3}` animation is omitted because no amount reads commander mana values.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it produces red mana matching a controlled Mountain.
#[test]
fn cactus_preserve_enters_tapped_and_taps_for_controlled_land_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(306, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[cactus_preserve()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, cactus_preserve());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, cactus_preserve(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}
