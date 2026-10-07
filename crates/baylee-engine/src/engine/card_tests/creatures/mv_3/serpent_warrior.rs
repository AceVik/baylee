//! `cards/creatures/mv_3/serpent_warrior.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Serpent Warrior — {2}{B} — 3/3 Snake Warrior: "When this creature enters,
/// you lose 3 life."
///
/// Two claims meet in one cast and each is the other's witness: the 3/3 body
/// is what proves a permanent really entered (a life loss read off a card
/// still on the stack would be a different card), and the life total is what
/// proves the loss is the *controller's* and not the table's. The life is
/// read both before and after the trigger resolves, because the price is a
/// triggered ability (CR 603.6a) and not an additional cost: the Warrior is
/// already standing at 20 while the question is still on the stack, and only
/// the resolution behind it takes the three.
#[test]
fn serpent_warrior_makes_its_controller_lose_three_life_as_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(23, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[serpent_warrior()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, serpent_warrior());
    // The spell resolves and its entry trigger goes on the stack behind it,
    // so this stops with the Warrior on the battlefield and the trigger
    // unanswered.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, serpent_warrior()).is_some() && !stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the loss is a triggered ability (CR 603.6a): the Warrior has already \
         entered and nothing has been paid for it yet"
    );

    pass_until(&mut engine, stack_is_empty);
    let warrior = on_battlefield(&engine, p0, serpent_warrior()).expect("the Warrior resolved");
    assert_eq!(pt(&engine, warrior), (3, 3), "the body the card prints");
    assert_eq!(
        engine.state().players[0].life,
        17,
        "\"you lose 3 life\" — three off the seat that cast it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"you\" is the controller: the loss never crosses the table"
    );
}
