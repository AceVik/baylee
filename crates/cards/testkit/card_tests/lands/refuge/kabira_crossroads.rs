//! `cards/lands/refuge/kabira_crossroads.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Kabira Crossroads` enters tapped, gains 2 life on entry, and taps for `{{W}}`
/// under `Coverage::Implemented`.
/// Playing the land puts its enters-the-battlefield trigger on the stack, which resolves
/// to raise its controller's life total from 20 to 22.
#[test]
fn kabira_crossroads_enters_tapped_and_gains_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1920, forest())
        .hand(0, &[kabira_crossroads()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, kabira_crossroads());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 22);
    assert!(is_tapped(&engine, land));
}
