//! `cards/lands/deserts/sunscorched_desert.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sunscorched Desert: "When this land enters, it deals 1 damage to target player or planeswalker." / "{T}: Add {C}."
/// Under `Coverage::Partial`, the enters-the-battlefield trigger is omitted due to target specification constraints.
/// The land enters without triggering damage and taps to add {C} to the mana pool.
#[test]
fn sunscorched_desert_enters_without_trigger_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(105, forest())
        .hand(0, &[sunscorched_desert()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p1_life_before = engine.state().players[1].life;
    let desert = play_land(&mut engine, p0, sunscorched_desert());

    assert!(stack_is_empty(&engine), "no enters trigger on the stack");
    assert_eq!(
        engine.state().players[1].life,
        p1_life_before,
        "opponent life unchanged"
    );

    activate(&mut engine, p0, sunscorched_desert(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, desert));
}
