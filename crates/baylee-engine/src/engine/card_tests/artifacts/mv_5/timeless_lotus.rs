//! `cards/artifacts/mv_5/timeless_lotus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn timeless_lotus() -> CardIndex {
    card_index("fec77e0a-433e-4d32-9260-e26a37f6ad23")
}

/// "Timeless Lotus enters tapped."
#[test]
fn the_lotus_enters_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 5])
        .hand(0, &[timeless_lotus()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, timeless_lotus());
    pass_until(&mut engine, stack_is_empty);
    let lotus = on_battlefield(&engine, p0, timeless_lotus()).unwrap();
    assert!(is_tapped(&engine, lotus));
}

/// "{T}: Add {W}{U}{B}{R}{G}." One of each, five in all.
#[test]
fn tapping_adds_one_mana_of_each_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[timeless_lotus()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let lotus = on_battlefield(&engine, p0, timeless_lotus()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == lotus);
    assert!(is_tapped(&engine, lotus));
    let pool = &engine.state().players[0].mana_pool;
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert_eq!(pool.available(color), 1, "{color:?}");
    }
    assert_eq!(pool.total(), 5, "and nothing else");
}
