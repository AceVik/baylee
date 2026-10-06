//! `cards/artifacts/mv_2/talisman_of_dominance.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Talisman of Dominance` prints `{{T}}: Add {{C}}.` and `{{T}}: Add {{U}} or {{B}}. This artifact deals 1 damage to you.`
/// with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Talisman of Dominance` with 20 starting life.
/// Activating ability 1 prompts for a choice between blue and black mana via `Pending::ChooseColor`,
/// adds the chosen blue mana, deals 1 damage to its controller, and taps the talisman without using the stack.
#[test]
fn talisman_of_dominance_adds_colored_mana_and_deals_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[talisman_of_dominance()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let talisman =
        on_battlefield(&engine, p0, talisman_of_dominance()).expect("talisman on battlefield");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, talisman_of_dominance(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![ManaColor::Blue, ManaColor::Black]);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("chose blue mana");

    assert!(stack_is_empty(&engine), "mana ability skips the stack");
    assert!(
        is_tapped(&engine, talisman),
        "`Talisman of Dominance` is tapped"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "controller took 1 damage"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "added one blue mana");
    assert_eq!(pool.total(), 1, "exactly one mana in pool");
}
