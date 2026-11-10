//! `cards/artifacts/mv_3/bloodstone_cameo.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Bloodstone Cameo` prints `{{T}}: Add {{B}} or {{R}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Bloodstone Cameo` with an empty mana pool.
/// Activating its mana ability prompts for a choice between black and red mana via `Pending::ChooseColor`,
/// adds the chosen black mana, and taps the cameo without using the stack.
#[test]
fn bloodstone_cameo_taps_for_black_or_red_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[bloodstone_cameo()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cameo = on_battlefield(&engine, p0, bloodstone_cameo()).expect("cameo on battlefield");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, bloodstone_cameo(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![ManaColor::Black, ManaColor::Red]);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("chose black mana");

    assert!(stack_is_empty(&engine), "mana ability skips the stack");
    assert!(is_tapped(&engine, cameo), "`Bloodstone Cameo` is tapped");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1, "added one black mana");
    assert_eq!(pool.total(), 1, "exactly one mana in pool");
}
