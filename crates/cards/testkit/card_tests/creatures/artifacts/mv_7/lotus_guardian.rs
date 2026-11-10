//! `cards/creatures/artifacts/mv_7/lotus_guardian.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Lotus Guardian` prints `Flying` and `{{T}}: Add one mana of any color.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Lotus Guardian` with an empty mana pool.
/// Activating its mana ability prompts for a color choice via `Pending::ChooseColor` with all five colors available,
/// taps the guardian, and adds the chosen mana without using the stack.
#[test]
fn lotus_guardian_taps_to_add_any_color_of_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[lotus_guardian()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let guardian = on_battlefield(&engine, p0, lotus_guardian()).expect("guardian seated");
    assert_eq!(pt(&engine, guardian), (4, 4));
    assert!(keywords(&engine, guardian).contains(KeywordSet::FLYING));
    assert!(!is_tapped(&engine, guardian));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    activate(&mut engine, p0, lotus_guardian(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(
        options,
        vec![
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green,
        ]
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("choosing blue mana");

    assert!(
        stack_is_empty(&engine),
        "mana ability does not use the stack"
    );
    assert!(
        is_tapped(&engine, guardian),
        "`Lotus Guardian` tapped to pay its cost"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "added one blue mana");
    assert_eq!(pool.total(), 1, "exactly one mana in pool");
}
