//! `cards/creatures/mv_3/venerable_monk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Venerable Monk is a `{2}{W}` 2/2 whose whole rules text is "When this
/// creature enters, you gain 2 life." The scenario is built so that the two
/// halves cannot be confused: three Plains pay the spell, and the life total
/// is read *while the card is still on the stack* — twenty, because the gain
/// belongs to the enters trigger and not to the cast — and again once the
/// stack has emptied, where it is twenty-two. The body is asserted with it,
/// so the card is a permanent that arrived rather than a life total that
/// moved on its own, and the seat across the table is read to say that
/// "you" is the controller and not the table.
#[test]
fn venerable_monk_gains_two_life_as_it_enters_and_leaves_the_other_seat_alone() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[venerable_monk()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, venerable_monk());
    assert!(
        on_stack(&engine, venerable_monk()).is_some(),
        "three Plains pay {{2}}{{W}}, so the creature is a spell before it is a permanent"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the gain is the enters trigger's, so nothing is gained while the spell is still on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    let monk = on_battlefield(&engine, p0, venerable_monk()).expect("the Monk resolved");
    assert_eq!(pt(&engine, monk), (2, 2), "the printed 2/2 body arrived");
    assert_eq!(
        engine.state().players[0].life,
        22,
        "\"When this creature enters, you gain 2 life\" — two, and not a life per permanent or per turn"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"you\" is the controller, and the seat across the table gains nothing"
    );
}
