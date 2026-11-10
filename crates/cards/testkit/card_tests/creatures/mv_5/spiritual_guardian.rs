//! `cards/creatures/mv_5/spiritual_guardian.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Spiritual Guardian — {3}{W}{W} — Creature — Spirit, 3/4 — "When this
/// creature enters, you gain 4 life."
///
/// The card is a body and one enters-trigger, so a scenario that only looked
/// at the board would pass on a vanilla 3/4 and say nothing about the sentence
/// the card is played for. Five Plains are the exact `{3}{W}{W}`, so the pool
/// is emptied by the cast and nothing else on the table can move a life total;
/// the four life therefore arrives only from the trigger, and the opponent's
/// twenty is the control that says "you" means the controller and not the
/// table. The life is read *after* the stack has emptied, because a permanent
/// entering puts its triggers on the stack (CR 603.2) and a read taken while
/// the Guardian is still a spell would count the life before it was gained.
#[test]
fn spiritual_guardian_gains_four_life_when_it_enters_and_nothing_for_the_opponent() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[spiritual_guardian()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(
        engine.state().players[0].life,
        20,
        "nothing has been gained yet"
    );
    assert!(
        on_battlefield(&engine, p0, spiritual_guardian()).is_none(),
        "and the card is in hand, not on the table"
    );

    cast_from_hand(&mut engine, p0, spiritual_guardian());
    assert!(
        on_stack(&engine, spiritual_guardian()).is_some(),
        "{{3}}{{W}}{{W}} out of five Plains puts the creature on the stack \
         first, which is where its enters-trigger comes from"
    );
    pass_until(&mut engine, stack_is_empty);

    let guardian =
        on_battlefield(&engine, p0, spiritual_guardian()).expect("the Guardian resolved");
    assert_eq!(
        pt(&engine, guardian),
        (3, 4),
        "the body the card prints, and no state-based check eating it"
    );
    assert_eq!(
        engine.state().players[0].life,
        24,
        "\"When this creature enters, you gain 4 life\" — four, once"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the controller alone"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the five Plains were spent on the cast, so the life is not bought \
         with mana left floating"
    );
}
