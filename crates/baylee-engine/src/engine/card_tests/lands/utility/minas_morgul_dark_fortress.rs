//! `cards/lands/utility/minas_morgul_dark_fortress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Minas Morgul, Dark Fortress: "Minas Morgul enters tapped." / "{T}: Add {B}." / "{3}{B}, {T}: Put a shadow counter on target creature..."
/// Under `Coverage::Partial`, the shadow counter activation is omitted.
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for black mana.
#[test]
fn minas_morgul_dark_fortress_enters_tapped_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(337, forest())
        .hand(0, &[minas_morgul_dark_fortress()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, minas_morgul_dark_fortress());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, minas_morgul_dark_fortress(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, land));
}
