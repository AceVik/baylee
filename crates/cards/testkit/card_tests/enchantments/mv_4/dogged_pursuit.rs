//! `cards/enchantments/mv_4/dogged_pursuit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn dogged_pursuit() -> CardIndex {
    card_index("987f8c13-0164-4536-b505-d9fa799222fb")
}

/// "At the beginning of your end step, each opponent loses 1 life and you
/// gain 1 life": at a table of three, both opponents lose 1, only on the
/// controller's end step, not on an opponent's.
#[test]
fn dogged_pursuit_drains_each_opponent_on_my_end_step_only() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[dogged_pursuit()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    walk_to_step_trigger(&mut engine, p0, Step::End);
    assert_eq!(
        [
            life_of(&engine, p0),
            life_of(&engine, p1),
            life_of(&engine, p2)
        ],
        [20, 20, 20],
        "the trigger waits on the stack"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), 21, "I gain 1");
    assert_eq!(life_of(&engine, p1), 19, "the first opponent loses 1");
    assert_eq!(life_of(&engine, p2), 19, "so does the second");

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::End
    });
    assert!(
        stack_is_empty(&engine),
        "an opponent's end step is not mine"
    );
    assert_eq!(life_of(&engine, p0), 21);
}
