//! `cards/artifacts/mv_3/the_everflowing_well.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `The Everflowing Well` is a legendary artifact costing `{2}{U}` under `Coverage::Partial`.
/// It prints "When `The Everflowing Well` enters, mill two cards, then draw two cards."
/// When cast from hand off three `island()` sources, its enters-the-battlefield trigger
/// mills two cards into its controller's graveyard and draws two cards.
#[test]
fn the_everflowing_well_enters_and_mills_two_then_draws_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[the_everflowing_well()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, the_everflowing_well());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, the_everflowing_well()).is_some(),
        "`The Everflowing Well` is on the battlefield"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2,
        "two cards were milled into the graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1 + 2,
        "the artifact was cast from hand and two cards were drawn"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 4,
        "four cards left the library: two milled and two drawn"
    );
}
