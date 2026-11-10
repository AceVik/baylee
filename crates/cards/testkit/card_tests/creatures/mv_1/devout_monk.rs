//! `cards/creatures/mv_1/devout_monk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Devout Monk prints one line — "When this creature enters, you gain 1
/// life" — so the only thing worth playing is the gap between the cast and
/// the trigger resolving. The test reads the life total twice: still twenty
/// while the enters-trigger sits on the stack with the 1/1 already on the
/// battlefield, and twenty-one once it has resolved, which is what separates
/// a life gain that belongs to the entry from one that was paid on the way
/// in. The opponent's life is the control: "you gain" is the controller of
/// the entering creature and not the table.
#[test]
fn devout_monk_gains_its_controller_a_life_when_it_enters_and_no_one_else() {
    let p0 = PlayerId::new(0);
    let _p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[devout_monk()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, devout_monk());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, devout_monk()).is_some() && !stack_is_empty(e)
    });

    let monk = on_battlefield(&engine, p0, devout_monk()).expect("the Monk resolved");
    assert_eq!(pt(&engine, monk), (1, 1), "the body the card prints");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the life is the enters-trigger's, and the trigger is still on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"when this creature enters, you gain 1 life\""
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and \"you\" is the Monk's controller, not the table"
    );
}
