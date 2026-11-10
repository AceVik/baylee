//! `cards/creatures/artifacts/mv_4/peace_strider.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn peace_strider() -> CardIndex {
    card_index("b0e96742-77c0-4de5-84b0-0d59a8bee3e0")
}

/// Peace Strider — {4} 3/3: "When this creature enters, you gain 3 life."
/// Cast off four Forests, its controller goes from 20 to 23 and the
/// opponent stays at 20.
#[test]
fn peace_strider_gains_three_life_on_entering() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 4])
        .hand(0, &[peace_strider()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(engine.state().players[0].life, 20);
    cast_from_hand(&mut engine, p0, peace_strider());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, peace_strider()).is_some());
    assert_eq!(engine.state().players[0].life, 23, "+3 for the trigger");
    assert_eq!(engine.state().players[1].life, 20);
}
