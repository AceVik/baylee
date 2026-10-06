//! `cards/artifacts/mv_2/moss_diamond.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Moss Diamond` prints `This artifact enters tapped.` and `{{T}}: Add {{G}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls two copies of `forest()` and casts `Moss Diamond` from hand.
/// Upon entering the battlefield, the artifact is tapped due to `EnterModifier::Tapped`.
/// Advancing to seat 0's next turn untaps the diamond, allowing its mana ability to activate and add one green mana.
#[test]
fn moss_diamond_enters_tapped_and_taps_for_green() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[moss_diamond()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, moss_diamond());
    pass_until(&mut engine, stack_is_empty);

    let diamond = on_battlefield(&engine, p0, moss_diamond()).expect("diamond is on battlefield");
    assert!(is_tapped(&engine, diamond), "`Moss Diamond` enters tapped");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, diamond),
        "diamond untapped on next turn"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, moss_diamond(), 0);
    assert!(stack_is_empty(&engine), "mana ability skips the stack");
    assert!(is_tapped(&engine, diamond), "diamond tapped to add mana");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1, "added one green mana");
    assert_eq!(pool.total(), 1, "exactly one mana in pool");
}
