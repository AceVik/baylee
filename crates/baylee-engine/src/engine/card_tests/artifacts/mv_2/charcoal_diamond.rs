//! `cards/artifacts/mv_2/charcoal_diamond.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Charcoal Diamond` prints `This artifact enters tapped.` and `{{T}}: Add {{B}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls two copies of `forest()` and casts `Charcoal Diamond` from hand.
/// Upon entering the battlefield, the artifact is tapped due to `EnterModifier::Tapped`.
/// Advancing to seat 0's next turn untaps the diamond, allowing its mana ability to activate and add one black mana.
#[test]
fn charcoal_diamond_enters_tapped_and_taps_for_black() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[charcoal_diamond()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, charcoal_diamond());
    pass_until(&mut engine, stack_is_empty);

    let diamond =
        on_battlefield(&engine, p0, charcoal_diamond()).expect("diamond is on battlefield");
    assert!(
        is_tapped(&engine, diamond),
        "`Charcoal Diamond` enters tapped"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, diamond),
        "diamond untapped on next turn"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, charcoal_diamond(), 0);
    assert!(stack_is_empty(&engine), "mana ability skips the stack");
    assert!(is_tapped(&engine, diamond), "diamond tapped to add mana");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1, "added one black mana");
    assert_eq!(pool.total(), 1, "exactly one mana in pool");
}
