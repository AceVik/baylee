//! `cards/artifacts/mv_2/implements_of_sacrifice.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Implements of Sacrifice` prints `{{1}}, {{T}}, Sacrifice this artifact: Add two mana of any one color.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Implements of Sacrifice` and a `forest()`.
/// Floating one generic mana pays to activate the mana ability, prompting `Pending::ChooseColor`
/// with all five mana colors, sacrificing the artifact and adding two black mana without using the stack.
#[test]
fn implements_of_sacrifice_adds_two_mana_of_chosen_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), implements_of_sacrifice()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana_but(&mut engine, p0, Some(implements_of_sacrifice()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    activate(&mut engine, p0, implements_of_sacrifice(), 0);
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
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("chose black mana");

    assert!(stack_is_empty(&engine), "mana ability skips the stack");
    assert!(
        in_graveyard(&engine, p0, implements_of_sacrifice()).is_some(),
        "`Implements of Sacrifice` was sacrificed"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 2, "added two black mana");
    assert_eq!(pool.available(ManaColor::Green), 0, "green mana was spent");
    assert_eq!(pool.total(), 2, "exactly two mana in pool");
}
