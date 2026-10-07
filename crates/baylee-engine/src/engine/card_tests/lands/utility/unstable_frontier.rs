//! `cards/lands/utility/unstable_frontier.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Unstable Frontier: "{T}: Add {C}." / "{T}: Target land you control becomes the basic land type of your choice until end of turn."
/// Under `Coverage::Partial`, the basic land type grant ability is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn unstable_frontier_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(136, forest())
        .battlefield(0, &[unstable_frontier()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let frontier = on_battlefield(&engine, p0, unstable_frontier()).expect("Frontier deployed");
    activate(&mut engine, p0, unstable_frontier(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, frontier));
}
