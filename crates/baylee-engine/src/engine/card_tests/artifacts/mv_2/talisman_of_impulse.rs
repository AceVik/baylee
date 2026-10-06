//! `cards/artifacts/mv_2/talisman_of_impulse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Talisman of Impulse` prints `{{T}}: Add {{C}}.` and `{{T}}: Add {{R}} or {{G}}. This artifact deals 1 damage to you.`
/// with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Talisman of Impulse` with 20 starting life.
/// Activating ability 1 prompts for a choice between red and green mana via `Pending::ChooseColor`,
/// adds the chosen red mana, deals 1 damage to its controller, and taps the talisman without using the stack.
#[test]
fn talisman_of_impulse_adds_colored_mana_and_deals_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[talisman_of_impulse()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let talisman =
        on_battlefield(&engine, p0, talisman_of_impulse()).expect("talisman on battlefield");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, talisman_of_impulse(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![ManaColor::Red, ManaColor::Green]);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("chose red mana");

    assert!(stack_is_empty(&engine), "mana ability skips the stack");
    assert!(
        is_tapped(&engine, talisman),
        "`Talisman of Impulse` is tapped"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "controller took 1 damage"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "added one red mana");
    assert_eq!(pool.total(), 1, "exactly one mana in pool");
}
